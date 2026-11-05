//! Figma files: the kiwi message in `canvas.fig` of the zip, read into JSON.

use crate::color::Color;
use crate::display_list::{CLOSE, CUBIC, LINE, MOVE};
use crate::geom::bounds;
use crate::import::{Item, Shape, Sheet, gradient, quad, rgba, then};
use crate::style::{Blend, Cap, Fill, FillKind, FillStop, Join};
use serde_json::{Map, Value};
use std::collections::HashMap;

type Res<T> = Result<T, String>;
type Files = Vec<(String, Vec<u8>)>;

const BAD: &str = "not a Figma file";

/// The files in the zip `z` by name, each inflated.
fn unzip(z: &[u8]) -> Res<Files> {
    let u16_at = |i: usize| {
        z.get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
            .ok_or(BAD)
    };
    let u32_at = |i: usize| {
        z.get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .ok_or(BAD)
    };
    let end = (0..z.len().saturating_sub(21))
        .rev()
        .find(|&i| z[i..].starts_with(b"PK\x05\x06"))
        .ok_or(BAD)?;
    let mut at = u32_at(end + 16)?;
    let mut out = Vec::new();
    for _ in 0..u16_at(end + 10)? {
        let (method, size, name_len) = (u16_at(at + 10)?, u32_at(at + 20)?, u16_at(at + 28)?);
        let name =
            String::from_utf8_lossy(z.get(at + 46..at + 46 + name_len).ok_or(BAD)?).into_owned();
        let local = u32_at(at + 42)?;
        let start = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
        let data = z.get(start..start + size).ok_or(BAD)?;
        out.push((
            name,
            match method {
                0 => data.to_vec(),
                8 => miniz_oxide::inflate::decompress_to_vec(data).map_err(|_| BAD)?,
                _ => return Err(BAD.into()),
            },
        ));
        at += 46 + name_len + u16_at(at + 30)? + u16_at(at + 32)?;
    }
    Ok(out)
}

struct Kiwi<'a> {
    b: &'a [u8],
    at: usize,
}

impl Kiwi<'_> {
    fn byte(&mut self) -> Res<u8> {
        let b = *self.b.get(self.at).ok_or(BAD)?;
        self.at += 1;
        Ok(b)
    }

    fn uint(&mut self) -> Res<u64> {
        let mut v = 0;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 127) << shift;
            if b < 128 {
                break;
            }
        }
        Ok(v)
    }

    fn int(&mut self) -> Res<i64> {
        let v = self.uint()?;
        Ok(if v & 1 == 1 {
            !(v >> 1) as i64
        } else {
            (v >> 1) as i64
        })
    }

    fn float(&mut self) -> Res<f32> {
        let b = self.byte()?;
        if b == 0 {
            return Ok(0.0);
        }
        let bits = u32::from_le_bytes([b, self.byte()?, self.byte()?, self.byte()?]);
        Ok(f32::from_bits(bits.rotate_left(23)))
    }

    fn string(&mut self) -> Res<String> {
        let end = self.b[self.at..].iter().position(|&c| c == 0).ok_or(BAD)?;
        let s = String::from_utf8_lossy(&self.b[self.at..self.at + end]).into_owned();
        self.at += end + 1;
        Ok(s)
    }
}

struct Field {
    name: String,
    ty: i64,
    array: bool,
    id: u64,
}

struct Type {
    name: String,
    kind: u8,
    fields: Vec<Field>,
}

fn schema(b: &[u8]) -> Res<Vec<Type>> {
    let mut k = Kiwi { b, at: 0 };
    (0..k.uint()?)
        .map(|_| {
            let (name, kind) = (k.string()?, k.byte()?);
            let fields = (0..k.uint()?)
                .map(|_| {
                    Ok(Field {
                        name: k.string()?,
                        ty: k.int()?,
                        array: k.byte()? > 0,
                        id: k.uint()?,
                    })
                })
                .collect::<Res<_>>()?;
            Ok(Type { name, kind, fields })
        })
        .collect()
}

