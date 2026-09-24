use crate::display_list::{Op, Paint, rect};
use crate::text::layout;
use loro::{
    Container, LoroDoc, LoroMap, LoroText, LoroTree, LoroValue, TreeID, TreeParentId, UndoManager,
    UpdateOptions, ValueOrContainer,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    Create {
        parent: String,
        kind: NewKind,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
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
    Set {
        id: String,
        name: Option<String>,
        fill: Option<u32>,
        size: Option<f64>,
        clip: Option<bool>,
    },
    Delete {
        ids: Vec<String>,
    },
    Group {
        ids: Vec<String>,
        frame: bool,
    },
    Ungroup {
        ids: Vec<String>,
    },
    Move {
        ids: Vec<String>,
        parent: String,
        index: usize,
    },
    Order {
        ids: Vec<String>,
        to: Order,
    },
    Duplicate {
        ids: Vec<String>,
    },
    Copy {
        ids: Vec<String>,
    },
    Paste {
        above: Vec<String>,
    },
    Undo,
    Redo,
    BeginUndoGroup,
    EndUndoGroup,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NewKind {
    Rect,
    Text,
    Frame,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    Forward,
    Backward,
    Front,
    Back,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub pages: Vec<Page>,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Page {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub bleed: f64,
    pub children: Vec<Node>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
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
    Rect {
        fill: u32,
    },
    Text {
        text: String,
        size: f64,
    },
    Group {
        children: Vec<Node>,
    },
    Frame {
        fill: u32,
        clip: bool,
        children: Vec<Node>,
    },
}

pub struct Doc {
    doc: LoroDoc,
    tree: LoroTree,
    undo: UndoManager,
    clipboard: Vec<(Clip, Option<TreeID>)>,
}

struct Clip {
    meta: LoroValue,
    children: Vec<Clip>,
}

type Res<T> = Result<T, String>;

const MM: f64 = 72.0 / 25.4;
const GRAY: u32 = 0xd9d9d9ff;
const WHITE: u32 = 0xffffffff;
const SAMPLE: &str = "Satz sets type in the browser. The engine shapes this paragraph \
with harfrust, breaks it into lines with the Knuth-Plass algorithm and justifies \
every line but the last to the width of its frame. The canvas and the PDF draw \
the same glyphs from the same font.";

impl Doc {
    pub fn new() -> Doc {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("nodes");
        tree.enable_fractional_index(0);
        let undo = UndoManager::new(&doc);
        let mut d = Doc {
            doc,
            tree,
            undo,
            clipboard: Vec::new(),
        };
        let page = d.tree.create(None).unwrap();
        let m = d.meta(page);
        m.insert("kind", "page").unwrap();
        m.insert("width", 148.0 * MM).unwrap();
        m.insert("height", 210.0 * MM).unwrap();
        m.insert("bleed", 3.0 * MM).unwrap();
        let mut add = |parent: &str, kind, [x, y, w, h]: [f64; 4], fill: Option<u32>| {
            let id = d
                .apply(Command::Create {
                    parent: parent.into(),
                    kind,
                    x: x * MM,
                    y: y * MM,
                    w: w * MM,
                    h: h * MM,
                })
                .unwrap()
                .remove(0);
            d.apply(Command::Set {
                id: id.clone(),
                name: None,
                fill,
                size: None,
                clip: None,
            })
            .unwrap();
            id
        };
        let page = page.to_string();
        add(
            &page,
            NewKind::Rect,
            [-3.0, -3.0, 154.0, 80.0],
            Some(0xe8452cff),
        );
        let text = add(&page, NewKind::Text, [15.0, 95.0, 118.0, 70.0], None);
        let frame = add(
            &page,
            NewKind::Frame,
            [15.0, 172.0, 118.0, 23.0],
            Some(0xf2efe8ff),
        );
        add(
            &frame,
            NewKind::Rect,
            [95.0, 180.0, 60.0, 40.0],
            Some(0x2c5fd9ff),
        );
        d.apply(Command::SetText {
            id: text.clone(),
            text: SAMPLE.into(),
        })
        .unwrap();
        d.apply(Command::Set {
            id: text,
            name: None,
            fill: None,
            size: Some(14.0),
            clip: None,
        })
        .unwrap();
        d.undo = UndoManager::new(&d.doc);
        d
    }

    pub fn apply(&mut self, cmd: Command) -> Res<Vec<String>> {
        let out = match cmd {
            Command::Create {
                parent,
                kind,
                x,
                y,
                w,
                h,
            } => {
                let p = self.node(&parent)?;
                let id = self.tree.create(p).map_err(err)?;
                let m = self.meta(id);
                let (name, fill) = match kind {
                    NewKind::Rect => ("rect", GRAY),
                    NewKind::Text => ("text", 0),
                    NewKind::Frame => ("frame", WHITE),
                };
                m.insert("kind", name).map_err(err)?;
                match kind {
                    NewKind::Text => {
                        m.insert("size", 12.0).map_err(err)?;
                        m.insert_container("text", LoroText::new())
                            .map_err(err)?
                            .insert(0, "Text")
                            .map_err(err)?;
                    }
                    NewKind::Frame => {
                        m.insert("fill", fill as i64).map_err(err)?;
                        m.insert("clip", true).map_err(err)?;
                    }
                    NewKind::Rect => m.insert("fill", fill as i64).map_err(err)?,
                }
                self.set_frame(id, [x, y, w, h])?;
                vec![id.to_string()]
            }
            Command::SetFrame { id, x, y, w, h } => {
                self.set_frame(self.node(&id)?, [x, y, w, h])?;
                vec![]
            }
            Command::SetText { id, text } => {
                let t = container(&self.meta(self.node(&id)?), "text")
                    .and_then(|c| c.into_text().ok())
                    .ok_or("not a text node")?;
                t.update(&text, UpdateOptions::default())
                    .map_err(|e| format!("{e:?}"))?;
                vec![]
            }
            Command::Set {
                id,
                name,
                fill,
                size,
                clip,
            } => {
                let m = self.meta(self.node(&id)?);
                if let Some(v) = name {
                    m.insert("name", v).map_err(err)?;
                }
                if let Some(v) = fill {
                    m.insert("fill", v as i64).map_err(err)?;
                }
                if let Some(v) = size {
                    m.insert("size", v).map_err(err)?;
                }
                if let Some(v) = clip {
                    m.insert("clip", v).map_err(err)?;
                }
                vec![]
            }
            Command::Delete { ids } => {
                for id in self.nodes(&ids)? {
                    let parent = self.tree.parent(id);
                    self.remove(id)?;
                    self.prune(parent)?;
                }
                vec![]
            }
            Command::Group { ids, frame } => {
                let ids = self.sorted(&ids)?;
                let top = *ids.last().ok_or("nothing to group")?;
                let parent = self.tree.parent(top).ok_or("no parent")?;
                let index = self.index(top) + 1;
                let bounds = union(ids.iter().map(|&id| self.bounds(id)));
                let g = self.tree.create_at(parent, index).map_err(err)?;
                let m = self.meta(g);
                if frame {
                    m.insert("kind", "frame").map_err(err)?;
                    m.insert("fill", 0).map_err(err)?;
                    m.insert("clip", true).map_err(err)?;
                    self.set_frame(g, bounds)?;
                } else {
                    m.insert("kind", "group").map_err(err)?;
                }
                let olds: Vec<_> = ids.iter().map(|&id| self.tree.parent(id)).collect();
                for (i, &id) in ids.iter().enumerate() {
                    self.tree.mov_to(id, g, i).map_err(err)?;
                }
                for p in olds {
                    self.prune(p)?;
                }
                vec![g.to_string()]
            }
            Command::Ungroup { ids } => {
                let mut out = Vec::new();
                for g in self.nodes(&ids)? {
                    if !matches!(self.kind(g).as_str(), "group" | "frame") {
                        continue;
                    }
                    let parent = self.tree.parent(g).ok_or("no parent")?;
                    let index = self.index(g);
                    for (i, c) in self.children(g).into_iter().enumerate() {
                        self.tree.mov_to(c, parent, index + i).map_err(err)?;
                        out.push(c.to_string());
                    }
                    self.remove(g)?;
                }
                out
            }
            Command::Move { ids, parent, index } => {
                let p = self.node(&parent)?;
                if !matches!(self.kind(p).as_str(), "page" | "group" | "frame") {
                    return Err("not a container".into());
                }
                let ids = self.sorted(&ids)?;
                let olds: Vec<_> = ids.iter().map(|&id| self.tree.parent(id)).collect();
                let others = self
                    .children(p)
                    .into_iter()
                    .filter(|c| !ids.contains(c))
                    .count();
                for (i, &id) in ids.iter().enumerate() {
                    self.tree
                        .mov_to(id, p, index.min(others) + i)
                        .map_err(err)?;
                }
                for p in olds {
                    self.prune(p)?;
                }
                vec![]
            }
            Command::Order { ids, to } => {
                let mut ids = self.sorted(&ids)?;
                if matches!(to, Order::Forward | Order::Back) {
                    ids.reverse();
                }
                for id in ids {
                    let parent = self.tree.parent(id).ok_or("no parent")?;
                    let (i, n) = (self.index(id), self.children(parent).len());
                    let to = match to {
                        Order::Forward => (i + 1).min(n - 1),
                        Order::Backward => i.saturating_sub(1),
                        Order::Front => n - 1,
                        Order::Back => 0,
                    };
                    self.tree.mov_to(id, parent, to).map_err(err)?;
                }
                vec![]
            }
            Command::Undo => {
                self.undo.group_end();
                self.undo.undo().map_err(err)?;
                vec![]
            }
            Command::Redo => {
                self.undo.group_end();
                self.undo.redo().map_err(err)?;
                vec![]
            }
            Command::BeginUndoGroup => {
                self.undo.group_end();
                self.undo.group_start().map_err(err)?;
                vec![]
            }
            Command::EndUndoGroup => {
                self.undo.group_end();
                vec![]
            }
            Command::Duplicate { ids } => {
                let mut out = Vec::new();
                for id in self.sorted(&ids)?.into_iter().rev() {
                    let parent = self.tree.parent(id).ok_or("no parent")?;
                    let copy = self.paste(&self.clip(id), parent, self.index(id) + 1)?;
                    out.push(copy.to_string());
                }
                out.reverse();
                out
            }
            Command::Copy { ids } => {
                self.clipboard = self
                    .sorted(&ids)?
                    .into_iter()
                    .map(|id| (self.clip(id), self.tree.parent(id).and_then(parent_node)))
                    .collect();
                vec![]
            }
            Command::Paste { above } => {
                let above = self.sorted(&above)?.pop();
                let mut out = Vec::new();
                for (i, (clip, from)) in self.clipboard.iter().enumerate() {
                    let (parent, index) = match above {
                        Some(a) => (
                            self.tree.parent(a).ok_or("no parent")?,
                            self.index(a) + 1 + i,
                        ),
                        None => {
                            let p = from
                                .filter(|&p| self.node(&p.to_string()).is_ok())
                                .or_else(|| {
                                    self.tree
                                        .roots()
                                        .into_iter()
                                        .find(|&r| self.kind(r) == "page")
                                })
                                .ok_or("no page")?;
                            (p.into(), self.children(p).len())
                        }
                    };
                    out.push(self.paste(clip, parent, index)?.to_string());
                }
                out
            }
        };
        self.doc.commit();
        Ok(out)
    }

    pub fn snapshot(&self) -> Snapshot {
        let pages = self
            .tree
            .roots()
            .into_iter()
            .filter(|&p| self.kind(p) == "page")
            .map(|p| {
                let m = self.meta(p);
                Page {
                    id: p.to_string(),
                    width: num(&m, "width"),
                    height: num(&m, "height"),
                    bleed: num(&m, "bleed"),
                    children: self.children(p).into_iter().map(|c| self.snap(c)).collect(),
                }
            })
            .collect();
        Snapshot {
            pages,
            can_undo: self.undo.can_undo(),
            can_redo: self.undo.can_redo(),
        }
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
        for n in &p.children {
            draw(n, &mut ops);
        }
        ops
    }

    pub fn hit(&self, page: usize, x: f64, y: f64) -> Vec<String> {
        let mut path = Vec::new();
        if let Some(p) = self.snapshot().pages.get(page) {
            hit(&p.children, x, y, &mut path);
        }
        path
    }

    fn snap(&self, id: TreeID) -> Node {
        let m = self.meta(id);
        let children = || {
            self.children(id)
                .into_iter()
                .map(|c| self.snap(c))
                .collect()
        };
        let fill = || {
            value(&m, "fill")
                .and_then(|v| v.into_i64().ok())
                .unwrap_or(0) as u32
        };
        let kind = match self.kind(id).as_str() {
            "text" => Kind::Text {
                text: container(&m, "text")
                    .and_then(|c| c.into_text().ok())
                    .map(|t| t.to_string())
                    .unwrap_or_default(),
                size: num(&m, "size"),
            },
            "group" => Kind::Group {
                children: children(),
            },
            "frame" => Kind::Frame {
                fill: fill(),
                clip: value(&m, "clip").and_then(|v| v.into_bool().ok()) == Some(true),
                children: children(),
            },
            _ => Kind::Rect { fill: fill() },
        };
        let name = value(&m, "name")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| match &kind {
                Kind::Rect { .. } => "Rectangle".into(),
                Kind::Text { text, .. } => text.chars().take(40).collect(),
                Kind::Group { .. } => "Group".into(),
                Kind::Frame { .. } => "Frame".into(),
            });
        let [x, y, w, h] = self.bounds(id);
        Node {
            id: id.to_string(),
            name,
            x,
            y,
            w,
            h,
            kind,
        }
    }

    fn set_frame(&self, id: TreeID, [x, y, w, h]: [f64; 4]) -> Res<()> {
        let [ox, oy, ow, oh] = self.bounds(id);
        match self.kind(id).as_str() {
            "group" => {
                let sx = if ow > 0.0 { w / ow } else { 1.0 };
                let sy = if oh > 0.0 { h / oh } else { 1.0 };
                for c in self.children(id) {
                    let [cx, cy, cw, ch] = self.bounds(c);
                    self.set_frame(
                        c,
                        [x + (cx - ox) * sx, y + (cy - oy) * sy, cw * sx, ch * sy],
                    )?;
                }
                return Ok(());
            }
            "frame" => {
                for c in self.children(id) {
                    let [cx, cy, cw, ch] = self.bounds(c);
                    self.set_frame(c, [cx + x - ox, cy + y - oy, cw, ch])?;
                }
            }
            _ => {}
        }
        let m = self.meta(id);
        for (k, v) in [("x", x), ("y", y), ("w", w), ("h", h)] {
            m.insert(k, v).map_err(err)?;
        }
        Ok(())
    }

    fn bounds(&self, id: TreeID) -> [f64; 4] {
        if self.kind(id) == "group" {
            return union(self.children(id).into_iter().map(|c| self.bounds(c)));
        }
        let m = self.meta(id);
        ["x", "y", "w", "h"].map(|k| num(&m, k))
    }

    fn clip(&self, id: TreeID) -> Clip {
        Clip {
            meta: self.meta(id).get_deep_value(),
            children: self
                .children(id)
                .into_iter()
                .map(|c| self.clip(c))
                .collect(),
        }
    }

    fn paste(&self, clip: &Clip, parent: TreeParentId, index: usize) -> Res<TreeID> {
        let new = self.tree.create_at(parent, index).map_err(err)?;
        let to = self.meta(new);
        for (k, v) in clip.meta.clone().into_map().unwrap().iter() {
            match (k.as_str(), v) {
                ("text", LoroValue::String(t)) => to
                    .insert_container(k, LoroText::new())
                    .map_err(err)?
                    .insert(0, t)
                    .map_err(err)?,
                _ => to.insert(k, v.clone()).map_err(err)?,
            }
        }
        for (i, c) in clip.children.iter().enumerate() {
            self.paste(c, new.into(), i)?;
        }
        Ok(new)
    }

    fn prune(&self, parent: Option<TreeParentId>) -> Res<()> {
        if let Some(TreeParentId::Node(p)) = parent
            && self.kind(p) == "group"
            && self.children(p).is_empty()
        {
            let up = self.tree.parent(p);
            self.remove(p)?;
            self.prune(up)?;
        }
        Ok(())
    }

    /// Deleted nodes move under a trash root so that undo restores them with their id.
    fn remove(&self, id: TreeID) -> Res<()> {
        let trash = match self
            .tree
            .roots()
            .into_iter()
            .find(|&r| self.kind(r) == "trash")
        {
            Some(t) => t,
            None => {
                let t = self.tree.create(None).map_err(err)?;
                self.meta(t).insert("kind", "trash").map_err(err)?;
                t
            }
        };
        self.tree.mov(id, trash).map_err(err)
    }

    fn node(&self, id: &str) -> Res<TreeID> {
        let t = TreeID::try_from(id).map_err(err)?;
        let mut root = t;
        while self.tree.contains(root) && self.tree.is_node_deleted(&root) == Ok(false) {
            match self.tree.parent(root) {
                Some(TreeParentId::Node(p)) => root = p,
                _ if self.kind(root) == "page" => return Ok(t),
                _ => break,
            }
        }
        Err(format!("no node {id}"))
    }

    fn nodes(&self, ids: &[String]) -> Res<Vec<TreeID>> {
        ids.iter().map(|id| self.node(id)).collect()
    }

    fn sorted(&self, ids: &[String]) -> Res<Vec<TreeID>> {
        let ids = self.nodes(ids)?;
        let mut all = Vec::new();
        for p in self.tree.roots() {
            self.walk(p, &mut all);
        }
        Ok(all.into_iter().filter(|n| ids.contains(n)).collect())
    }

    fn walk(&self, id: TreeID, out: &mut Vec<TreeID>) {
        out.push(id);
        for c in self.children(id) {
            self.walk(c, out);
        }
    }

    fn children(&self, id: impl Into<TreeParentId>) -> Vec<TreeID> {
        self.tree.children(id).unwrap_or_default()
    }

    fn index(&self, id: TreeID) -> usize {
        self.tree
            .parent(id)
            .and_then(|p| self.tree.children(p))
            .and_then(|c| c.iter().position(|&x| x == id))
            .unwrap_or(0)
    }

    fn meta(&self, id: TreeID) -> LoroMap {
        self.tree.get_meta(id).unwrap()
    }

    fn kind(&self, id: TreeID) -> String {
        value(&self.meta(id), "kind")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }
}

impl Default for Doc {
    fn default() -> Self {
        Doc::new()
    }
}

fn draw(n: &Node, ops: &mut Vec<Op>) {
    let frame = [n.x, n.y, n.w, n.h].map(|v| v as f32);
    let item = |ops: &mut Vec<Op>, body: Vec<Op>| {
        let key = n.id.bytes().fold(0x811c9dc5u32, |h, b| {
            (h ^ b as u32).wrapping_mul(0x01000193)
        });
        ops.push(Op::BeginItem { item: key });
        ops.extend(body);
        ops.push(Op::EndItem);
    };
    let fill = |color: u32| Op::FillPath {
        paint: Paint::Solid {
            color: color.to_be_bytes().map(|c| c as f32 / 255.0),
        },
        path: rect(frame[0], frame[1], frame[2], frame[3]),
    };
    match &n.kind {
        Kind::Rect { fill: c } => item(ops, vec![fill(*c)]),
        Kind::Text { text, size } => {
            let black = Paint::Solid {
                color: [0.0, 0.0, 0.0, 1.0],
            };
            item(ops, layout(text, *size as f32, frame, &black))
        }
        Kind::Group { children } => children.iter().for_each(|c| draw(c, ops)),
        Kind::Frame {
            fill: c,
            clip,
            children,
        } => {
            if c & 0xff != 0 {
                item(ops, vec![fill(*c)]);
            }
            if *clip && (n.w <= 0.0 || n.h <= 0.0) {
                return;
            }
            if *clip {
                ops.push(Op::PushClip {
                    path: rect(frame[0], frame[1], frame[2], frame[3]),
                    invert: false,
                });
            }
            children.iter().for_each(|c| draw(c, ops));
            if *clip {
                ops.push(Op::PopClip);
            }
        }
    }
}

fn hit(nodes: &[Node], x: f64, y: f64, path: &mut Vec<String>) -> bool {
    for n in nodes.iter().rev() {
        let inside = x >= n.x && x <= n.x + n.w && y >= n.y && y <= n.y + n.h;
        path.push(n.id.clone());
        let found = match &n.kind {
            Kind::Group { children } => hit(children, x, y, path),
            Kind::Frame { children, clip, .. } => {
                (inside || !clip) && hit(children, x, y, path) || inside
            }
            _ => inside,
        };
        if found {
            return true;
        }
        path.pop();
    }
    false
}

fn parent_node(p: TreeParentId) -> Option<TreeID> {
    match p {
        TreeParentId::Node(p) => Some(p),
        _ => None,
    }
}

fn union(boxes: impl Iterator<Item = [f64; 4]>) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for [x, y, w, h] in boxes {
        b = [b[0].min(x), b[1].min(y), b[2].max(x + w), b[3].max(y + h)];
    }
    if b[0] > b[2] {
        return [0.0; 4];
    }
    [b[0], b[1], b[2] - b[0], b[3] - b[1]]
}

fn err(e: impl Display) -> String {
    e.to_string()
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

    fn page(d: &Doc) -> Page {
        d.snapshot().pages.remove(0)
    }

    fn create(d: &mut Doc, parent: &str, kind: NewKind, frame: [f64; 4]) -> String {
        let [x, y, w, h] = frame;
        d.apply(Command::Create {
            parent: parent.into(),
            kind,
            x,
            y,
            w,
            h,
        })
        .unwrap()
        .remove(0)
    }

    fn empty() -> (Doc, String) {
        let mut d = Doc::new();
        let p = page(&d);
        let ids = p.children.iter().map(|n| n.id.clone()).collect();
        d.apply(Command::Delete { ids }).unwrap();
        (d, p.id)
    }

    fn frame(n: &Node) -> [f64; 4] {
        [n.x, n.y, n.w, n.h]
    }

    fn ids(nodes: &[Node]) -> Vec<String> {
        nodes.iter().map(|n| n.id.clone()).collect()
    }

    fn children(n: &Node) -> &[Node] {
        match &n.kind {
            Kind::Group { children } | Kind::Frame { children, .. } => children,
            _ => &[],
        }
    }

    #[test]
    fn default_doc_has_a_page_with_rect_text_and_clipped_frame() {
        let s = Doc::new().snapshot();
        assert_eq!(s.pages.len(), 1);
        let p = &s.pages[0];
        assert!((p.width - 419.53).abs() < 0.01);
        assert!((p.bleed - 8.50).abs() < 0.01);
        assert!(matches!(
            p.children[0].kind,
            Kind::Rect { fill: 0xe8452cff }
        ));
        assert!(matches!(p.children[1].kind, Kind::Text { size: 14.0, .. }));
        assert!(
            matches!(&p.children[2].kind, Kind::Frame { clip: true, children, .. } if children.len() == 1)
        );
    }

    #[test]
    fn create_appends_on_top_with_figma_defaults() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [1.0, 2.0, 3.0, 4.0]);
        let b = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 9.0, 9.0]);
        let t = create(&mut d, &b, NewKind::Text, [0.0, 0.0, 9.0, 9.0]);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [a, b]);
        let n = &pg.children[0];
        assert_eq!(
            (n.name.as_str(), frame(n)),
            ("Rectangle", [1.0, 2.0, 3.0, 4.0])
        );
        assert_eq!(n.kind, Kind::Rect { fill: GRAY });
        let f = &pg.children[1];
        assert!(matches!(
            f.kind,
            Kind::Frame {
                fill: WHITE,
                clip: true,
                ..
            }
        ));
        assert_eq!(children(f)[0].id, t);
        assert_eq!(children(f)[0].name, "Text");
    }

    #[test]
    fn set_changes_properties_and_unknown_nodes_are_errors() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        d.apply(Command::Set {
            id: a.clone(),
            name: Some("Bg".into()),
            fill: Some(0xff0000ff),
            size: None,
            clip: None,
        })
        .unwrap();
        let n = &page(&d).children[0];
        assert_eq!(
            (n.name.as_str(), &n.kind),
            ("Bg", &Kind::Rect { fill: 0xff0000ff })
        );
        let bad = Command::Delete {
            ids: vec!["nope".into()],
        };
        assert!(d.apply(bad).is_err());
        d.apply(Command::Delete {
            ids: vec![a.clone()],
        })
        .unwrap();
        assert!(d.apply(Command::Delete { ids: vec![a] }).is_err());
    }

    #[test]
    fn group_takes_the_place_of_the_topmost_child_and_has_union_bounds() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &p, NewKind::Rect, [50.0, 50.0, 1.0, 1.0]);
        let c = create(&mut d, &p, NewKind::Rect, [20.0, 30.0, 10.0, 10.0]);
        let g = d
            .apply(Command::Group {
                ids: vec![c.clone(), a.clone()],
                frame: false,
            })
            .unwrap()
            .remove(0);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [b.clone(), g.clone()]);
        assert_eq!(frame(&pg.children[1]), [0.0, 0.0, 30.0, 40.0]);
        assert_eq!(ids(children(&pg.children[1])), [a.clone(), c.clone()]);

        d.apply(Command::Ungroup { ids: vec![g] }).unwrap();
        assert_eq!(ids(&page(&d).children), [b, a, c]);
    }

    #[test]
    fn resizing_a_group_scales_children_and_moving_a_frame_moves_them() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &p, NewKind::Rect, [10.0, 10.0, 10.0, 10.0]);
        let g = d
            .apply(Command::Group {
                ids: vec![a, b],
                frame: false,
            })
            .unwrap()
            .remove(0);
        d.apply(Command::SetFrame {
            id: g.clone(),
            x: 100.0,
            y: 0.0,
            w: 40.0,
            h: 20.0,
        })
        .unwrap();
        let pg = page(&d);
        let kids = children(&pg.children[0]);
        assert_eq!(frame(&kids[0]), [100.0, 0.0, 20.0, 10.0]);
        assert_eq!(frame(&kids[1]), [120.0, 10.0, 20.0, 10.0]);

        let f = d
            .apply(Command::Group {
                ids: vec![g],
                frame: true,
            })
            .unwrap()
            .remove(0);
        d.apply(Command::SetFrame {
            id: f,
            x: 0.0,
            y: 5.0,
            w: 10.0,
            h: 10.0,
        })
        .unwrap();
        let pg = page(&d);
        assert_eq!(frame(&pg.children[0]), [0.0, 5.0, 10.0, 10.0]);
        let g = &children(&pg.children[0])[0];
        assert_eq!(frame(g), [0.0, 5.0, 40.0, 20.0]);
    }

    #[test]
    fn emptied_groups_disappear() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let g = d
            .apply(Command::Group {
                ids: vec![a.clone()],
                frame: false,
            })
            .unwrap()
            .remove(0);
        d.apply(Command::Group {
            ids: vec![g],
            frame: false,
        })
        .unwrap();
        d.apply(Command::Move {
            ids: vec![a.clone()],
            parent: p.clone(),
            index: 0,
        })
        .unwrap();
        assert_eq!(ids(&page(&d).children), [a]);
    }

    #[test]
    fn move_reorders_and_reparents() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let b = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let c = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let f = create(&mut d, &p, NewKind::Frame, [0.0; 4]);
        let mv = |d: &mut Doc, ids: &[&String], parent: &str, index| {
            d.apply(Command::Move {
                ids: ids.iter().map(|s| s.to_string()).collect(),
                parent: parent.into(),
                index,
            })
            .unwrap();
        };
        mv(&mut d, &[&a], &p, 2);
        assert_eq!(ids(&page(&d).children), [&b, &c, &a, &f].map(String::clone));
        mv(&mut d, &[&c, &b], &f, 0);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [&a, &f].map(String::clone));
        assert_eq!(ids(children(&pg.children[1])), [&b, &c].map(String::clone));
        let into_self = Command::Move {
            ids: vec![f.clone()],
            parent: f.clone(),
            index: 0,
        };
        assert!(d.apply(into_self).is_err());
        let into_leaf = Command::Move {
            ids: vec![b],
            parent: a,
            index: 0,
        };
        assert!(d.apply(into_leaf).is_err());
    }

    #[test]
    fn order_changes_z_order_within_the_parent() {
        let (mut d, p) = empty();
        let [a, b, c] = [0; 3].map(|_| create(&mut d, &p, NewKind::Rect, [0.0; 4]));
        let ids_of = |d: &Doc| ids(&page(d).children);
        let order = |d: &mut Doc, ids: &[&String], to| {
            d.apply(Command::Order {
                ids: ids.iter().map(|s| s.to_string()).collect(),
                to,
            })
            .unwrap();
            ids_of(d)
        };
        assert_eq!(
            order(&mut d, &[&a], Order::Forward),
            [&b, &a, &c].map(String::clone)
        );
        assert_eq!(
            order(&mut d, &[&c], Order::Back),
            [&c, &b, &a].map(String::clone)
        );
        assert_eq!(
            order(&mut d, &[&c, &b], Order::Front),
            [&a, &c, &b].map(String::clone)
        );
        assert_eq!(
            order(&mut d, &[&b], Order::Backward),
            [&a, &b, &c].map(String::clone)
        );
        assert_eq!(
            order(&mut d, &[&a], Order::Backward),
            [&a, &b, &c].map(String::clone)
        );
    }

    #[test]
    fn duplicate_copies_subtrees_above_the_originals() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 9.0, 9.0]);
        let t = create(&mut d, &f, NewKind::Text, [1.0, 1.0, 5.0, 5.0]);
        d.apply(Command::SetText {
            id: t,
            text: "Hi".into(),
        })
        .unwrap();
        let copy = d
            .apply(Command::Duplicate {
                ids: vec![f.clone()],
            })
            .unwrap()
            .remove(0);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [&f, &copy].map(String::clone));
        let (orig, dup) = (&pg.children[0], &pg.children[1]);
        assert_ne!(children(orig)[0].id, children(dup)[0].id);
        assert_eq!(children(dup)[0].kind, children(orig)[0].kind);
        assert_eq!(frame(&children(dup)[0]), [1.0, 1.0, 5.0, 5.0]);
    }

    #[test]
    fn paste_goes_above_the_target_or_back_into_the_source_parent() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 9.0, 9.0]);
        let t = create(&mut d, &f, NewKind::Text, [1.0, 1.0, 5.0, 5.0]);
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let b = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        d.apply(Command::Copy {
            ids: vec![t.clone(), f.clone()],
        })
        .unwrap();
        d.apply(Command::Delete {
            ids: vec![t.clone()],
        })
        .unwrap();
        let pasted = d
            .apply(Command::Paste {
                above: vec![a.clone()],
            })
            .unwrap();
        assert_eq!(pasted.len(), 2);
        let pg = page(&d);
        assert_eq!(
            ids(&pg.children),
            [&f, &a, &pasted[0], &pasted[1], &b].map(String::clone)
        );
        assert_eq!(children(&pg.children[2]).len(), 1);
        assert!(matches!(pg.children[3].kind, Kind::Text { .. }));
        let again = d.apply(Command::Paste { above: vec![] }).unwrap();
        let pg = page(&d);
        assert_eq!(pg.children.last().unwrap().id, again[0]);
        assert_eq!(ids(children(&pg.children[0])), [again[1].clone()]);
    }

    #[test]
    fn set_text_changes_text() {
        let mut d = Doc::new();
        let id = page(&d).children[1].id.clone();
        d.apply(Command::SetText {
            id: id.clone(),
            text: "Hallo".into(),
        })
        .unwrap();
        assert_eq!(
            page(&d).children[1].kind,
            Kind::Text {
                text: "Hallo".into(),
                size: 14.0
            }
        );
        assert!(
            d.apply(Command::SetText {
                id: page(&d).children[0].id.clone(),
                text: String::new()
            })
            .is_err()
        );
    }

    #[test]
    fn hit_returns_the_topmost_node_under_the_point() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &p, NewKind::Rect, [5.0, 5.0, 10.0, 10.0]);
        assert_eq!(d.hit(0, 7.0, 7.0), [b]);
        assert_eq!(d.hit(0, 2.0, 2.0), [a]);
        assert!(d.hit(0, 20.0, 2.0).is_empty());
        assert!(d.hit(1, 2.0, 2.0).is_empty());
    }

    #[test]
    fn hit_returns_the_path_down_to_the_deepest_node() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 100.0]);
        let a = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &f, NewKind::Rect, [50.0, 50.0, 10.0, 10.0]);
        let g = d
            .apply(Command::Group {
                ids: vec![a.clone(), b],
                frame: false,
            })
            .unwrap()
            .remove(0);
        assert_eq!(d.hit(0, 5.0, 5.0), [f.clone(), g.clone(), a]);
        assert_eq!(d.hit(0, 30.0, 30.0), [f]);
    }

    #[test]
    fn clipped_content_is_only_hit_inside_its_frame() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let a = create(&mut d, &f, NewKind::Rect, [5.0, 5.0, 20.0, 20.0]);
        assert!(d.hit(0, 15.0, 15.0).is_empty());
        assert_eq!(d.hit(0, 8.0, 8.0), [f.clone(), a.clone()]);
        d.apply(Command::Set {
            id: f.clone(),
            name: None,
            fill: None,
            size: None,
            clip: Some(false),
        })
        .unwrap();
        assert_eq!(d.hit(0, 15.0, 15.0), [f, a]);
    }

    #[test]
    fn undo_reverts_one_command_and_redo_reapplies_it() {
        let mut d = Doc::new();
        let before = d.snapshot();
        assert!(!before.can_undo);
        let id = page(&d).children[0].id.clone();
        d.apply(Command::Delete { ids: vec![id] }).unwrap();
        let after = d.snapshot();
        assert!(after.can_undo);
        d.apply(Command::Undo).unwrap();
        let undone = d.snapshot();
        assert_eq!(undone.pages, before.pages);
        assert!(undone.can_redo && !undone.can_undo);
        d.apply(Command::Redo).unwrap();
        assert_eq!(d.snapshot().pages, after.pages);
    }

    #[test]
    fn an_undo_group_is_undone_in_one_step() {
        let mut d = Doc::new();
        let before = d.snapshot().pages;
        let id = page(&d).children[0].id.clone();
        d.apply(Command::BeginUndoGroup).unwrap();
        for x in 1..4 {
            d.apply(Command::SetFrame {
                id: id.clone(),
                x: x as f64,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            })
            .unwrap();
        }
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().pages, before);
        assert!(!d.snapshot().can_undo);

        d.apply(Command::Redo).unwrap();
        d.apply(Command::BeginUndoGroup).unwrap();
        d.apply(Command::Delete { ids: vec![id] }).unwrap();
        d.apply(Command::EndUndoGroup).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(frame(&page(&d).children[0]), [3.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn undo_restores_structure_and_text_with_the_same_ids() {
        let mut d = Doc::new();
        let ids = ids(&page(&d).children);
        let g = d
            .apply(Command::Group {
                ids: ids[..2].to_vec(),
                frame: false,
            })
            .unwrap()
            .remove(0);
        let grouped = d.snapshot().pages;
        d.apply(Command::Ungroup { ids: vec![g] }).unwrap();
        d.apply(Command::SetText {
            id: ids[1].clone(),
            text: "x".into(),
        })
        .unwrap();
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().pages, grouped);
    }

    #[test]
    fn render_emits_items_and_clips_frame_children() {
        let ops = Doc::new().render(0);
        assert!(matches!(ops[0], Op::Page { .. }));
        assert_eq!(ops.iter().filter(|o| **o == Op::EndItem).count(), 4);
        assert!(ops.iter().any(|o| matches!(o, Op::GlyphRun { .. })));
        let push = ops.iter().position(|o| matches!(o, Op::PushClip { .. }));
        let pop = ops.iter().position(|o| *o == Op::PopClip);
        assert!(push.unwrap() < pop.unwrap());
        assert_eq!(pop.unwrap(), ops.len() - 1);
        assert!(Doc::new().render(9).is_empty());
    }
}
