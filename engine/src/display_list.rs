use crate::color::Ink;
use crate::geom::map;
use crate::style::Blend;
use serde::{Serialize, Serializer};

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
    /// `text` is what the glyphs stand for, glyph `i` for `text[ranges[i]]`; only
    /// the PDF uses it.
    GlyphRun {
        font: u32,
        size: f32,
        paint: Paint,
        glyphs: Vec<u16>,
        positions: Vec<f32>,
        #[serde(skip)]
        text: String,
        #[serde(skip)]
        ranges: Vec<std::ops::Range<usize>>,
    },
    /// The image `image` of `image::id`, whose unit square `transform` maps into
    /// page space.
    Image {
        image: u32,
        transform: [f32; 6],
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
        #[serde(serialize_with = "number")]
        blend: Blend,
        blur: f32,
        shadows: Vec<Shadow>,
    },
    PopLayer,
    /// The ops up to `PopTransform` are drawn through `transform` [a b c d e f].
    PushTransform {
        transform: [f32; 6],
    },
    PopTransform,
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
    /// `color` is the screen colour, `ink` how it prints.
    Solid {
        color: [f32; 4],
        #[serde(skip)]
        ink: Ink,
        #[serde(skip)]
        overprint: bool,
    },
    Linear {
        transform: [f32; 6],
        stops: Vec<Stop>,
        #[serde(skip)]
        overprint: bool,
    },
    Radial {
        transform: [f32; 6],
        stops: Vec<Stop>,
        #[serde(skip)]
        overprint: bool,
    },
}