fn value(k: &mut Kiwi, types: &[Type], ty: i64, array: bool) -> Res<Value> {
    if array {
        let n = k.uint()?;
        if ty == -2 {
            let bytes = k.b.get(k.at..k.at + n as usize).ok_or(BAD)?;
            k.at += n as usize;
            return Ok(Value::Array(bytes.iter().map(|&b| b.into()).collect()));
        }
        return (0..n).map(|_| value(k, types, ty, false)).collect();
    }
    Ok(match ty {
        -1 => Value::Bool(k.byte()? > 0),
        -2 => k.byte()?.into(),
        -3 | -7 => k.int()?.into(),
        -4 | -8 => k.uint()?.into(),
        -5 => serde_json::Number::from_f64(k.float()?.into()).map_or(Value::Null, Value::Number),
        -6 => k.string()?.into(),
        _ => {
            let t = types
                .get(usize::try_from(ty).map_err(|_| BAD)?)
                .ok_or(BAD)?;
            match t.kind {
                0 => {
                    let v = k.uint()?;
                    t.fields
                        .iter()
                        .find(|f| f.id == v)
                        .map_or(Value::Null, |f| f.name.clone().into())
                }
                1 => {
                    let mut m = Map::new();
                    for f in &t.fields {
                        m.insert(f.name.clone(), value(k, types, f.ty, f.array)?);
                    }
                    Value::Object(m)
                }
                _ => {
                    let mut m = Map::new();
                    loop {
                        let id = k.uint()?;
                        if id == 0 {
                            break;
                        }
                        let f = t.fields.iter().find(|f| f.id == id).ok_or(BAD)?;
                        m.insert(f.name.clone(), value(k, types, f.ty, f.array)?);
                    }
                    Value::Object(m)
                }
            }
        }
    })
}

/// The segments after the header: each a little endian length and raw deflate or zstd data.
fn segments(b: &[u8]) -> Res<Vec<Vec<u8>>> {
    let mut at = 12;
    let mut out = Vec::new();
    while at + 4 <= b.len() {
        let n = u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize;
        let data = b.get(at + 4..at + 4 + n).ok_or(BAD)?;
        out.push(if data.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
            let mut d = ruzstd::decoding::StreamingDecoder::new(data).map_err(|_| BAD)?;
            let mut v = Vec::new();
            std::io::Read::read_to_end(&mut d, &mut v).map_err(|_| BAD)?;
            v
        } else {
            miniz_oxide::inflate::decompress_to_vec(data).map_err(|_| BAD)?
        });
        at += 4 + n;
    }
    Ok(out)
}

/// The message of the Figma file `bytes` and the images of its zip by hash.
fn read(bytes: &[u8]) -> Res<(Value, Files)> {
    let (canvas, images) = if bytes.starts_with(b"PK") {
        let mut files = unzip(bytes)?;
        let i = files.iter().position(|f| f.0 == "canvas.fig").ok_or(BAD)?;
        let canvas = files.swap_remove(i).1;
        let images = files
            .into_iter()
            .filter_map(|(n, b)| Some((n.strip_prefix("images/")?.to_string(), b)))
            .collect();
        (canvas, images)
    } else {
        (bytes.to_vec(), vec![])
    };
    if !canvas.starts_with(b"fig-kiwi") {
        return Err(BAD.into());
    }
    let [schema_bytes, data, ..] = &segments(&canvas)?[..] else {
        return Err(BAD.into());
    };
    let types = schema(schema_bytes)?;
    let root = types.iter().position(|t| t.name == "Message").ok_or(BAD)?;
    Ok((
        value(&mut Kiwi { b: data, at: 0 }, &types, root as i64, false)?,
        images,
    ))
}

const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn num(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

fn affine(m: &Value) -> [f32; 6] {
    if m.is_null() {
        return IDENTITY;
    }
    ["m00", "m10", "m01", "m11", "m02", "m12"].map(|k| num(&m[k]))
}

fn inverse([a, b, c, d, e, f]: [f32; 6]) -> [f32; 6] {
    let det = a * d - b * c;
    if det == 0.0 {
        return IDENTITY;
    }
    [
        d / det,
        -b / det,
        -c / det,
        a / det,
        (c * f - d * e) / det,
        (b * e - a * f) / det,
    ]
}

fn bytes(v: &Value) -> Vec<u8> {
    v.as_array().map_or(vec![], |a| {
        a.iter().map(|b| b.as_u64().unwrap_or(0) as u8).collect()
    })
}

fn color(c: &Value, opacity: f32) -> Color {
    let ch = |k: &str, by: f32| (num(&c[k]).clamp(0.0, 1.0) * by * 255.0).round() as u8;
    rgba([ch("r", 1.0), ch("g", 1.0), ch("b", 1.0), ch("a", opacity)])
}

/// `SCREAMING_SNAKE` to the camel case of Satz's names.
fn blend(v: &Value) -> Blend {
    let words: Vec<String> = v
        .as_str()
        .unwrap_or("")
        .split('_')
        .map(str::to_lowercase)
        .collect();
    let name = words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i == 0 {
                w.clone()
            } else {
                w[..1].to_uppercase() + &w[1..]
            }
        })
        .collect::<String>();
    serde_json::from_value(name.into()).unwrap_or(Blend::Normal)
}

struct Fig<'a> {
    blobs: Vec<Vec<u8>>,
    images: Files,
    kids: HashMap<String, Vec<&'a Value>>,
}

