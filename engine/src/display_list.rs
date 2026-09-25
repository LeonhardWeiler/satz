use serde::Serialize;

pub const MOVE: f32 = 0.0;
pub const LINE: f32 = 1.0;
pub const CUBIC: f32 = 4.0;
pub const CLOSE: f32 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Op {
    Page {
        width: f32,
        height: f32,
        bleed: f32,
    },
    BeginItem {
        item: u32,
    },
    EndItem,
    FillPath {
        paint: Paint,
        path: Vec<f32>,
    },
    GlyphRun {
        font: u32,
        size: f32,
        paint: Paint,
        glyphs: Vec<u16>,
        positions: Vec<f32>,
    },
    Image {
        image: u32,
        rect: [f32; 4],
    },
    PushClip {
        path: Vec<f32>,
        invert: bool,
    },
    PopClip,
    StrokePath {
        paint: Paint,
        width: f32,
        cap: u32,
        join: u32,
        path: Vec<f32>,
    },
    /// Blur and shadow blurs are Gaussian sigmas in pt.
    PushLayer {
        opacity: f32,
        blend: u32,
        blur: f32,
        shadows: Vec<Shadow>,
    },
    PopLayer,
    /// The ops up to `EndMask` are an alpha mask for the ops up to `PopMask`.
    BeginMask,
    EndMask,
    PopMask,
}

/// Gradients map their unit space into page space with `transform` [a b c d e f]:
/// linear runs from (0, 0) to (1, 0), radial is the unit circle around (0, 0).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Paint {
    Solid {
        color: [f32; 4],
    },
    Linear {
        transform: [f32; 6],
        stops: Vec<Stop>,
    },
    Radial {
        transform: [f32; 6],
        stops: Vec<Stop>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stop {
    pub at: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: [f32; 4],
}

pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<f32> {
    vec![
        MOVE,
        x,
        y,
        LINE,
        x + w,
        y,
        LINE,
        x + w,
        y + h,
        LINE,
        x,
        y + h,
        CLOSE,
    ]
}

fn floats(out: &mut Vec<u32>, v: &[f32]) {
    out.extend(v.iter().map(|f| f.to_bits()));
}

fn paint(out: &mut Vec<u32>, p: &Paint) {
    match p {
        Paint::Solid { color } => {
            out.push(0);
            floats(out, color);
        }
        Paint::Linear { transform, stops } | Paint::Radial { transform, stops } => {
            out.push(if matches!(p, Paint::Linear { .. }) {
                1
            } else {
                2
            });
            floats(out, transform);
            out.push(stops.len() as u32);
            for s in stops {
                floats(out, &[s.at]);
                floats(out, &s.color);
            }
        }
    }
}

/// Index of the op that closes the one opened at `at`.
/// `EndMask` closes `BeginMask` and opens up to `PopMask`.
pub fn close(ops: &[Op], at: usize) -> usize {
    let mut depth = 1;
    for (i, op) in ops.iter().enumerate().skip(at + 1) {
        match op {
            Op::PopClip | Op::PopLayer | Op::PopMask | Op::EndItem | Op::EndMask => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
                if *op == Op::EndMask {
                    depth += 1;
                }
            }
            Op::PushClip { .. } | Op::PushLayer { .. } | Op::BeginMask | Op::BeginItem { .. } => {
                depth += 1
            }
            _ => {}
        }
    }
    ops.len()
}

/// Writes the hash of the words since the innermost open item or layer into the word before them.
fn seal(out: &mut [u32], open: &mut Vec<usize>) {
    let start = open.pop().expect("close without open");
    out[start - 1] = out[start..]
        .iter()
        .fold(0x811c9dc5, |h, w| (h ^ w).wrapping_mul(0x01000193));
}