impl Paint {
    /// Whether the paint leaves the inks it does not use under it as they are;
    /// only opaque paints do.
    pub fn overprints(&self) -> bool {
        match self {
            Paint::Solid {
                color, overprint, ..
            } => *overprint && color[3] >= 1.0,
            Paint::Linear {
                stops, overprint, ..
            }
            | Paint::Radial {
                stops, overprint, ..
            } => *overprint && stops.iter().all(|s| s.color[3] >= 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stop {
    pub at: f32,
    pub color: [f32; 4],
    #[serde(skip)]
    pub ink: Ink,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: [f32; 4],
    #[serde(skip)]
    pub ink: Ink,
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

/// `ops` moved by (dx, dy).
pub fn shift(mut ops: Vec<Op>, dx: f32, dy: f32) -> Vec<Op> {
    if dx == 0.0 && dy == 0.0 {
        return ops;
    }
    let path = |p: &mut Vec<f32>| *p = map(p, |[x, y]| [x + dx, y + dy]);
    let paint = |p: &mut Paint| {
        if let Paint::Linear { transform, .. } | Paint::Radial { transform, .. } = p {
            transform[4] += dx;
            transform[5] += dy;
        }
    };
    for op in &mut ops {
        match op {
            Op::FillPath { paint: pt, path: p }
            | Op::StrokePath {
                paint: pt, path: p, ..
            } => {
                paint(pt);
                path(p);
            }
            Op::PushClip { path: p, .. } => path(p),
            Op::GlyphRun {
                paint: pt,
                positions,
                ..
            } => {
                paint(pt);
                for xy in positions.chunks_mut(2) {
                    xy[0] += dx;
                    xy[1] += dy;
                }
            }
            Op::Image { transform, .. } => {
                transform[4] += dx;
                transform[5] += dy;
            }
            Op::PushTransform {
                transform: [a, b, c, d, e, f],
            } => {
                *e += dx - (*a * dx + *c * dy);
                *f += dy - (*b * dx + *d * dy);
            }
            _ => {}
        }
    }
    ops
}

fn floats(out: &mut Vec<u32>, v: &[f32]) {
    out.extend(v.iter().map(|f| f.to_bits()));
}

fn paint(out: &mut Vec<u32>, p: &Paint) {
    match p {
        Paint::Solid { color, .. } => {
            out.push(0);
            floats(out, color);
        }
        Paint::Linear {
            transform, stops, ..
        }
        | Paint::Radial {
            transform, stops, ..
        } => {
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
            Op::PopClip
            | Op::PopLayer
            | Op::PopMask
            | Op::PopTransform
            | Op::EndItem
            | Op::EndMask => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
                if *op == Op::EndMask {
                    depth += 1;
                }
            }
            Op::PushClip { .. }
            | Op::PushLayer { .. }
            | Op::PushTransform { .. }
            | Op::BeginMask
            | Op::BeginItem { .. } => depth += 1,
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
                ..
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
            Op::Image { image, transform } => {
                out.extend([5, *image]);
                floats(&mut out, transform);
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
                out.extend([opacity.to_bits(), *blend as u32, blur.to_bits()]);
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
            Op::PushTransform { transform } => {
                out.push(14);
                floats(&mut out, transform);
            }
            Op::PopTransform => out.push(15),
        }
    }
    out
}

/// `ops` with every colour replaced by `f(colour, ink, overprints)`.
pub fn recolor(ops: &[Op], f: &impl Fn(&[f32; 4], &Ink, bool) -> [f32; 4]) -> Vec<Op> {
    ops.iter()
        .map(|op| {
            let mut op = op.clone();
            match &mut op {
                Op::FillPath { paint, .. }
                | Op::StrokePath { paint, .. }
                | Op::GlyphRun { paint, .. } => {
                    let over = paint.overprints();
                    match paint {
                        Paint::Solid { color, ink, .. } => *color = f(color, ink, over),
                        Paint::Linear { stops, .. } | Paint::Radial { stops, .. } => {
                            for s in stops {
                                s.color = f(&s.color, &s.ink, over);
                            }
                        }
                    }
                }
                Op::PushLayer { shadows, .. } => {
                    for s in shadows {
                        s.color = f(&s.color, &s.ink, false);
                    }
                }
                _ => {}
            }
            op
        })
        .collect()
}

fn number<S: Serializer>(blend: &Blend, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u32(*blend as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_moves_curves_glyphs_gradients_and_images() {
        let linear = Paint::Linear {
            transform: [1.0, 0.0, 0.0, 1.0, 2.0, 3.0],
            stops: Vec::new(),
            overprint: false,
        };
        let ops = shift(
            vec![
                Op::StrokePath {
                    paint: linear.clone(),
                    width: 1.0,
                    cap: 0,
                    join: 0,
                    path: vec![MOVE, 0.0, 0.0, CUBIC, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, CLOSE],
                },
                Op::GlyphRun {
                    font: 0,
                    size: 1.0,
                    paint: linear,
                    glyphs: vec![1, 2],
                    positions: vec![0.0, 0.0, 5.0, 0.0],
                    text: String::new(),
                    ranges: Vec::new(),
                },
                Op::Image {
                    image: 0,
                    transform: [4.0, 0.0, 0.0, 4.0, 1.0, 1.0],
                },
            ],
            10.0,
            1.0,
        );
        let Op::StrokePath { path, paint, .. } = &ops[0] else {
            panic!()
        };
        assert_eq!(
            path,
            &[
                MOVE, 10.0, 1.0, CUBIC, 11.0, 2.0, 12.0, 3.0, 13.0, 4.0, CLOSE
            ]
        );
        let Paint::Linear { transform, .. } = paint else {
            panic!()
        };
        assert_eq!(transform[4..], [12.0, 4.0]);
        let Op::GlyphRun { positions, .. } = &ops[1] else {
            panic!()
        };
        assert_eq!(positions, &[10.0, 1.0, 15.0, 1.0]);
        assert!(matches!(
            ops[2],
            Op::Image {
                transform: [4.0, 0.0, 0.0, 4.0, 11.0, 2.0],
                ..
            }
        ));
    }

    #[test]
    fn item_hash_follows_item_content() {
        let hash = |color| {
            encode(&[
                Op::BeginItem { item: 0 },
                Op::FillPath {
                    paint: Paint::Solid {
                        color,
                        ink: Ink::Rgb,
                        overprint: false,
                    },
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
                    blend: Blend::Normal,
                    blur: 0.0,
                    shadows: vec![],
                },
                Op::PushLayer {
                    opacity,
                    blend: Blend::Normal,
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
    fn the_fixture_of_the_ts_decoder_is_current() {
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
                blend: Blend::Multiply,
                blur: 2.0,
                shadows: vec![Shadow {
                    offset: [1.0, 2.0],
                    blur: 3.0,
                    color: [0.0, 0.0, 0.0, 0.25],
                    ink: Ink::Rgb,
                }],
            },
            Op::BeginMask,
            Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                    ink: Ink::Rgb,
                    overprint: false,
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
                            ink: Ink::Rgb,
                        },
                        Stop {
                            at: 1.0,
                            color: [0.0, 0.5, 1.0, 0.5],
                            ink: Ink::Rgb,
                        },
                    ],
                    overprint: false,
                },
                path: rect(10.0, 20.0, 30.5, 40.0),
            },
            Op::StrokePath {
                paint: Paint::Radial {
                    transform: [2.0, 0.0, 0.0, 3.0, 1.0, 1.0],
                    stops: vec![Stop {
                        at: 0.5,
                        color: [1.0, 1.0, 1.0, 1.0],
                        ink: Ink::Rgb,
                    }],
                    overprint: false,
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
                    ink: Ink::Rgb,
                    overprint: false,
                },
                glyphs: vec![3, 65535, 42],
                positions: vec![0.0, 0.0, 6.5, 0.0, 13.0, 0.0],
                text: String::new(),
                ranges: vec![],
            },
            Op::Image {
                image: 2,
                transform: [3.0, 0.0, 0.0, 4.0, 1.0, 2.0],
            },
            Op::EndItem,
            Op::PopMask,
            Op::PopLayer,
            Op::PopClip,
        ];
        let words = encode(&ops);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let json = serde_json::to_string_pretty(&ops).unwrap();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../web/src/testdata");
        for (name, data) in [
            ("display-list.bin", bytes),
            ("display-list.json", json.into()),
        ] {
            let path = format!("{dir}/{name}");
            if std::env::var_os("SATZ_WRITE_FIXTURES").is_some() {
                std::fs::write(&path, &data).unwrap();
            }
            assert!(
                std::fs::read(&path).is_ok_and(|f| f == data),
                "{path} is stale; SATZ_WRITE_FIXTURES=1 cargo test fixture writes it"
            );
        }
    }
}
