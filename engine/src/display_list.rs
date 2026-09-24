use serde::Serialize;

pub const MOVE: f32 = 0.0;
pub const LINE: f32 = 1.0;
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
        color: [f32; 4],
        path: Vec<f32>,
    },
    GlyphRun {
        font: u32,
        size: f32,
        color: [f32; 4],
        glyphs: Vec<u16>,
        positions: Vec<f32>,
    },
    Image {
        image: u32,
        rect: [f32; 4],
    },
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

pub fn encode(ops: &[Op]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut open = Vec::new();
    let floats = |out: &mut Vec<u32>, v: &[f32]| out.extend(v.iter().map(|f| f.to_bits()));
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
                let start = open.pop().expect("EndItem without BeginItem");
                out[start - 1] = out[start..]
                    .iter()
                    .fold(0x811c9dc5, |h, w| (h ^ w).wrapping_mul(0x01000193));
                out.push(2);
            }
            Op::FillPath { color, path } => {
                out.push(3);
                floats(&mut out, color);
                out.push(path.len() as u32);
                floats(&mut out, path);
            }
            Op::GlyphRun {
                font,
                size,
                color,
                glyphs,
                positions,
            } => {
                out.extend([4, *font, size.to_bits()]);
                floats(&mut out, color);
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
                    color,
                    path: rect(0.0, 0.0, 1.0, 1.0),
                },
                Op::EndItem,
            ])[2]
        };
        assert_eq!(hash([1.0; 4]), hash([1.0; 4]));
        assert_ne!(hash([1.0; 4]), hash([0.5; 4]));
    }

    #[test]
    fn writes_fixture_for_the_ts_decoder() {
        let ops = vec![
            Op::Page {
                width: 420.5,
                height: 595.25,
                bleed: 8.5,
            },
            Op::BeginItem { item: 7 },
            Op::FillPath {
                color: [1.0, 0.5, 0.25, 1.0],
                path: rect(10.0, 20.0, 30.5, 40.0),
            },
            Op::GlyphRun {
                font: 0,
                size: 12.0,
                color: [0.0, 0.0, 0.0, 1.0],
                glyphs: vec![3, 65535, 42],
                positions: vec![0.0, 0.0, 6.5, 0.0, 13.0, 0.0],
            },
            Op::Image {
                image: 2,
                rect: [1.0, 2.0, 3.0, 4.0],
            },
            Op::EndItem,
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
