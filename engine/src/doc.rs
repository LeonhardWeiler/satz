use crate::display_list::{Op, rect};
use crate::text::layout;
use loro::{
    Container, ContainerID, ContainerTrait, LoroDoc, LoroList, LoroMap, LoroText, LoroValue,
    UpdateOptions, ValueOrContainer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    SetFrame {
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    SetText {
        id: String,
        text: String,
    },
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Snapshot {
    pub pages: Vec<Page>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub bleed: f64,
    pub items: Vec<Item>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Item {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Kind {
    Rect { fill: u32 },
    Text { text: String, size: f64 },
}

pub struct Doc {
    doc: LoroDoc,
}

const MM: f64 = 72.0 / 25.4;
const SAMPLE: &str = "Satz sets type in the browser. The engine shapes this paragraph \
with harfrust, breaks it into lines with the Knuth-Plass algorithm and justifies \
every line but the last to the width of its frame. The canvas and the PDF draw \
the same glyphs from the same font.";

impl Doc {
    pub fn new() -> Doc {
        let doc = LoroDoc::new();
        let page = doc
            .get_list("pages")
            .push_container(LoroMap::new())
            .unwrap();
        page.insert("width", 148.0 * MM).unwrap();
        page.insert("height", 210.0 * MM).unwrap();
        page.insert("bleed", 3.0 * MM).unwrap();
        let items = page.insert_container("items", LoroList::new()).unwrap();
        let item = |kind: &str, x: f64, y: f64, w: f64, h: f64| {
            let m = items.push_container(LoroMap::new()).unwrap();
            m.insert("kind", kind).unwrap();
            for (k, v) in [("x", x), ("y", y), ("w", w), ("h", h)] {
                m.insert(k, v).unwrap();
            }
            m
        };
        item("rect", -3.0 * MM, -3.0 * MM, 154.0 * MM, 80.0 * MM)
            .insert("fill", 0xe8452cffu32 as i64)
            .unwrap();
        let text = item("text", 15.0 * MM, 95.0 * MM, 118.0 * MM, 90.0 * MM);
        text.insert("size", 14.0).unwrap();
        text.insert_container("text", LoroText::new())
            .unwrap()
            .insert(0, SAMPLE)
            .unwrap();
        doc.commit();
        Doc { doc }
    }

    pub fn apply(&mut self, cmd: Command) -> Result<(), String> {
        match cmd {
            Command::SetFrame { id, x, y, w, h } => {
                let m = self.item(&id)?;
                for (k, v) in [("x", x), ("y", y), ("w", w), ("h", h)] {
                    m.insert(k, v).map_err(|e| e.to_string())?;
                }
            }
            Command::SetText { id, text } => {
                let t = container(&self.item(&id)?, "text")
                    .and_then(|c| c.into_text().ok())
                    .ok_or("not a text item")?;
                t.update(&text, UpdateOptions::default())
                    .map_err(|e| format!("{e:?}"))?;
            }
        }
        self.doc.commit();
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        let pages = maps(&self.doc.get_list("pages"))
            .map(|p| Page {
                width: num(&p, "width"),
                height: num(&p, "height"),
                bleed: num(&p, "bleed"),
                items: container(&p, "items")
                    .and_then(|c| c.into_list().ok())
                    .map(|l| maps(&l).map(|m| item(&m)).collect())
                    .unwrap_or_default(),
            })
            .collect();
        Snapshot { pages }
    }

    pub fn render(&self, page: usize) -> Vec<Op> {
        let snap = self.snapshot();
        let Some(p) = snap.pages.get(page) else {
            return Vec::new();
        };
        let mut ops = vec![Op::Page {
            width: p.width as f32,
            height: p.height as f32,
            bleed: p.bleed as f32,
        }];
        for (i, it) in p.items.iter().enumerate() {
            ops.push(Op::BeginItem { item: i as u32 });
            let frame = [it.x, it.y, it.w, it.h].map(|v| v as f32);
            match &it.kind {
                Kind::Rect { fill } => ops.push(Op::FillPath {
                    color: fill.to_be_bytes().map(|c| c as f32 / 255.0),
                    path: rect(frame[0], frame[1], frame[2], frame[3]),
                }),
                Kind::Text { text, size } => ops.extend(layout(text, *size as f32, frame)),
            }
            ops.push(Op::EndItem);
        }
        ops
    }

    fn item(&self, id: &str) -> Result<LoroMap, String> {
        ContainerID::try_from(id)
            .ok()
            .and_then(|c| self.doc.get_container(c))
            .and_then(|c| c.into_map().ok())
            .ok_or_else(|| format!("no item {id}"))
    }
}

impl Default for Doc {
    fn default() -> Self {
        Doc::new()
    }
}

fn item(m: &LoroMap) -> Item {
    let kind = match value(m, "kind").and_then(|v| v.into_string().ok()) {
        Some(s) if s.as_str() == "text" => Kind::Text {
            text: container(m, "text")
                .and_then(|c| c.into_text().ok())
                .map(|t| t.to_string())
                .unwrap_or_default(),
            size: num(m, "size"),
        },
        _ => Kind::Rect {
            fill: value(m, "fill")
                .and_then(|v| v.into_i64().ok())
                .unwrap_or(0) as u32,
        },
    };
    Item {
        id: m.id().to_string(),
        x: num(m, "x"),
        y: num(m, "y"),
        w: num(m, "w"),
        h: num(m, "h"),
        kind,
    }
}

fn maps(list: &LoroList) -> impl Iterator<Item = LoroMap> + '_ {
    (0..list.len()).filter_map(|i| list.get(i)?.into_container().ok()?.into_map().ok())
}

fn container(m: &LoroMap, key: &str) -> Option<Container> {
    m.get(key).and_then(|v| v.into_container().ok())
}

fn value(m: &LoroMap, key: &str) -> Option<LoroValue> {
    m.get(key)
        .and_then(|v: ValueOrContainer| v.into_value().ok())
}

fn num(m: &LoroMap, key: &str) -> f64 {
    value(m, key)
        .and_then(|v| v.into_double().ok())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_doc_has_a_page_with_rect_and_text() {
        let s = Doc::new().snapshot();
        assert_eq!(s.pages.len(), 1);
        let p = &s.pages[0];
        assert!((p.width - 419.53).abs() < 0.01);
        assert!((p.bleed - 8.50).abs() < 0.01);
        assert!(matches!(p.items[0].kind, Kind::Rect { .. }));
        assert!(matches!(p.items[1].kind, Kind::Text { .. }));
    }

    #[test]
    fn commands_change_the_snapshot() {
        let mut d = Doc::new();
        let id = d.snapshot().pages[0].items[1].id.clone();
        d.apply(Command::SetFrame {
            id: id.clone(),
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        })
        .unwrap();
        d.apply(Command::SetText {
            id: id.clone(),
            text: "Hallo".into(),
        })
        .unwrap();
        let it = &d.snapshot().pages[0].items[1];
        assert_eq!((it.x, it.y, it.w, it.h), (1.0, 2.0, 3.0, 4.0));
        assert_eq!(
            it.kind,
            Kind::Text {
                text: "Hallo".into(),
                size: 14.0
            }
        );
    }

    #[test]
    fn unknown_item_is_an_error() {
        let cmd = Command::SetText {
            id: "nope".into(),
            text: String::new(),
        };
        assert!(Doc::new().apply(cmd).is_err());
    }

    #[test]
    fn render_emits_page_and_items() {
        let ops = Doc::new().render(0);
        assert!(matches!(ops[0], Op::Page { .. }));
        assert_eq!(ops.iter().filter(|o| **o == Op::EndItem).count(), 2);
        assert!(ops.iter().any(|o| matches!(o, Op::GlyphRun { .. })));
        assert!(Doc::new().render(9).is_empty());
    }
}