fn id(guid: &Value) -> String {
    format!("{}:{}", guid["sessionID"], guid["localID"])
}

impl Fig<'_> {
    /// The path of blob `i`, a byte of command and its little endian points, through `t`.
    fn path(&self, i: &Value, t: [f32; 6]) -> Vec<f32> {
        let Some(b) = i.as_u64().and_then(|i| self.blobs.get(i as usize)) else {
            return vec![];
        };
        let [a, bb, c, d, e, f] = t;
        let mut out = Vec::new();
        let (mut at, mut open) = (0, false);
        let pts = |n: usize, at: &mut usize| -> Option<Vec<[f32; 2]>> {
            let v = (0..n)
                .map(|k| {
                    let p = |o: usize| {
                        b.get(*at + o..*at + o + 4)
                            .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
                    };
                    let (x, y) = (p(k * 8)?, p(k * 8 + 4)?);
                    Some([a * x + c * y + e, bb * x + d * y + f])
                })
                .collect();
            *at += n * 8;
            v
        };
        while let Some(&cmd) = b.get(at) {
            at += 1;
            let Some(p) = pts(
                [0, 1, 1, 2, 3].get(cmd as usize).copied().unwrap_or(0),
                &mut at,
            ) else {
                break;
            };
            match cmd {
                0 if open => {
                    out.push(CLOSE);
                    open = false;
                }
                0 => {}
                1 => {
                    out.extend([MOVE, p[0][0], p[0][1]]);
                    open = true;
                }
                2 => out.extend([LINE, p[0][0], p[0][1]]),
                3 => quad(&mut out, p[0], p[1]),
                4 => out.extend([CUBIC, p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1]]),
                _ => break,
            }
        }
        out
    }

    fn paints(&self, paints: &Value, size: [f32; 2], t: [f32; 6]) -> Vec<Fill> {
        let box_ = then(t, [size[0], 0.0, 0.0, size[1], 0.0, 0.0]);
        paints
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["visible"] != false)
            .filter_map(|p| {
                let opacity = p["opacity"].as_f64().unwrap_or(1.0) as f32;
                let stops = || {
                    p["stops"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|s| FillStop {
                            at: num(&s["position"]),
                            color: color(&s["color"], opacity),
                        })
                        .collect()
                };
                let to = |adjust| then(box_, then(inverse(affine(&p["transform"])), adjust));
                Some(match p["type"].as_str()? {
                    "SOLID" => Fill::solid(color(&p["color"], opacity)),
                    "GRADIENT_LINEAR" => gradient(
                        FillKind::Linear,
                        to([1.0, 0.0, 0.0, 1.0, 0.0, 0.5]),
                        stops(),
                    ),
                    "GRADIENT_RADIAL" | "GRADIENT_DIAMOND" => gradient(
                        FillKind::Radial,
                        to([0.5, 0.0, 0.0, 0.5, 0.5, 0.5]),
                        stops(),
                    ),
                    _ => return None,
                })
            })
            .collect()
    }

    fn draw(&self, n: &Value, t: [f32; 6], opacity: f32, out: &mut Vec<Item>) {
        if n["visible"] == false {
            return;
        }
        let opacity = opacity * n["opacity"].as_f64().unwrap_or(1.0) as f32;
        let size = [num(&n["size"]["x"]), num(&n["size"]["y"])];
        let fills = self.paints(&n["fillPaints"], size, t);
        let mut shape = |path: Vec<f32>, fills: Vec<Fill>| {
            out.push(Item::Shape(Shape {
                path,
                fills,
                strokes: vec![],
                weight: 1.0,
                join: Join::Miter,
                cap: Cap::None,
                opacity,
                blend: blend(&n["blendMode"]),
            }))
        };
        for g in n["fillGeometry"].as_array().into_iter().flatten() {
            shape(self.path(&g["commandsBlob"], t), fills.clone());
        }
        for g in n["textData"]["glyphs"].as_array().into_iter().flatten() {
            let s = num(&g["fontSize"]);
            let at = [
                s,
                0.0,
                0.0,
                -s,
                num(&g["position"]["x"]),
                num(&g["position"]["y"]),
            ];
            shape(self.path(&g["commandsBlob"], then(t, at)), fills.clone());
        }
        let strokes = self.paints(&n["strokePaints"], size, t);
        for g in n["strokeGeometry"].as_array().into_iter().flatten() {
            shape(self.path(&g["commandsBlob"], t), strokes.clone());
        }
        for p in n["fillPaints"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["visible"] != false)
        {
            let hash: String = bytes(&p["image"]["hash"])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            let Some((_, image)) = self.images.iter().find(|i| i.0 == hash) else {
                continue;
            };
            let corners = [[0.0, 0.0], [size[0], 0.0], [0.0, size[1]], size]
                .map(|[x, y]| [t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]]);
            let (x, y) = (
                corners.iter().map(|c| c[0]).fold(f32::MAX, f32::min),
                corners.iter().map(|c| c[1]).fold(f32::MAX, f32::min),
            );
            let (r, b) = (
                corners.iter().map(|c| c[0]).fold(f32::MIN, f32::max),
                corners.iter().map(|c| c[1]).fold(f32::MIN, f32::max),
            );
            let rect = [x, y, r - x, b - y];
            out.push(Item::Image(image.clone(), rect, rect));
        }
        let key = if n["type"] == "INSTANCE" && !self.kids.contains_key(&id(&n["guid"])) {
            id(&n["symbolData"]["symbolID"])
        } else {
            id(&n["guid"])
        };
        for k in self.kids.get(&key).into_iter().flatten() {
            self.draw(k, then(t, affine(&k["transform"])), opacity, out);
        }
    }
}