pub fn encode(ops: &[Op]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut open = Vec::new();
    for op in ops {
        match op {
            Op::Page {
                width,
                height,
                bleed,
            } => {
                out.push(0);
                floats(&mut out, &[*width, *height, *bleed]);
            }
            Op::BeginItem { item } => {
                out.extend([1, *item, 0]);
                open.push(out.len());
            }
            Op::EndItem => {
                seal(&mut out, &mut open);
                out.push(2);
            }
            Op::FillPath { paint: p, path } => {
                out.push(3);
                paint(&mut out, p);
                out.push(path.len() as u32);
                floats(&mut out, path);
            }
            Op::GlyphRun {
                font,
                size,
                paint: p,
                glyphs,
                positions,
            } => {
                out.extend([4, *font, size.to_bits()]);
                paint(&mut out, p);
                out.push(glyphs.len() as u32);
                out.extend(
                    glyphs
                        .chunks(2)
                        .map(|c| c[0] as u32 | (*c.get(1).unwrap_or(&0) as u32) << 16),
                );
                floats(&mut out, positions);
            }
            Op::Image { image, rect } => {
                out.extend([5, *image]);
                floats(&mut out, rect);
            }
            Op::PushClip { path, invert } => {
                out.extend([6, *invert as u32, path.len() as u32]);
                floats(&mut out, path);
            }
            Op::PopClip => out.push(7),
            Op::StrokePath {
                paint: p,
                width,
                cap,
                join,
                path,
            } => {
                out.push(8);
                paint(&mut out, p);
                out.extend([width.to_bits(), *cap, *join, path.len() as u32]);
                floats(&mut out, path);
            }
            Op::PushLayer {
                opacity,
                blend,
                blur,
                shadows,
            } => {
                out.extend([9, 0]);
                open.push(out.len());
                out.extend([opacity.to_bits(), *blend, blur.to_bits()]);
                out.push(shadows.len() as u32);
                for s in shadows {
                    floats(&mut out, &s.offset);
                    floats(&mut out, &[s.blur]);
                    floats(&mut out, &s.color);
                }
            }
            Op::PopLayer => {
                seal(&mut out, &mut open);
                out.push(10);
            }
            Op::BeginMask => out.push(11),
            Op::EndMask => out.push(12),
            Op::PopMask => out.push(13),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_hash_follows_item_content() {
        let hash = |color| {
            encode(&[
                Op::BeginItem { item: 0 },
                Op::FillPath {
                    paint: Paint::Solid { color },
                    path: rect(0.0, 0.0, 1.0, 1.0),
                },
                Op::EndItem,
            ])[2]
        };
        assert_eq!(hash([1.0; 4]), hash([1.0; 4]));
        assert_ne!(hash([1.0; 4]), hash([0.5; 4]));
    }

    #[test]
    fn layer_hash_follows_everything_inside_the_layer() {
        let hash = |opacity| {
            encode(&[
                Op::PushLayer {
                    opacity: 1.0,
                    blend: 0,
                    blur: 0.0,
                    shadows: vec![],
                },
                Op::PushLayer {
                    opacity,
                    blend: 0,
                    blur: 0.0,
                    shadows: vec![],
                },
                Op::PopLayer,
                Op::PopLayer,
            ])[1]
        };
        assert_eq!(hash(0.5), hash(0.5));
        assert_ne!(hash(0.5), hash(0.25));
    }

    #[test]
    fn close_finds_the_matching_end() {
        let ops = [
            Op::BeginMask,
            Op::BeginItem { item: 0 },
            Op::EndItem,
            Op::EndMask,
            Op::BeginItem { item: 0 },
            Op::EndItem,
            Op::PopMask,
        ];
        assert_eq!([0, 3, 1].map(|at| close(&ops, at)), [3, 6, 2]);
    }

    #[test]
    fn writes_fixture_for_the_ts_decoder() {
        let ops = vec![
            Op::Page {
                width: 420.5,
                height: 595.25,
                bleed: 8.5,
            },
            Op::PushClip {
                path: rect(1.0, 2.0, 3.0, 4.0),
                invert: true,
            },
            Op::PushLayer {
                opacity: 0.5,
                blend: 1,
                blur: 2.0,
                shadows: vec![Shadow {
                    offset: [1.0, 2.0],
                    blur: 3.0,
                    color: [0.0, 0.0, 0.0, 0.25],
                }],
            },
            Op::BeginMask,
            Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                path: rect(0.0, 0.0, 1.0, 1.0),
            },
            Op::EndMask,
            Op::BeginItem { item: 7 },
            Op::FillPath {
                paint: Paint::Linear {
                    transform: [1.0, 0.0, 0.0, 1.0, 5.0, 6.0],
                    stops: vec![
                        Stop {
                            at: 0.0,
                            color: [1.0, 0.5, 0.25, 1.0],
                        },
                        Stop {
                            at: 1.0,
                            color: [0.0, 0.5, 1.0, 0.5],
                        },
                    ],
                },
                path: rect(10.0, 20.0, 30.5, 40.0),
            },
            Op::StrokePath {
                paint: Paint::Radial {
                    transform: [2.0, 0.0, 0.0, 3.0, 1.0, 1.0],
                    stops: vec![Stop {
                        at: 0.5,
                        color: [1.0, 1.0, 1.0, 1.0],
                    }],
                },
                width: 1.5,
                cap: 1,
                join: 2,
                path: rect(1.0, 1.0, 2.0, 2.0),
            },
            Op::GlyphRun {
                font: 0,
                size: 12.0,
                paint: Paint::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
                glyphs: vec![3, 65535, 42],
                positions: vec![0.0, 0.0, 6.5, 0.0, 13.0, 0.0],
            },
            Op::Image {
                image: 2,
                rect: [1.0, 2.0, 3.0, 4.0],
            },
            Op::EndItem,
            Op::PopMask,
            Op::PopLayer,
            Op::PopClip,
        ];
        let words = encode(&ops);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../web/src/testdata");
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(format!("{dir}/display-list.bin"), bytes).unwrap();
        std::fs::write(
            format!("{dir}/display-list.json"),
            serde_json::to_string_pretty(&ops).unwrap(),
        )
        .unwrap();
    }
}