/// A page for each frame on the canvases of the Figma file `bytes`, and one for what lies beside them.
pub fn sheets(file: &[u8]) -> Res<Vec<Sheet>> {
    let (msg, images) = read(file)?;
    let all = msg["nodeChanges"].as_array().ok_or(BAD)?;
    let mut fig = Fig {
        blobs: msg["blobs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|b| bytes(&b["bytes"]))
            .collect(),
        images,
        kids: HashMap::new(),
    };
    for n in all.iter().filter(|n| n["parentIndex"].is_object()) {
        fig.kids
            .entry(id(&n["parentIndex"]["guid"]))
            .or_default()
            .push(n);
    }
    for k in fig.kids.values_mut() {
        k.sort_by(|a, b| {
            a["parentIndex"]["position"]
                .as_str()
                .cmp(&b["parentIndex"]["position"].as_str())
        });
    }
    let kids = |n: &Value| {
        fig.kids
            .get(&id(&n["guid"]))
            .into_iter()
            .flatten()
            .filter(|n| n["visible"] != false)
    };
    let root = all.iter().find(|n| n["type"] == "DOCUMENT").ok_or(BAD)?;
    let mut out = Vec::new();
    for canvas in
        kids(root).filter(|c| c["type"] == "CANVAS" && c["name"] != "Internal Only Canvas")
    {
        for n in kids(canvas).filter(|n| n["type"] == "FRAME") {
            let mut items = Vec::new();
            fig.draw(n, IDENTITY, 1.0, &mut items);
            let name = n["name"].as_str().map(String::from);
            out.push(Sheet {
                name,
                width: num(&n["size"]["x"]).into(),
                height: num(&n["size"]["y"]).into(),
                items,
            });
        }
        let loose = |dx: f32, dy: f32| {
            let mut items = Vec::new();
            for n in kids(canvas).filter(|n| n["type"] != "FRAME") {
                fig.draw(
                    n,
                    then([1.0, 0.0, 0.0, 1.0, dx, dy], affine(&n["transform"])),
                    1.0,
                    &mut items,
                );
            }
            items
        };
        let paths = loose(0.0, 0.0)
            .into_iter()
            .filter_map(|i| {
                if let Item::Shape(s) = i {
                    Some(s.path)
                } else {
                    None
                }
            })
            .filter(|p| !p.is_empty());
        let Some([x, y, w, h]) = paths
            .map(|p| bounds(&p))
            .reduce(|[x, y, w, h], [a, b, c, d]| {
                let (l, t) = (x.min(a), y.min(b));
                [l, t, (x + w).max(a + c) - l, (y + h).max(b + d) - t]
            })
        else {
            continue;
        };
        out.push(Sheet {
            name: canvas["name"].as_str().map(String::from),
            width: w.into(),
            height: h.into(),
            items: loose(-x, -y),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_figma_file_imports_with_a_page_for_each_frame_and_its_text_as_outlines() {
        let bytes =
            crate::import::import(include_bytes!("../testdata/structure.fig"), "structure.fig")
                .unwrap();
        let d = crate::doc::Doc::load(&bytes).unwrap();
        let s = d.build_snapshot();
        let pages: Vec<_> = s
            .pages
            .iter()
            .map(|p| (p.children[0].name.as_str(), p.width, p.height))
            .collect();
        assert_eq!(
            pages,
            [
                ("Groups", 500.0, 500.0),
                ("Symbols and images", 500.0, 500.0)
            ]
        );
        let crate::doc::Kind::Frame { children, .. } = &s.pages[1].children[0].kind else {
            panic!()
        };
        assert!(
            children
                .iter()
                .any(|c| c.style.fills.iter().any(|f| f.kind == FillKind::Image))
        );
        assert!(children.len() > 20, "{}", children.len());
    }
}
