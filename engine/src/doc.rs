use crate::color::{Color, ColorMode, Swatch};
use crate::display_list::{CLOSE, LINE, MOVE, Op, rect};
use crate::geom::{Shape, bounds, contains, fit, near, outline};
use crate::layout::{Align3, Direction, Layout, MainAlign, Size, Sizing, arrange};
use crate::style::{
    Align, Blend, Cap, Constraint, Constraints, Effect, EffectKind, Fill, FillKind, FillStop, Join,
    Style,
};
use crate::text::{self, Attrs, PARAGRAPH, STYLED, Span, TextAlign, TextStyle};
use crate::variable::{Collection, Mode, Modes, Palette, Scope, Value, Variable};
use loro::{
    Container, ExpandType, LoroDoc, LoroMap, LoroText, LoroTree, LoroValue, StyleConfig, TextDelta,
    TreeID, TreeParentId, UndoManager, UpdateOptions, ValueOrContainer,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::Range;

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
    /// Moves and resizes a layer; a frame's children follow their constraints
    /// unless `ignore_constraints`.
    SetFrame {
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        #[serde(default)]
        ignore_constraints: bool,
    },
    SetText {
        id: String,
        text: String,
    },
    /// Sets text attributes on a range in UTF-16 code units, or on the whole text.
    /// Paragraph attributes cover the whole paragraphs the range touches; setting a
    /// styled attribute detaches the text style there.
    Format {
        id: String,
        range: Option<[usize; 2]>,
        #[serde(flatten)]
        props: TextProps,
    },
    AddTextStyle {
        name: String,
        size: f64,
        line_height: f64,
        letter_spacing: f64,
        paragraph_spacing: f64,
    },
    SetTextStyle {
        id: String,
        name: Option<String>,
        size: Option<f64>,
        line_height: Option<f64>,
        letter_spacing: Option<f64>,
        paragraph_spacing: Option<f64>,
    },
    /// Removes a text style; text using it keeps its values.
    DeleteTextStyle {
        id: String,
    },
    Set {
        id: String,
        #[serde(flatten)]
        props: Props,
    },
    /// Sets a path shape's outline from points in page space.
    SetPath {
        id: String,
        path: Vec<f32>,
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
    /// Toggles one layer's mask flag, or wraps several layers in a mask group
    /// whose lowest layer masks the others.
    Mask {
        ids: Vec<String>,
    },
    /// Adds auto layout to one frame, or wraps layers in a new auto layout frame;
    /// direction, gap and padding follow the current arrangement.
    AutoLayout {
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
    SetDocument {
        raster_ppi: Option<f64>,
        color_mode: Option<ColorMode>,
    },
    Paste {
        above: Vec<String>,
    },
    AddSwatch {
        name: String,
        color: Color,
        spot: bool,
    },
    SetSwatch {
        id: String,
        name: Option<String>,
        color: Option<Color>,
        spot: Option<bool>,
    },
    /// Removes a swatch; what uses it keeps the colour it stood for.
    DeleteSwatch {
        id: String,
    },
    /// Adds a collection with one mode; returns both ids.
    AddCollection {
        name: String,
    },
    SetCollection {
        id: String,
        name: String,
    },
    /// Removes a collection and its variables.
    DeleteCollection {
        id: String,
    },
    SetMode {
        collection: String,
        id: String,
        name: String,
    },
    /// Removes a mode other than the last; layers using it fall back to the first.
    DeleteMode {
        collection: String,
        id: String,
    },
    /// Removes a variable; what uses it keeps the value it had.
    DeleteVariable {
        id: String,
    },
    /// Adds a mode whose values start as those of the first mode.
    AddMode {
        collection: String,
        name: String,
    },
    /// Adds a variable with `value` in every mode of its collection.
    AddVariable {
        collection: String,
        name: String,
        value: Value,
    },
    /// Renames a variable or sets its value in `mode`, by default the first.
    SetVariable {
        id: String,
        name: Option<String>,
        mode: Option<String>,
        value: Option<Value>,
    },
    /// Binds a number property of a layer or text style to a number variable, or
    /// unbinds it; see `BINDABLE` and `text::STYLED`.
    Bind {
        id: String,
        prop: String,
        variable: Option<String>,
    },
    /// Chooses the mode of a collection for a page or layer and what it contains;
    /// `None` inherits it.
    UseMode {
        id: String,
        collection: String,
        mode: Option<String>,
    },
    Undo,
    Redo,
    BeginUndoGroup,
    EndUndoGroup,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Props {
    pub name: Option<String>,
    pub clip: Option<bool>,
    pub radius: Option<f32>,
    pub count: Option<u32>,
    pub ratio: Option<f32>,
    pub path: Option<Vec<f32>>,
    pub fills: Option<Vec<Fill>>,
    pub constraints: Option<Constraints>,
    pub strokes: Option<Vec<Fill>>,
    pub stroke_weight: Option<f32>,
    pub stroke_align: Option<Align>,
    pub join: Option<Join>,
    pub cap: Option<Cap>,
    pub arrow_start: Option<bool>,
    pub arrow_end: Option<bool>,
    pub opacity: Option<f32>,
    pub blend: Option<Blend>,
    pub effects: Option<Vec<Effect>>,
    pub mask: Option<bool>,
    pub direction: Option<Direction>,
    pub gap: Option<f64>,
    pub padding_top: Option<f64>,
    pub padding_right: Option<f64>,
    pub padding_bottom: Option<f64>,
    pub padding_left: Option<f64>,
    pub align_main: Option<MainAlign>,
    pub align_cross: Option<Align3>,
    pub sizing: Option<Sizing>,
    pub absolute: Option<bool>,
}

impl Props {
    fn check(&self) -> Res<()> {
        let within = |v: Option<f64>, lo: f64, hi: f64, what: &str| match v {
            Some(v) if !(lo..=hi).contains(&v) => Err(format!("{what} must be in {lo}..={hi}")),
            _ => Ok(()),
        };
        let f = |v: Option<f32>| v.map(f64::from);
        within(f(self.stroke_weight), 0.0, f64::MAX, "stroke weight")?;
        within(f(self.radius), 0.0, f64::MAX, "radius")?;
        within(self.count.map(f64::from), 3.0, 60.0, "count")?;
        within(f(self.ratio), 0.01, 1.0, "ratio")?;
        within(f(self.opacity), 0.0, 1.0, "opacity")?;
        for p in [
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        ] {
            within(p, 0.0, f64::MAX, "padding")?;
        }
        for e in self.effects.iter().flatten() {
            within(Some(e.radius.into()), 0.0, f64::MAX, "blur")?;
            e.color.check()?;
        }
        for f in self.fills.iter().chain(&self.strokes).flatten() {
            f.color.check()?;
            for s in &f.stops {
                s.color.check()?;
            }
        }
        Ok(())
    }
}

/// See `text::Attrs`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps {
    pub size: Option<f64>,
    pub line_height: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub paragraph_spacing: Option<f64>,
    pub fill: Option<Color>,
    pub text_style: Option<String>,
    pub text_align: Option<TextAlign>,
}

impl TextProps {
    fn check(&self) -> Res<()> {
        check_text(
            self.size,
            self.line_height,
            self.letter_spacing,
            self.paragraph_spacing,
        )?;
        self.fill.as_ref().map_or(Ok(()), Color::check)
    }
}

fn check_text(
    size: Option<f64>,
    line_height: Option<f64>,
    letter_spacing: Option<f64>,
    paragraph_spacing: Option<f64>,
) -> Res<()> {
    let within = |v: Option<f64>, lo: f64, what: &str| match v {
        Some(v) if !(v >= lo && v.is_finite()) => Err(format!("{what} must be at least {lo}")),
        _ => Ok(()),
    };
    within(size, 0.1, "text size")?;
    within(line_height, 0.0, "line height")?;
    within(letter_spacing, -100.0, "letter spacing")?;
    within(paragraph_spacing, 0.0, "paragraph spacing")
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NewKind {
    Rect,
    Ellipse,
    Polygon,
    Star,
    Line,
    Arrow,
    Path,
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
    /// Resolution at which the PDF rasterizes shadows and blurs.
    pub raster_ppi: f64,
    pub color_mode: ColorMode,
    #[serde(flatten)]
    pub palette: Palette,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Page {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub bleed: f64,
    pub modes: Modes,
    pub children: Vec<Node>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Modes chosen on this layer.
    pub modes: Modes,
    /// Modes this layer's variables resolve in, its own and inherited.
    pub active_modes: Modes,
    /// Number variables by property.
    pub bindings: BTreeMap<String, String>,
    #[serde(flatten)]
    pub style: Style,
    #[serde(flatten)]
    pub layout: Layout,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Kind {
    Shape(Shape),
    Text { text: String, spans: Vec<Span> },
    Group { children: Vec<Node> },
    Frame { clip: bool, children: Vec<Node> },
}

pub struct Doc {
    doc: LoroDoc,
    tree: LoroTree,
    undo: UndoManager,
    clipboard: Vec<(Clip, Option<TreeID>)>,
}

struct Clip {
    meta: LoroValue,
    text: Vec<TextDelta>,
    children: Vec<Clip>,
}

type Res<T> = Result<T, String>;

const MM: f64 = 72.0 / 25.4;
/// Properties a number variable can bind to; lengths count in mm, opacity in %,
/// text size in pt.
const BINDABLE: [&str; 11] = [
    "size",
    "w",
    "h",
    "radius",
    "strokeWeight",
    "opacity",
    "gap",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
];
const WHITE: u32 = 0xffffffff;
const BLACK: u32 = 0x000000ff;
const SAMPLE: &str = "Satz sets type in the browser. The engine shapes this paragraph \
with harfrust, breaks it into lines with the Knuth-Plass algorithm and justifies \
every line but the last to the width of its frame. The canvas and the PDF draw \
the same glyphs from the same font.";

impl Doc {
    pub fn new() -> Doc {
        let doc = LoroDoc::new();
        doc.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::After,
        }));
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
        d.apply(Command::SetDocument {
            raster_ppi: Some(300.0),
            color_mode: Some(ColorMode::Rgb),
        })
        .unwrap();
        let mut add = |parent: &str, kind, [x, y, w, h]: [f64; 4], props: Props| {
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
                props,
            })
            .unwrap();
            id
        };
        let fill = |c| Props {
            fills: Some(vec![Fill::solid(c)]),
            ..Props::default()
        };
        let page = page.to_string();
        add(
            &page,
            NewKind::Rect,
            [-3.0, -3.0, 154.0, 80.0],
            fill(0xe8452cff),
        );
        let text = add(
            &page,
            NewKind::Text,
            [15.0, 95.0, 118.0, 70.0],
            Props::default(),
        );
        let frame = add(
            &page,
            NewKind::Frame,
            [15.0, 172.0, 118.0, 23.0],
            fill(0xf2efe8ff),
        );
        add(
            &frame,
            NewKind::Rect,
            [95.0, 180.0, 60.0, 40.0],
            fill(0x2c5fd9ff),
        );
        let mm = MM as f32;
        let gradient = |kind, transform, stops: &[(f32, u32)]| Fill {
            kind,
            transform,
            stops: stops
                .iter()
                .map(|&(at, color)| FillStop {
                    at,
                    color: color.into(),
                })
                .collect(),
            ..Fill::default()
        };
        let named = |name: &str| Props {
            name: Some(name.into()),
            ..Props::default()
        };
        let shapes = [
            add(
                &page,
                NewKind::Ellipse,
                [8.0, 22.0, 30.0, 30.0],
                Props {
                    fills: Some(vec![gradient(
                        FillKind::Radial,
                        [0.6, 0.0, 0.0, 0.6, 0.4, 0.4],
                        &[(0.0, 0xfff3c4ff), (1.0, 0xf5a623ff)],
                    )]),
                    effects: Some(vec![Effect {
                        y: 2.0 * mm,
                        radius: 3.0 * mm,
                        color: Color::Rgb(0x00000066),
                        ..Effect::default()
                    }]),
                    ..named("Sun")
                },
            ),
            add(
                &page,
                NewKind::Star,
                [44.0, 22.0, 30.0, 30.0],
                Props {
                    fills: Some(vec![gradient(
                        FillKind::Linear,
                        [0.0, 1.0, -1.0, 0.0, 0.5, 0.0],
                        &[(0.0, 0xffe066ff), (1.0, 0xff8a00ff)],
                    )]),
                    strokes: Some(vec![Fill::solid(WHITE)]),
                    stroke_weight: Some(1.5),
                    stroke_align: Some(Align::Outside),
                    ..named("Star")
                },
            ),
            add(
                &page,
                NewKind::Polygon,
                [80.0, 26.0, 28.0, 26.0],
                Props {
                    fills: Some(vec![Fill::solid(0x2c5fd9ff)]),
                    strokes: Some(vec![Fill::solid(0x1b2a6bff)]),
                    opacity: Some(0.8),
                    blend: Some(Blend::Multiply),
                    ..named("Triangle")
                },
            ),
            add(
                &page,
                NewKind::Rect,
                [112.0, 22.0, 30.0, 22.0],
                Props {
                    radius: Some(4.0 * mm),
                    fills: Some(vec![Fill::solid(0xffffffcc)]),
                    strokes: Some(vec![Fill::solid(BLACK)]),
                    stroke_align: Some(Align::Center),
                    effects: Some(vec![Effect {
                        kind: EffectKind::Blur,
                        radius: 2.0 * mm,
                        ..Effect::default()
                    }]),
                    ..named("Blurred tile")
                },
            ),
            add(
                &page,
                NewKind::Arrow,
                [112.0, 62.0, 30.0, 0.0],
                Props {
                    strokes: Some(vec![Fill::solid(WHITE)]),
                    stroke_weight: Some(2.0),
                    ..named("Arrow")
                },
            ),
        ];
        let masked = [
            add(
                &page,
                NewKind::Ellipse,
                [8.0, 56.0, 66.0, 16.0],
                Props {
                    mask: Some(true),
                    ..named("Mask")
                },
            ),
            add(
                &page,
                NewKind::Rect,
                [8.0, 56.0, 66.0, 16.0],
                Props {
                    fills: Some(vec![gradient(
                        FillKind::Linear,
                        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                        &[(0.0, 0x2c5fd9ff), (0.5, WHITE), (1.0, 0x1b2a6bff)],
                    )]),
                    ..named("Stripes")
                },
            ),
        ];
        let mut group = |ids: Vec<String>, name: &str| {
            let g = d
                .apply(Command::Group { ids, frame: false })
                .unwrap()
                .remove(0);
            d.apply(Command::Set {
                id: g.clone(),
                props: named(name),
            })
            .unwrap();
            g
        };
        let masked = group(masked.to_vec(), "Masked");
        group([shapes.to_vec(), vec![masked]].concat(), "Shapes");
        d.apply(Command::SetText {
            id: text.clone(),
            text: SAMPLE.into(),
        })
        .unwrap();
        d.apply(Command::Format {
            id: text,
            range: None,
            props: TextProps {
                size: Some(14.0),
                text_align: Some(TextAlign::Justify),
                ..TextProps::default()
            },
        })
        .unwrap();
        d.undo = UndoManager::new(&d.doc);
        d
    }

    pub fn apply(&mut self, cmd: Command) -> Res<Vec<String>> {
        let history = matches!(cmd, Command::Undo | Command::Redo);
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
                let mode = self.color_mode();
                let closed = Props {
                    fills: Some(vec![Fill::solid(Color::gray(mode))]),
                    stroke_align: Some(Align::Inside),
                    ..Props::default()
                };
                let open = |path: Vec<f32>| Props {
                    strokes: Some(vec![Fill::solid(Color::black(mode))]),
                    arrow_end: Some(kind == NewKind::Arrow),
                    path: Some(path),
                    ..Props::default()
                };
                let line = vec![MOVE, 0.0, 0.0, LINE, 1.0, 0.0];
                let (name, shape, props) = match kind {
                    NewKind::Rect => (
                        "shape",
                        "rect",
                        Props {
                            radius: Some(0.0),
                            ..closed
                        },
                    ),
                    NewKind::Ellipse => ("shape", "ellipse", closed),
                    NewKind::Polygon => (
                        "shape",
                        "polygon",
                        Props {
                            count: Some(3),
                            ..closed
                        },
                    ),
                    NewKind::Star => (
                        "shape",
                        "star",
                        Props {
                            count: Some(5),
                            ratio: Some(0.382),
                            ..closed
                        },
                    ),
                    NewKind::Line | NewKind::Arrow => ("shape", "path", open(line)),
                    NewKind::Path => ("shape", "path", open(Vec::new())),
                    NewKind::Text => {
                        m.insert_container("text", LoroText::new())
                            .map_err(err)?
                            .insert(0, "Text")
                            .map_err(err)?;
                        m.insert("size", 12.0).map_err(err)?;
                        (
                            "text",
                            "",
                            Props {
                                fills: Some(vec![Fill::solid(Color::black(mode))]),
                                ..Props::default()
                            },
                        )
                    }
                    NewKind::Frame => (
                        "frame",
                        "",
                        Props {
                            fills: Some(vec![Fill::solid(Color::white(mode))]),
                            clip: Some(true),
                            ..closed
                        },
                    ),
                };
                m.insert("kind", name).map_err(err)?;
                if !shape.is_empty() {
                    m.insert("shape", shape).map_err(err)?;
                }
                self.set(id, props)?;
                self.set_frame(id, [x, y, w, h])?;
                vec![id.to_string()]
            }
            Command::SetFrame {
                id,
                x,
                y,
                w,
                h,
                ignore_constraints,
            } => {
                let id = self.node(&id)?;
                let [.., ow, oh] = self.bounds(id);
                self.unbind(id, |p| p == "w" && w != ow || p == "h" && h != oh)?;
                let old = self.layout(id).sizing;
                let mut sizing = old;
                for (size, changed) in [
                    (&mut sizing.horizontal, w != ow),
                    (&mut sizing.vertical, h != oh),
                ] {
                    if changed {
                        *size = Size::Fixed;
                    }
                }
                if sizing != old {
                    self.meta(id).insert("sizing", loro(sizing)?).map_err(err)?;
                }
                self.resize(id, [x, y, w, h], !ignore_constraints)?;
                vec![]
            }
            Command::SetText { id, text } => {
                self.text(self.node(&id)?)?
                    .update(&text, UpdateOptions::default())
                    .map_err(|e| format!("{e:?}"))?;
                vec![]
            }
            Command::Format { id, range, props } => {
                let n = self.node(&id)?;
                let t = self.text(n)?;
                props.check()?;
                let len = t.len_utf16();
                let range = match range {
                    Some([a, b]) if a <= b && b <= len => Some(a..b),
                    Some(_) => return Err("range outside the text".into()),
                    None => None,
                };
                if let Some(s) = props.text_style.as_deref().filter(|s| !s.is_empty()) {
                    self.find::<TextStyle>("textStyles", s)?;
                }
                let serde_json::Value::Object(set) = serde_json::to_value(&props).map_err(err)?
                else {
                    return Err("props are not a map".into());
                };
                let set: Vec<_> = set.into_iter().filter(|(_, v)| !v.is_null()).collect();
                if props.text_style.is_none()
                    && set.iter().any(|(k, _)| STYLED.contains(&k.as_str()))
                {
                    self.detach(n, range.clone(), |_| true)?;
                }
                let text = t.to_string();
                let m = self.meta(n);
                for (k, v) in &set {
                    let cleared: &[&str] = if k == "textStyle" { &STYLED } else { &[] };
                    let r = match &range {
                        Some(r) if PARAGRAPH.contains(&k.as_str()) => Some(paragraphs(&text, r)),
                        r => r.clone(),
                    };
                    match r {
                        Some(r) if r.is_empty() => {}
                        Some(r) => {
                            t.mark_utf16(r.clone(), k, loro(v)?).map_err(err)?;
                            for c in cleared {
                                t.unmark_utf16(r.clone(), c).map_err(err)?;
                            }
                        }
                        None => {
                            if k == "fill" {
                                let c = props.fill.clone().unwrap();
                                m.insert("fills", loro(vec![Fill::solid(c)])?)
                                    .map_err(err)?;
                            } else {
                                m.insert(k, loro(v)?).map_err(err)?;
                            }
                            for c in [k.as_str()].iter().chain(cleared) {
                                if len > 0 {
                                    t.unmark_utf16(0..len, c).map_err(err)?;
                                }
                            }
                        }
                    }
                }
                if range.is_none() {
                    self.unbind(n, |p| set.iter().any(|(k, _)| k == p))?;
                }
                vec![]
            }
            Command::AddTextStyle {
                name,
                size,
                line_height,
                letter_spacing,
                paragraph_spacing,
            } => {
                let id = self.new_id();
                self.put_text_style(
                    None,
                    TextStyle {
                        id: id.clone(),
                        name,
                        size,
                        line_height,
                        letter_spacing,
                        paragraph_spacing,
                        bindings: BTreeMap::new(),
                    },
                )?;
                vec![id]
            }
            Command::SetTextStyle {
                id,
                name,
                size,
                line_height,
                letter_spacing,
                paragraph_spacing,
            } => {
                let (i, mut s) = self.find::<TextStyle>("textStyles", &id)?;
                s.name = name.unwrap_or(s.name);
                for (k, v, to) in [
                    ("size", size, &mut s.size),
                    ("lineHeight", line_height, &mut s.line_height),
                    ("letterSpacing", letter_spacing, &mut s.letter_spacing),
                    (
                        "paragraphSpacing",
                        paragraph_spacing,
                        &mut s.paragraph_spacing,
                    ),
                ] {
                    if let Some(v) = v {
                        *to = v;
                        s.bindings.remove(k);
                    }
                }
                self.put_text_style(Some(i), s)?;
                vec![]
            }
            Command::DeleteTextStyle { id } => {
                let (i, _) = self.find::<TextStyle>("textStyles", &id)?;
                self.each(|n, _| match self.kind(n).as_str() {
                    "text" => self.detach(n, None, |s| s == id),
                    _ => Ok(()),
                })?;
                self.doc.get_list("textStyles").delete(i, 1).map_err(err)?;
                vec![]
            }
            Command::Set { id, props } => {
                let id = self.node(&id)?;
                let set = serde_json::to_value(&props).map_err(err)?;
                self.unbind(id, |p| !set[p].is_null())?;
                self.set(id, props)?;
                vec![]
            }
            Command::SetPath { id, path } => {
                let id = self.node(&id)?;
                self.set(
                    id,
                    Props {
                        path: Some(fit(&path, [0.0, 0.0, 1.0, 1.0])),
                        ..Props::default()
                    },
                )?;
                self.set_frame(id, bounds(&path).map(f64::from))?;
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
                vec![self.group(&self.sorted(&ids)?, frame)?.to_string()]
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
            Command::Mask { ids } => {
                let ids = self.sorted(&ids)?;
                let lowest = *ids.first().ok_or("nothing to mask")?;
                if ids.len() == 1 {
                    let on = value(&self.meta(lowest), "mask").and_then(|v| v.into_bool().ok());
                    self.meta(lowest)
                        .insert("mask", on != Some(true))
                        .map_err(err)?;
                    vec![lowest.to_string()]
                } else {
                    let g = self.group(&ids, false)?;
                    self.meta(g).insert("name", "Mask group").map_err(err)?;
                    self.meta(lowest).insert("mask", true).map_err(err)?;
                    vec![g.to_string()]
                }
            }
            Command::AutoLayout { ids } => {
                let ids = self.sorted(&ids)?;
                let single = match ids[..] {
                    [f] => self.kind(f) == "frame" && self.layout(f).direction == Direction::None,
                    _ => false,
                };
                let f = if single {
                    ids[0]
                } else {
                    self.group(&ids, true)?
                };
                let mut kids: Vec<_> = self
                    .children(f)
                    .into_iter()
                    .map(|c| (c, self.bounds(c)))
                    .collect();
                let spread = |a: usize| {
                    let centers = kids.iter().map(|(_, b)| b[a] + b[a + 2] / 2.0);
                    centers.clone().fold(f64::MIN, f64::max) - centers.fold(f64::MAX, f64::min)
                };
                let horizontal = spread(0) >= spread(1);
                let a = if horizontal { 0 } else { 1 };
                kids.sort_by(|p, q| p.1[a].total_cmp(&q.1[a]));
                for (i, &(c, _)) in kids.iter().enumerate() {
                    self.tree.mov_to(c, f, i).map_err(err)?;
                }
                let gaps: Vec<f64> = kids
                    .windows(2)
                    .map(|w| w[1].1[a] - w[0].1[a] - w[0].1[a + 2])
                    .collect();
                let gap = (gaps.iter().sum::<f64>() / gaps.len().max(1) as f64).max(0.0);
                let [x, y, w, h] = self.bounds(f);
                let [cx, cy, cw, ch] = union(kids.iter().map(|k| k.1));
                let [top, right, bottom, left] = if single && !kids.is_empty() {
                    [cy - y, x + w - cx - cw, y + h - cy - ch, cx - x].map(|v| v.max(0.0))
                } else {
                    [0.0; 4]
                };
                self.set(
                    f,
                    Props {
                        direction: Some(if horizontal {
                            Direction::Horizontal
                        } else {
                            Direction::Vertical
                        }),
                        gap: Some(gap),
                        padding_top: Some(top),
                        padding_right: Some(right),
                        padding_bottom: Some(bottom),
                        padding_left: Some(left),
                        sizing: Some(Sizing {
                            horizontal: Size::Hug,
                            vertical: Size::Hug,
                        }),
                        ..Props::default()
                    },
                )?;
                vec![f.to_string()]
            }
            Command::Move { ids, parent, index } => {
                let p = self.node(&parent)?;
                if !matches!(self.kind(p).as_str(), "page" | "group" | "frame") {
                    return Err("not a container".into());
                }
                let ids = self.sorted(&ids)?;
                let mut up = Some(p);
                while let Some(n) = up {
                    if ids.contains(&n) {
                        return Err("cannot move a layer into itself".into());
                    }
                    up = self.tree.parent(n).and_then(parent_node);
                }
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
            Command::SetDocument {
                raster_ppi,
                color_mode,
            } => {
                let m = self.doc.get_map("document");
                if let Some(ppi) = raster_ppi {
                    if !(72.0..=1200.0).contains(&ppi) {
                        return Err("raster ppi must be in 72..=1200".into());
                    }
                    m.insert("rasterPpi", ppi).map_err(err)?;
                }
                if let Some(mode) = color_mode {
                    m.insert("colorMode", loro(mode)?).map_err(err)?;
                }
                vec![]
            }
            Command::AddSwatch { name, color, spot } => {
                let id = self.new_id();
                let swatch = Swatch {
                    id: id.clone(),
                    name,
                    color,
                    spot,
                };
                self.check_swatch(&swatch)?;
                self.put("swatches", None, swatch)?;
                vec![id]
            }
            Command::SetSwatch {
                id,
                name,
                color,
                spot,
            } => {
                let (i, mut swatch) = self.swatch(&id)?;
                swatch.name = name.unwrap_or(swatch.name);
                swatch.color = color.unwrap_or(swatch.color);
                swatch.spot = spot.unwrap_or(swatch.spot);
                self.check_swatch(&swatch)?;
                self.put("swatches", Some(i), swatch)?;
                vec![]
            }
            Command::DeleteSwatch { id } => {
                let (i, _) = self.swatch(&id)?;
                let used = |c: &Color| matches!(c, Color::Swatch { swatch, .. } if *swatch == id);
                self.recolor(|c, s| used(c).then(|| c.resolve(s)))?;
                let palette = self.palette();
                let scope = Scope {
                    palette: &palette,
                    modes: &Modes::new(),
                };
                for (j, mut v) in palette.variables.iter().cloned().enumerate() {
                    let mut changed = false;
                    for value in v.values.values_mut() {
                        if let Value::Color(c) = value
                            && used(c)
                        {
                            *c = c.resolve(&scope);
                            changed = true;
                        }
                    }
                    if changed {
                        self.put("variables", Some(j), v)?;
                    }
                }
                self.doc.get_list("swatches").delete(i, 1).map_err(err)?;
                vec![]
            }
            Command::SetCollection { id, name } => {
                let (i, mut c) = self.find::<Collection>("collections", &id)?;
                c.name = name;
                self.put("collections", Some(i), c)?;
                vec![]
            }
            Command::DeleteCollection { id } => {
                let (i, _) = self.find::<Collection>("collections", &id)?;
                for (_, v) in self.variables(&id).collect::<Vec<_>>() {
                    self.delete_variable(&v.id)?;
                }
                self.forget_modes(|c, _| c == id)?;
                self.doc.get_list("collections").delete(i, 1).map_err(err)?;
                vec![]
            }
            Command::SetMode {
                collection,
                id,
                name,
            } => {
                let (i, mut c) = self.find::<Collection>("collections", &collection)?;
                let m = c
                    .modes
                    .iter_mut()
                    .find(|m| m.id == id)
                    .ok_or_else(|| format!("no mode {id}"))?;
                m.name = name;
                self.put("collections", Some(i), c)?;
                vec![]
            }
            Command::DeleteMode { collection, id } => {
                let (i, mut c) = self.find::<Collection>("collections", &collection)?;
                if c.modes.len() == 1 {
                    return Err("a collection keeps one mode".into());
                }
                c.modes.retain(|m| m.id != id);
                self.put("collections", Some(i), c)?;
                for (j, mut v) in self.variables(&collection).collect::<Vec<_>>() {
                    v.values.remove(&id);
                    self.put("variables", Some(j), v)?;
                }
                self.forget_modes(|c, m| c == collection && m == id)?;
                vec![]
            }
            Command::DeleteVariable { id } => {
                self.delete_variable(&id)?;
                vec![]
            }
            Command::AddCollection { name } => {
                let id = self.new_id();
                let mode = format!("{id}.0");
                self.put(
                    "collections",
                    None,
                    Collection {
                        id: id.clone(),
                        name,
                        modes: vec![Mode {
                            id: mode.clone(),
                            name: "Mode 1".into(),
                        }],
                    },
                )?;
                vec![id, mode]
            }
            Command::AddMode { collection, name } => {
                let (i, mut c) = self.find::<Collection>("collections", &collection)?;
                let id = self.new_id();
                let first = c.modes[0].id.clone();
                c.modes.push(Mode {
                    id: id.clone(),
                    name,
                });
                self.put("collections", Some(i), c)?;
                for (i, mut v) in self.variables(&collection) {
                    v.values.insert(id.clone(), v.values[&first].clone());
                    self.put("variables", Some(i), v)?;
                }
                vec![id]
            }
            Command::AddVariable {
                collection,
                name,
                value,
            } => {
                let (_, c) = self.find::<Collection>("collections", &collection)?;
                check_value(&value)?;
                if self.variables(&collection).any(|(_, v)| v.name == name) {
                    return Err(format!("a variable named {name} exists"));
                }
                let id = self.new_id();
                let values = c.modes.into_iter().map(|m| (m.id, value.clone()));
                self.put(
                    "variables",
                    None,
                    Variable {
                        id: id.clone(),
                        collection,
                        name,
                        values: values.collect(),
                    },
                )?;
                vec![id]
            }
            Command::SetVariable {
                id,
                name,
                mode,
                value,
            } => {
                let (i, mut v) = self.find::<Variable>("variables", &id)?;
                if let Some(name) = name {
                    if self
                        .variables(&v.collection)
                        .any(|(_, o)| o.id != id && o.name == name)
                    {
                        return Err(format!("a variable named {name} exists"));
                    }
                    v.name = name;
                }
                if let Some(value) = value {
                    let (_, c) = self.find::<Collection>("collections", &v.collection)?;
                    let mode = mode.unwrap_or_else(|| c.modes[0].id.clone());
                    let old = v
                        .values
                        .get(&mode)
                        .ok_or_else(|| format!("no mode {mode}"))?;
                    check_value(&value)?;
                    if !old.same_kind(&value) {
                        return Err("a variable keeps its type".into());
                    }
                    v.values.insert(mode, value);
                }
                self.put("variables", Some(i), v)?;
                vec![]
            }
            Command::Bind { id, prop, variable } => {
                if let Some(v) = &variable {
                    let (_, var) = self.find::<Variable>("variables", v)?;
                    if !var.values.values().all(|v| matches!(v, Value::Number(_))) {
                        return Err("only number variables bind to numbers".into());
                    }
                }
                if let Ok((i, mut s)) = self.find::<TextStyle>("textStyles", &id) {
                    if !STYLED.contains(&prop.as_str()) {
                        return Err(format!("{prop} cannot be bound"));
                    }
                    match variable {
                        Some(v) => s.bindings.insert(prop, v),
                        None => s.bindings.remove(&prop),
                    };
                    self.put("textStyles", Some(i), s)?;
                    return self.finish(vec![], false);
                }
                let n = self.node(&id)?;
                if !BINDABLE.contains(&prop.as_str()) {
                    return Err(format!("{prop} cannot be bound"));
                }
                if prop == "size" && variable.is_some() {
                    let t = self.text(n)?;
                    self.detach(n, None, |_| true)?;
                    if t.len_utf16() > 0 {
                        t.unmark_utf16(0..t.len_utf16(), "size").map_err(err)?;
                    }
                }
                let mut bindings = self.bindings(n);
                match variable {
                    Some(v) => bindings.insert(prop, v),
                    None => bindings.remove(&prop),
                };
                self.meta(n)
                    .insert("bindings", loro(bindings)?)
                    .map_err(err)?;
                vec![]
            }
            Command::UseMode {
                id,
                collection,
                mode,
            } => {
                let n = self.node(&id)?;
                let (_, c) = self.find::<Collection>("collections", &collection)?;
                let mut modes = self.modes(n);
                match mode {
                    Some(m) if c.modes.iter().any(|x| x.id == m) => modes.insert(collection, m),
                    Some(m) => return Err(format!("no mode {m}")),
                    None => modes.remove(&collection),
                };
                self.meta(n).insert("modes", loro(modes)?).map_err(err)?;
                vec![]
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
        self.finish(out, history)
    }

    fn finish(&self, out: Vec<String>, history: bool) -> Res<Vec<String>> {
        let palette = self.palette();
        for p in self.tree.roots() {
            if !history && self.kind(p) == "page" {
                self.settle(p, &Modes::new(), &palette)?;
                self.lay_out(p)?;
            }
        }
        self.doc.commit();
        Ok(out)
    }

    fn text(&self, id: TreeID) -> Res<LoroText> {
        container(&self.meta(id), "text")
            .and_then(|c| c.into_text().ok())
            .ok_or_else(|| "not a text node".into())
    }

    /// Replaces the text styles for which `only` holds in `range` of `id`, or in all
    /// of it, by the values they stand for.
    fn detach(
        &self,
        id: TreeID,
        range: Option<Range<usize>>,
        only: impl Fn(&str) -> bool,
    ) -> Res<()> {
        let t = self.text(id)?;
        let m = self.meta(id);
        let palette = self.palette();
        let modes = self.active_modes(id);
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let find = |id: &str| palette.text_styles.iter().find(|t| t.id == id && only(id));
        let own = value(&m, "textStyle")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if range.is_none()
            && let Some(st) = find(&own)
        {
            for (k, v) in st.values(&s) {
                m.insert(&k, loro(v)?).map_err(err)?;
            }
            m.insert("textStyle", "").map_err(err)?;
        }
        let r = range.clone().unwrap_or(0..t.len_utf16());
        let mut at = 0;
        for d in t.to_delta() {
            let TextDelta::Insert { insert, attributes } = d else {
                continue;
            };
            let (a, b) = (at, at + insert.encode_utf16().count());
            at = b;
            let (lo, hi) = (a.max(r.start), b.min(r.end));
            if lo >= hi {
                continue;
            }
            let marks = attributes.unwrap_or_default();
            let mine = match marks.get("textStyle") {
                Some(LoroValue::String(s)) => s.to_string(),
                _ if range.is_some() => own.clone(),
                _ => continue,
            };
            let Some(st) = find(&mine) else { continue };
            for (k, v) in st.values(&s) {
                if !marks.contains_key(&k) {
                    t.mark_utf16(lo..hi, &k, loro(v)?).map_err(err)?;
                }
            }
            t.mark_utf16(lo..hi, "textStyle", "").map_err(err)?;
        }
        Ok(())
    }

    fn put_text_style(&self, i: Option<usize>, s: TextStyle) -> Res<()> {
        check_text(
            Some(s.size),
            Some(s.line_height),
            Some(s.letter_spacing),
            Some(s.paragraph_spacing),
        )?;
        let taken = self
            .list::<TextStyle>("textStyles")
            .iter()
            .any(|o| o.id != s.id && o.name == s.name);
        if taken {
            return Err(format!("a text style named {} exists", s.name));
        }
        self.put("textStyles", i, s)
    }

    /// The modes `id` resolves variables in, its own and inherited.
    fn active_modes(&self, id: TreeID) -> Modes {
        let mut chain = vec![id];
        while let Some(p) = self
            .tree
            .parent(*chain.last().unwrap())
            .and_then(parent_node)
        {
            chain.push(p);
        }
        let mut modes = Modes::new();
        for n in chain.into_iter().rev() {
            modes.extend(self.modes(n));
        }
        modes
    }

    /// Writes the values of bound variables into `id` and its subtree.
    fn settle(&self, id: TreeID, inherited: &Modes, palette: &Palette) -> Res<()> {
        let mut modes = inherited.clone();
        modes.extend(self.modes(id));
        let scope = Scope {
            palette,
            modes: &modes,
        };
        let m = self.meta(id);
        let old = self.bounds(id);
        let mut frame = old;
        for (prop, var) in self.bindings(id) {
            let Some(&Value::Number(v)) = scope.value(&var) else {
                continue;
            };
            let v = match prop.as_str() {
                "opacity" => (v / 100.0).clamp(0.0, 1.0),
                "size" => v.max(0.1),
                _ => v.max(0.0) * MM,
            };
            match prop.as_str() {
                "w" => frame[2] = v,
                "h" => frame[3] = v,
                _ if num(&m, &prop) != v => {
                    m.insert(&prop, v).map_err(err)?;
                }
                _ => {}
            }
        }
        if frame != old {
            self.set_frame(id, frame)?;
        }
        for c in self.children(id) {
            self.settle(c, &modes, palette)?;
        }
        Ok(())
    }

    /// Calls `f` for every node, including deleted ones, with its active modes.
    fn each(&self, mut f: impl FnMut(TreeID, &Modes) -> Res<()>) -> Res<()> {
        let mut stack: Vec<_> = self
            .tree
            .roots()
            .into_iter()
            .map(|r| (r, Modes::new()))
            .collect();
        while let Some((id, mut modes)) = stack.pop() {
            modes.extend(self.modes(id));
            f(id, &modes)?;
            stack.extend(self.children(id).into_iter().map(|c| (c, modes.clone())));
        }
        Ok(())
    }

    /// Replaces the colours of every node that `f` maps to another colour.
    fn recolor(&self, f: impl Fn(&Color, &Scope) -> Option<Color>) -> Res<()> {
        let palette = self.palette();
        self.each(|id, modes| {
            let s = Scope {
                palette: &palette,
                modes,
            };
            let m = self.meta(id);
            for key in ["fills", "strokes", "effects"] {
                let Some(v) = value(&m, key) else { continue };
                let mut v = serde_json::to_value(v).map_err(err)?;
                if map_colors(&mut v, &|c| f(c, &s)) {
                    m.insert(key, loro(v)?).map_err(err)?;
                }
            }
            Ok(())
        })
    }

    fn delete_variable(&self, id: &str) -> Res<()> {
        let (i, _) = self.find::<Variable>("variables", id)?;
        self.recolor(|c, s| {
            matches!(c, Color::Variable { variable, .. } if variable == id).then(|| c.bind(s))
        })?;
        self.each(|n, _| {
            let bindings = self.bindings(n);
            self.unbind(n, |p| bindings[p] == id)
        })?;
        let palette = self.palette();
        let first = Scope {
            palette: &palette,
            modes: &Modes::new(),
        };
        for (j, s) in palette.text_styles.iter().enumerate() {
            if !s.bindings.values().any(|v| v == id) {
                continue;
            }
            let mut v = serde_json::to_value(s).map_err(err)?;
            v.as_object_mut().unwrap().extend(s.values(&first));
            let mut s: TextStyle = serde_json::from_value(v).map_err(err)?;
            s.bindings.retain(|_, v| v != id);
            self.put("textStyles", Some(j), s)?;
        }
        self.doc.get_list("variables").delete(i, 1).map_err(err)
    }

    /// Drops the modes chosen on any node for which `f(collection, mode)` holds.
    fn forget_modes(&self, f: impl Fn(&str, &str) -> bool) -> Res<()> {
        self.each(|n, _| {
            let mut modes = self.modes(n);
            let len = modes.len();
            modes.retain(|c, m| !f(c, m));
            if modes.len() < len {
                self.meta(n).insert("modes", loro(modes)?).map_err(err)?;
            }
            Ok(())
        })
    }

    fn layout(&self, id: TreeID) -> Layout {
        serde_json::to_value(self.meta(id).get_deep_value())
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    /// Lays out the auto layout frames in `id`'s subtree, innermost first.
    fn lay_out(&self, id: TreeID) -> Res<()> {
        let kids = self.children(id);
        for &c in &kids {
            self.lay_out(c)?;
        }
        let l = self.layout(id);
        let Some((horizontal, pad)) = l.axes().filter(|_| self.kind(id) == "frame") else {
            return Ok(());
        };
        let flow: Vec<TreeID> = kids
            .into_iter()
            .filter(|&c| !self.layout(c).absolute)
            .collect();
        let axes = |[x, y, w, h]: [f64; 4]| {
            if horizontal {
                [[x, w], [y, h]]
            } else {
                [[y, h], [x, w]]
            }
        };
        let unaxes = |[[m0, ms], [c0, cs]]: [[f64; 2]; 2]| {
            if horizontal {
                [m0, c0, ms, cs]
            } else {
                [c0, m0, cs, ms]
            }
        };
        let old = self.bounds(id);
        let [[m0, ms], [c0, cs]] = axes(old);
        let fill = |c: TreeID| {
            let cl = self.layout(c);
            [horizontal, !horizontal].map(|a| cl.size(a) == Size::Fill)
        };
        let children: Vec<_> = flow
            .iter()
            .map(|&c| {
                let [[_, a], [_, b]] = axes(self.bounds(c));
                ([a, b], fill(c))
            })
            .collect();
        let hug = [horizontal, !horizontal].map(|a| l.size(a) == Size::Hug);
        let (size, boxes) = arrange(&l, pad, [ms, cs], hug, &children);
        let frame = unaxes([[m0, size[0]], [c0, size[1]]]);
        if frame != old {
            self.set_frame(id, frame)?;
        }
        for (c, [[a0, a], [b0, b]]) in flow.into_iter().zip(boxes) {
            let old = self.bounds(c);
            let new = unaxes([[m0 + a0, a], [c0 + b0, b]]);
            if new != old {
                self.set_frame(c, new)?;
                if new[2..] != old[2..] {
                    self.lay_out(c)?;
                }
            }
        }
        Ok(())
    }

    fn constraints(&self, id: TreeID) -> Constraints {
        value(&self.meta(id), "constraints")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    fn bindings(&self, id: TreeID) -> BTreeMap<String, String> {
        value(&self.meta(id), "bindings")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    fn unbind(&self, id: TreeID, drop: impl Fn(&str) -> bool) -> Res<()> {
        let mut bindings = self.bindings(id);
        let n = bindings.len();
        bindings.retain(|p, _| !drop(p));
        if bindings.len() < n {
            self.meta(id)
                .insert("bindings", loro(bindings)?)
                .map_err(err)?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        let palette = self.palette();
        let pages = self
            .tree
            .roots()
            .into_iter()
            .filter(|&p| self.kind(p) == "page")
            .map(|p| {
                let m = self.meta(p);
                let modes = self.modes(p);
                Page {
                    id: p.to_string(),
                    width: num(&m, "width"),
                    height: num(&m, "height"),
                    bleed: num(&m, "bleed"),
                    children: self
                        .children(p)
                        .into_iter()
                        .map(|c| self.snap(c, &modes, &palette))
                        .collect(),
                    modes,
                }
            })
            .collect();
        Snapshot {
            pages,
            raster_ppi: num(&self.doc.get_map("document"), "rasterPpi"),
            color_mode: self.color_mode(),
            palette,
            can_undo: self.undo.can_undo(),
            can_redo: self.undo.can_redo(),
        }
    }

    fn swatches(&self) -> Vec<Swatch> {
        self.list("swatches")
    }

    fn palette(&self) -> Palette {
        Palette {
            swatches: self.swatches(),
            collections: self.list("collections"),
            variables: self.list("variables"),
            text_styles: self.list("textStyles"),
        }
    }

    fn variables(&self, collection: &str) -> impl Iterator<Item = (usize, Variable)> {
        let all: Vec<Variable> = self.list("variables");
        let collection = collection.to_string();
        all.into_iter()
            .enumerate()
            .filter(move |(_, v)| v.collection == collection)
    }

    fn modes(&self, id: TreeID) -> Modes {
        value(&self.meta(id), "modes")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    fn new_id(&self) -> String {
        format!("{}@{}", self.doc.len_ops(), self.doc.peer_id())
    }

    /// The entries of the list `name`, each stored as a map.
    fn list<T: DeserializeOwned>(&self, name: &str) -> Vec<T> {
        serde_json::to_value(self.doc.get_list(name).get_deep_value())
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    fn find<T: DeserializeOwned>(&self, name: &str, id: &str) -> Res<(usize, T)> {
        let all: Vec<serde_json::Value> = self.list(name);
        let i = all
            .iter()
            .position(|v| v["id"] == id)
            .ok_or_else(|| format!("no {} {id}", name.trim_end_matches('s')))?;
        Ok((i, serde_json::from_value(all[i].clone()).map_err(err)?))
    }

    /// Writes `v` over entry `i` of the list `name`, or appends it.
    fn put(&self, name: &str, i: Option<usize>, v: impl Serialize) -> Res<()> {
        let list = self.doc.get_list(name);
        let m = match i {
            Some(i) => list
                .get(i)
                .and_then(|v| v.into_container().ok())
                .and_then(|c| c.into_map().ok())
                .ok_or("no entry")?,
            None => list
                .insert_container(list.len(), LoroMap::new())
                .map_err(err)?,
        };
        for (k, v) in loro(v)?.into_map().unwrap().iter() {
            m.insert(k, v.clone()).map_err(err)?;
        }
        Ok(())
    }

    fn check_swatch(&self, swatch: &Swatch) -> Res<()> {
        swatch.check()?;
        if self
            .swatches()
            .iter()
            .any(|s| s.id != swatch.id && s.name == swatch.name)
        {
            return Err(format!("a swatch named {} exists", swatch.name));
        }
        Ok(())
    }

    fn swatch(&self, id: &str) -> Res<(usize, Swatch)> {
        self.find("swatches", id)
    }

    fn color_mode(&self) -> ColorMode {
        value(&self.doc.get_map("document"), "colorMode")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
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
        draw_all(&p.children, &mut ops, &snap.palette);
        ops
    }

    pub fn hit(&self, page: usize, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        let mut path = Vec::new();
        if let Some(p) = self.snapshot().pages.get(page) {
            hit(&p.children, x, y, tolerance, &mut path);
        }
        path
    }

    fn snap(&self, id: TreeID, inherited: &Modes, palette: &Palette) -> Node {
        let m = self.meta(id);
        let v = serde_json::to_value(m.get_deep_value()).unwrap_or_default();
        let modes = self.modes(id);
        let mut active_modes = inherited.clone();
        active_modes.extend(modes.clone());
        let children = || {
            self.children(id)
                .into_iter()
                .map(|c| self.snap(c, &active_modes, palette))
                .collect()
        };
        let kind = match self.kind(id).as_str() {
            "text" => Kind::Text {
                text: v["text"].as_str().unwrap_or_default().into(),
                spans: self.spans(
                    id,
                    &v,
                    &Scope {
                        palette,
                        modes: &active_modes,
                    },
                ),
            },
            "group" => Kind::Group {
                children: children(),
            },
            "frame" => Kind::Frame {
                clip: v["clip"] == true,
                children: children(),
            },
            _ => Kind::Shape(
                serde_json::from_value(v.clone()).unwrap_or(Shape::Rect { radius: 0.0 }),
            ),
        };
        let style: Style = serde_json::from_value(v.clone()).unwrap_or_default();
        let name = v["name"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| match &kind {
                Kind::Shape(Shape::Rect { .. }) => "Rectangle".into(),
                Kind::Shape(Shape::Ellipse) => "Ellipse".into(),
                Kind::Shape(Shape::Polygon { .. }) => "Polygon".into(),
                Kind::Shape(Shape::Star { .. }) => "Star".into(),
                Kind::Shape(Shape::Path { .. }) if style.arrow_start || style.arrow_end => {
                    "Arrow".into()
                }
                Kind::Shape(Shape::Path { path }) if path.len() == 6 => "Line".into(),
                Kind::Shape(Shape::Path { .. }) => "Vector".into(),
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
            modes,
            active_modes,
            bindings: self.bindings(id),
            layout: serde_json::from_value(v.clone()).unwrap_or_default(),
            style,
            kind,
        }
    }

    /// The text of `id` in runs of equal attributes: the layer's, under those of its
    /// text style, under the marks; paragraph attributes from each paragraph's start.
    fn spans(&self, id: TreeID, node: &serde_json::Value, s: &Scope) -> Vec<Span> {
        let resolve = |marks: serde_json::Map<String, serde_json::Value>| -> Attrs {
            let mut m = node.as_object().cloned().unwrap_or_default();
            let style = marks.get("textStyle").or(m.get("textStyle"));
            let style = style.and_then(|v| v.as_str()).unwrap_or_default();
            if let Some(st) = s.palette.text_styles.iter().find(|t| t.id == style) {
                m.extend(st.values(s));
            }
            m.extend(marks);
            serde_json::from_value(serde_json::Value::Object(m)).unwrap_or_default()
        };
        let mut out: Vec<Span> = Vec::new();
        let mut para: Option<Attrs> = None;
        for d in self.text(id).map(|t| t.to_delta()).unwrap_or_default() {
            let TextDelta::Insert { insert, attributes } = d else {
                continue;
            };
            let marks = serde_json::to_value(attributes.unwrap_or_default())
                .ok()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            let attrs = resolve(marks);
            for piece in insert.split_inclusive('\n') {
                let p = para.get_or_insert_with(|| attrs.clone());
                let a = Attrs {
                    text_align: p.text_align,
                    paragraph_spacing: p.paragraph_spacing,
                    ..attrs.clone()
                };
                let len = piece.encode_utf16().count();
                match out.last_mut() {
                    Some(l) if l.attrs == a => l.len += len,
                    _ => out.push(Span { len, attrs: a }),
                }
                if piece.ends_with('\n') {
                    para = None;
                }
            }
        }
        if out.is_empty() {
            out.push(Span {
                len: 0,
                attrs: resolve(Default::default()),
            });
        }
        out
    }

    fn set(&self, id: TreeID, props: Props) -> Res<()> {
        props.check()?;
        let m = self.meta(id);
        let serde_json::Value::Object(props) = serde_json::to_value(props).map_err(err)? else {
            return Err("props are not a map".into());
        };
        for (k, v) in props.into_iter().filter(|(_, v)| !v.is_null()) {
            m.insert(&k, loro(v)?).map_err(err)?;
        }
        Ok(())
    }

    fn set_frame(&self, id: TreeID, frame: [f64; 4]) -> Res<()> {
        self.resize(id, frame, true)
    }

    fn resize(&self, id: TreeID, [x, y, w, h]: [f64; 4], follow: bool) -> Res<()> {
        if !(w >= 0.0 && h >= 0.0) {
            return Err("width and height must not be negative".into());
        }
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
            "frame" if follow => {
                for c in self.children(id) {
                    let [cx, cy, cw, ch] = self.bounds(c);
                    let k = self.constraints(c);
                    let [nx, nw] = constrain(k.horizontal, [cx, cw], [ox, ow], [x, w]);
                    let [ny, nh] = constrain(k.vertical, [cy, ch], [oy, oh], [y, h]);
                    self.set_frame(c, [nx, ny, nw, nh])?;
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

    /// Wraps `ids`, in document order, in a group or frame at the place of the topmost.
    fn group(&self, ids: &[TreeID], frame: bool) -> Res<TreeID> {
        let top = *ids.last().ok_or("nothing to group")?;
        let parent = self.tree.parent(top).ok_or("no parent")?;
        let index = self.index(top) + 1;
        let bounds = union(ids.iter().map(|&id| self.bounds(id)));
        let g = self.tree.create_at(parent, index).map_err(err)?;
        let m = self.meta(g);
        if frame {
            m.insert("kind", "frame").map_err(err)?;
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
        Ok(g)
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
            text: self.text(id).map(|t| t.to_delta()).unwrap_or_default(),
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
                ("text", LoroValue::String(_)) => to
                    .insert_container(k, LoroText::new())
                    .map_err(err)?
                    .apply_delta(&clip.text)
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

/// Draws siblings; a mask masks the siblings above it.
fn draw_all(nodes: &[Node], ops: &mut Vec<Op>, pal: &Palette) {
    for (i, n) in nodes.iter().enumerate() {
        if n.style.mask {
            ops.push(Op::BeginMask);
            draw(n, ops, pal);
            ops.push(Op::EndMask);
            draw_all(&nodes[i + 1..], ops, pal);
            ops.push(Op::PopMask);
            return;
        }
        draw(n, ops, pal);
    }
}

fn draw(n: &Node, ops: &mut Vec<Op>, pal: &Palette) {
    let frame = [n.x, n.y, n.w, n.h].map(|v| v as f32);
    let item = |ops: &mut Vec<Op>, body: Vec<Op>| {
        if body.is_empty() {
            return;
        }
        let key = n.id.bytes().fold(0x811c9dc5u32, |h, b| {
            (h ^ b as u32).wrapping_mul(0x01000193)
        });
        ops.push(Op::BeginItem { item: key });
        ops.extend(body);
        ops.push(Op::EndItem);
    };
    let s = &Scope {
        palette: pal,
        modes: &n.active_modes,
    };
    let layer = n.style.layer(s);
    let wrapped = layer.is_some();
    ops.extend(layer);
    match &n.kind {
        Kind::Shape(shape) => item(ops, n.style.shape(&outline(shape, frame), frame, s)),
        Kind::Text { text, spans } => item(ops, text::draw(text, spans, &n.style.fills, frame, s)),
        Kind::Group { children } => draw_all(children, ops, pal),
        Kind::Frame { clip, children } => {
            let r = rect(frame[0], frame[1], frame[2], frame[3]);
            item(ops, n.style.shape(&r, frame, s));
            if !*clip {
                draw_all(children, ops, pal);
            } else if n.w > 0.0 && n.h > 0.0 {
                ops.push(Op::PushClip {
                    path: r,
                    invert: false,
                });
                draw_all(children, ops, pal);
                ops.push(Op::PopClip);
            }
        }
    }
    if wrapped {
        ops.push(Op::PopLayer);
    }
}

/// Like `draw_all`, a node is only hit inside every mask below it among its siblings.
fn hit(nodes: &[Node], x: f64, y: f64, tolerance: f64, path: &mut Vec<String>) -> bool {
    for (i, n) in nodes.iter().enumerate().rev() {
        let masks = nodes[..i].iter().filter(|m| m.style.mask);
        if masks
            .into_iter()
            .any(|m| !hit(std::slice::from_ref(m), x, y, tolerance, &mut Vec::new()))
        {
            continue;
        }
        let inside = x >= n.x && x <= n.x + n.w && y >= n.y && y <= n.y + n.h;
        path.push(n.id.clone());
        let found = match &n.kind {
            Kind::Group { children } => hit(children, x, y, tolerance, path),
            Kind::Frame { children, clip } => {
                (inside || !clip) && hit(children, x, y, tolerance, path) || inside
            }
            Kind::Shape(s) => {
                let p = outline(s, [n.x, n.y, n.w, n.h].map(|v| v as f32));
                let (x, y) = (x as f32, y as f32);
                let stroke = if n.style.strokes.iter().any(|f| f.visible) {
                    n.style.stroke_weight / 2.0
                } else {
                    0.0
                };
                p.contains(&CLOSE) && contains(&p, x, y)
                    || near(&p, x, y, tolerance as f32 + stroke)
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

/// Start and size on one axis of a child at `c` in a parent that moves from `p` to `n`.
fn constrain(
    k: Constraint,
    [c0, cs]: [f64; 2],
    [p0, ps]: [f64; 2],
    [n0, ns]: [f64; 2],
) -> [f64; 2] {
    match k {
        Constraint::Min => [n0 + c0 - p0, cs],
        Constraint::Max => [n0 + ns - (p0 + ps - c0), cs],
        Constraint::Stretch => [n0 + c0 - p0, (cs + ns - ps).max(0.0)],
        Constraint::Center => [n0 + ns / 2.0 - (p0 + ps / 2.0 - c0), cs],
        Constraint::Scale if ps > 0.0 => [n0 + (c0 - p0) * ns / ps, cs * ns / ps],
        Constraint::Scale => [n0 + c0 - p0, cs],
    }
}

/// The UTF-16 range of the paragraphs that `r` touches in `text`.
fn paragraphs(text: &str, r: &Range<usize>) -> Range<usize> {
    let breaks: Vec<usize> = text
        .encode_utf16()
        .enumerate()
        .filter(|&(_, c)| c == '\n' as u16)
        .map(|(i, _)| i + 1)
        .collect();
    let last = if r.end > r.start { r.end - 1 } else { r.end };
    let start = breaks
        .iter()
        .rev()
        .find(|&&b| b <= r.start)
        .copied()
        .unwrap_or(0);
    let end = breaks
        .iter()
        .find(|&&b| b > last)
        .copied()
        .unwrap_or(text.encode_utf16().count());
    start..end
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

fn loro(v: impl Serialize) -> Res<LoroValue> {
    serde_json::from_value(serde_json::to_value(v).map_err(err)?).map_err(err)
}

/// Replaces the colours in `v` that `f` maps to another colour.
fn map_colors(v: &mut serde_json::Value, f: &impl Fn(&Color) -> Option<Color>) -> bool {
    match v {
        serde_json::Value::Object(o)
            if (o.contains_key("swatch") || o.contains_key("variable"))
                && let Ok(c) = serde_json::from_value::<Color>(v.clone())
                && let Some(c) = f(&c) =>
        {
            *v = serde_json::to_value(c).unwrap();
            true
        }
        serde_json::Value::Object(o) => o.values_mut().fold(false, |d, v| map_colors(v, f) | d),
        serde_json::Value::Array(a) => a.iter_mut().fold(false, |d, v| map_colors(v, f) | d),
        _ => false,
    }
}

fn check_value(v: &Value) -> Res<()> {
    match v {
        Value::Color(Color::Variable { .. }) => Err("a variable cannot hold a variable".into()),
        Value::Color(c) => c.check(),
        Value::Number(n) if !n.is_finite() => Err("numbers must be finite".into()),
        Value::Number(_) => Ok(()),
    }
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
    use crate::display_list::Paint;

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
        assert_eq!(p.children[0].kind, Kind::Shape(Shape::Rect { radius: 0.0 }));
        assert_eq!(p.children[0].style.fills, [Fill::solid(0xe8452cff)]);
        assert!(
            matches!(&p.children[1].kind, Kind::Text { spans, .. } if spans[0].attrs.size == 14.0)
        );
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
        assert_eq!(n.kind, Kind::Shape(Shape::Rect { radius: 0.0 }));
        assert_eq!(n.style.fills, [Fill::solid(0xd9d9d9ff)]);
        assert_eq!(n.style.stroke_align, Align::Inside);
        let f = &pg.children[1];
        assert!(matches!(f.kind, Kind::Frame { clip: true, .. }));
        assert_eq!(f.style.fills, [Fill::solid(WHITE)]);
        assert_eq!(children(f)[0].id, t);
        assert_eq!(children(f)[0].name, "Text");
    }

    #[test]
    fn set_changes_properties_and_unknown_nodes_are_errors() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        d.apply(Command::Set {
            id: a.clone(),
            props: Props {
                name: Some("Bg".into()),
                fills: Some(vec![Fill::solid(0xff0000ff)]),
                opacity: Some(0.5),
                blend: Some(Blend::Multiply),
                ..Props::default()
            },
        })
        .unwrap();
        let n = &page(&d).children[0];
        assert_eq!(n.name, "Bg");
        assert_eq!(n.style.fills, [Fill::solid(0xff0000ff)]);
        assert_eq!((n.style.opacity, n.style.blend), (0.5, Blend::Multiply));
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
            ignore_constraints: false,
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
            ignore_constraints: false,
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
    fn a_move_into_a_moved_subtree_changes_nothing() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let f = create(&mut d, &p, NewKind::Frame, [0.0; 4]);
        let g = d
            .apply(Command::Group {
                ids: vec![f.clone()],
                frame: false,
            })
            .unwrap()
            .remove(0);
        let before = d.snapshot();
        let cyclic = Command::Move {
            ids: vec![a, g],
            parent: f,
            index: 0,
        };
        assert!(d.apply(cyclic).is_err());
        assert_eq!(d.snapshot(), before);
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
        assert!(matches!(&page(&d).children[1].kind, Kind::Text { text, .. } if text == "Hallo"));
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
        assert_eq!(d.hit(0, 7.0, 7.0, 0.0), [b]);
        assert_eq!(d.hit(0, 2.0, 2.0, 0.0), [a]);
        assert!(d.hit(0, 20.0, 2.0, 0.0).is_empty());
        assert!(d.hit(1, 2.0, 2.0, 0.0).is_empty());
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
        assert_eq!(d.hit(0, 5.0, 5.0, 0.0), [f.clone(), g.clone(), a]);
        assert_eq!(d.hit(0, 30.0, 30.0, 0.0), [f]);
    }

    #[test]
    fn clipped_content_is_only_hit_inside_its_frame() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let a = create(&mut d, &f, NewKind::Rect, [5.0, 5.0, 20.0, 20.0]);
        assert!(d.hit(0, 15.0, 15.0, 0.0).is_empty());
        assert_eq!(d.hit(0, 8.0, 8.0, 0.0), [f.clone(), a.clone()]);
        d.apply(Command::Set {
            id: f.clone(),
            props: Props {
                clip: Some(false),
                ..Props::default()
            },
        })
        .unwrap();
        assert_eq!(d.hit(0, 15.0, 15.0, 0.0), [f, a]);
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
                ignore_constraints: false,
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
        assert_eq!(ops.iter().filter(|o| **o == Op::EndItem).count(), 11);
        assert!(ops.iter().any(|o| matches!(o, Op::GlyphRun { .. })));
        let at = |f: fn(&Op) -> bool| ops.iter().position(f).unwrap();
        assert!(
            at(|o| matches!(o, Op::PushClip { invert: false, .. })) < at(|o| *o == Op::PopClip)
        );
        assert!(at(|o| *o == Op::BeginMask) < at(|o| *o == Op::PopMask));
        assert!(
            ops.iter()
                .any(|o| matches!(o, Op::PushLayer { blur, .. } if *blur > 0.0))
        );
        assert!(Doc::new().render(9).is_empty());
    }

    fn set(d: &mut Doc, id: &str, props: Props) {
        d.apply(Command::Set {
            id: id.into(),
            props,
        })
        .unwrap();
    }

    #[test]
    fn shapes_are_hit_on_their_outline_not_their_box() {
        let (mut d, p) = empty();
        let e = create(&mut d, &p, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(d.hit(0, 5.0, 5.0, 0.0), [e]);
        assert!(d.hit(0, 0.5, 0.5, 0.0).is_empty());

        let l = create(&mut d, &p, NewKind::Line, [0.0; 4]);
        d.apply(Command::SetPath {
            id: l.clone(),
            path: vec![MOVE, 20.0, 10.0, LINE, 10.0, 0.0],
        })
        .unwrap();
        let n = &page(&d).children[1];
        assert_eq!(
            (frame(n), n.name.as_str()),
            ([10.0, 0.0, 10.0, 10.0], "Line")
        );
        assert_eq!(d.hit(0, 15.0, 6.0, 0.5), [l]);
        assert!(d.hit(0, 12.0, 8.0, 0.5).is_empty());
        assert!(d.hit(0, 15.0, 6.0, 0.0).is_empty());
    }

    #[test]
    fn inside_strokes_are_clipped_and_twice_as_wide() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![]),
                strokes: Some(vec![Fill::solid(BLACK)]),
                stroke_weight: Some(2.0),
                ..Props::default()
            },
        );
        let ops = d.render(0);
        assert!(matches!(ops[2], Op::PushClip { invert: false, .. }));
        assert!(matches!(ops[3], Op::StrokePath { width: 4.0, .. }));
        assert_eq!(ops[4], Op::PopClip);
        set(
            &mut d,
            &r,
            Props {
                stroke_align: Some(Align::Outside),
                ..Props::default()
            },
        );
        assert!(matches!(d.render(0)[2], Op::PushClip { invert: true, .. }));
    }

    #[test]
    fn arrows_add_a_head_to_the_stroke() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Arrow, [0.0, 0.0, 10.0, 0.0]);
        assert_eq!(page(&d).children[0].name, "Arrow");
        let Op::StrokePath { path, .. } = &d.render(0)[2] else {
            panic!("no stroke");
        };
        assert_eq!(path[..6], [MOVE, 0.0, 0.0, LINE, 10.0, 0.0]);
        assert_eq!(
            path[6..],
            [MOVE, 5.0, -5.0, LINE, 10.0, 0.0, LINE, 5.0, 5.0]
        );
    }

    #[test]
    fn opacity_blend_and_effects_wrap_the_node_in_a_layer() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        assert!(
            !d.render(0)
                .iter()
                .any(|o| matches!(o, Op::PushLayer { .. }))
        );
        set(
            &mut d,
            &r,
            Props {
                opacity: Some(0.5),
                effects: Some(vec![
                    Effect::default(),
                    Effect {
                        kind: EffectKind::Blur,
                        radius: 4.0,
                        ..Effect::default()
                    },
                    Effect {
                        visible: false,
                        ..Effect::default()
                    },
                ]),
                ..Props::default()
            },
        );
        let ops = d.render(0);
        let Op::PushLayer {
            opacity,
            blur,
            shadows,
            ..
        } = &ops[1]
        else {
            panic!("no layer");
        };
        assert_eq!((*opacity, *blur, shadows.len()), (0.5, 2.0, 1));
        assert_eq!(shadows[0].offset, [0.0, 3.0]);
        assert_eq!(shadows[0].blur, 3.0);
        assert_eq!(ops.last(), Some(&Op::PopLayer));
    }

    #[test]
    fn a_mask_masks_the_siblings_above_it() {
        let (mut d, p) = empty();
        let [a, m, b, c] = [0; 4].map(|_| create(&mut d, &p, NewKind::Ellipse, [0.0; 4]));
        set(
            &mut d,
            &m,
            Props {
                mask: Some(true),
                ..Props::default()
            },
        );
        let ops = d.render(0);
        let kinds: Vec<_> = ops[1..]
            .iter()
            .filter(|o| !matches!(o, Op::FillPath { .. } | Op::EndItem))
            .collect();
        let key = |id: &String| {
            id.bytes().fold(0x811c9dc5u32, |h, b| {
                (h ^ b as u32).wrapping_mul(0x01000193)
            })
        };
        let item = |id| Op::BeginItem { item: key(id) };
        assert_eq!(
            kinds,
            [
                &item(&a),
                &Op::BeginMask,
                &item(&m),
                &Op::EndMask,
                &item(&b),
                &item(&c),
                &Op::PopMask
            ]
        );
    }

    #[test]
    fn masking_several_layers_wraps_them_in_a_mask_group_over_the_lowest() {
        let (mut d, p) = empty();
        let [a, b, c] = [0; 3].map(|_| create(&mut d, &p, NewKind::Rect, [0.0; 4]));
        let g = d
            .apply(Command::Mask {
                ids: vec![c.clone(), b.clone()],
            })
            .unwrap()
            .remove(0);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [a.clone(), g.clone()]);
        assert_eq!(pg.children[1].name, "Mask group");
        let kids = children(&pg.children[1]);
        assert_eq!(ids(kids), [b.clone(), c]);
        assert_eq!((kids[0].style.mask, kids[1].style.mask), (true, false));

        assert_eq!(
            d.apply(Command::Mask {
                ids: vec![a.clone()]
            })
            .unwrap(),
            std::slice::from_ref(&a)
        );
        assert!(page(&d).children[0].style.mask);
        d.apply(Command::Mask { ids: vec![a] }).unwrap();
        assert!(!page(&d).children[0].style.mask);
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(page(&d).children.len(), 3);
    }

    #[test]
    fn masked_layers_are_hit_only_inside_the_mask() {
        let (mut d, p) = empty();
        let below = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let m = create(&mut d, &p, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        let above = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        d.apply(Command::Mask { ids: vec![m] }).unwrap();
        assert_eq!(d.hit(0, 5.0, 5.0, 0.0), [above]);
        assert_eq!(d.hit(0, 0.5, 0.5, 0.0), [below]);
    }

    #[test]
    fn set_and_set_frame_reject_values_out_of_range() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Star, [0.0; 4]);
        let t = create(&mut d, &p, NewKind::Text, [0.0; 4]);
        let set = |d: &mut Doc, id: &String, props| {
            d.apply(Command::Set {
                id: id.clone(),
                props,
            })
        };
        let bad = [
            Props {
                stroke_weight: Some(-0.1),
                ..Props::default()
            },
            Props {
                radius: Some(-1.0),
                ..Props::default()
            },
            Props {
                count: Some(2),
                ..Props::default()
            },
            Props {
                count: Some(61),
                ..Props::default()
            },
            Props {
                ratio: Some(0.0),
                ..Props::default()
            },
            Props {
                ratio: Some(1.01),
                ..Props::default()
            },
            Props {
                opacity: Some(1.5),
                ..Props::default()
            },
            Props {
                opacity: Some(f32::NAN),
                ..Props::default()
            },
            Props {
                effects: Some(vec![Effect {
                    radius: -1.0,
                    ..Effect::default()
                }]),
                ..Props::default()
            },
        ];
        for props in bad {
            let what = format!("{props:?}");
            assert!(set(&mut d, &r, props).is_err(), "{what}");
        }
        assert!(format(&mut d, &t, None, sized(0.09)).is_err());
        format(&mut d, &t, None, sized(0.1)).unwrap();
        let set_frame = |d: &mut Doc, w, h| {
            d.apply(Command::SetFrame {
                id: r.clone(),
                x: -5.0,
                y: 0.0,
                w,
                h,
                ignore_constraints: false,
            })
        };
        assert!(set_frame(&mut d, -1.0, 1.0).is_err());
        assert!(set_frame(&mut d, 1.0, -1.0).is_err());
        set_frame(&mut d, 10.0, 0.0).unwrap();
        assert_eq!(frame(&page(&d).children[0]), [-5.0, 0.0, 10.0, 0.0]);
    }

    #[test]
    fn gradients_map_the_unit_box_into_the_frame() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [10.0, 20.0, 100.0, 50.0]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill {
                    kind: FillKind::Linear,
                    transform: [0.0, 1.0, -1.0, 0.0, 0.5, 0.0],
                    stops: vec![FillStop {
                        at: 0.0,
                        color: WHITE.into(),
                    }],
                    ..Fill::default()
                }]),
                ..Props::default()
            },
        );
        let Op::FillPath {
            paint: Paint::Linear { transform, stops },
            ..
        } = &d.render(0)[2]
        else {
            panic!("no gradient");
        };
        assert_eq!(*transform, [0.0, 50.0, -100.0, 0.0, 60.0, 20.0]);
        assert_eq!(stops[0].color, [1.0; 4]);
    }

    #[test]
    fn the_document_rasterizes_at_300_ppi_until_changed() {
        let mut d = Doc::new();
        assert_eq!(d.snapshot().raster_ppi, 300.0);
        d.apply(ppi(150.0)).unwrap();
        assert_eq!(d.snapshot().raster_ppi, 150.0);
        assert!(d.apply(ppi(0.0)).is_err());
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().raster_ppi, 300.0);
    }

    #[test]
    fn the_raster_ppi_stays_within_72_to_1200() {
        let mut d = Doc::new();
        for bad in [0.0, 71.0, 1201.0] {
            assert!(d.apply(ppi(bad)).is_err());
            assert_eq!(d.snapshot().raster_ppi, 300.0);
        }
        d.apply(ppi(1200.0)).unwrap();
        assert_eq!(d.snapshot().raster_ppi, 1200.0);
    }

    fn ppi(raster_ppi: f64) -> Command {
        Command::SetDocument {
            raster_ppi: Some(raster_ppi),
            color_mode: None,
        }
    }

    fn cmyk(d: &mut Doc) {
        d.apply(Command::SetDocument {
            raster_ppi: None,
            color_mode: Some(ColorMode::Cmyk),
        })
        .unwrap();
    }

    fn process(c: f32, m: f32, y: f32, k: f32) -> Color {
        Color::Cmyk {
            cmyk: [c, m, y, k],
            alpha: 1.0,
        }
    }

    #[test]
    fn a_document_is_rgb_until_switched_to_cmyk_and_keeps_its_colours() {
        let mut d = Doc::new();
        assert_eq!(d.snapshot().color_mode, ColorMode::Rgb);
        cmyk(&mut d);
        let s = d.snapshot();
        assert_eq!((s.color_mode, s.raster_ppi), (ColorMode::Cmyk, 300.0));
        assert_eq!(
            s.pages[0].children[0].style.fills,
            [Fill::solid(0xe8452cff)]
        );
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().color_mode, ColorMode::Rgb);
    }

    #[test]
    fn new_layers_in_a_cmyk_document_get_process_gray_black_and_white() {
        let (mut d, p) = empty();
        cmyk(&mut d);
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let t = create(&mut d, &p, NewKind::Text, [0.0; 4]);
        let l = create(&mut d, &p, NewKind::Line, [0.0; 4]);
        let f = create(&mut d, &p, NewKind::Frame, [0.0; 4]);
        let pg = page(&d);
        let style = |id: &str| {
            pg.children
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .style
                .clone()
        };
        assert_eq!(style(&r).fills, [Fill::solid(process(0.0, 0.0, 0.0, 0.15))]);
        assert_eq!(style(&t).fills, [Fill::solid(process(0.0, 0.0, 0.0, 1.0))]);
        assert_eq!(
            style(&l).strokes,
            [Fill::solid(process(0.0, 0.0, 0.0, 1.0))]
        );
        assert_eq!(style(&f).fills, [Fill::solid(process(0.0, 0.0, 0.0, 0.0))]);
    }

    #[test]
    fn cmyk_colours_render_through_the_fogra51_preview() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let cyan = Color::Cmyk {
            cmyk: [1.0, 0.0, 0.0, 0.0],
            alpha: 0.5,
        };
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![
                    Fill::solid(process(0.0, 0.0, 0.0, 0.0)),
                    Fill::solid(cyan),
                ]),
                ..Props::default()
            },
        );
        let colors: Vec<[f32; 4]> = d
            .render(0)
            .into_iter()
            .filter_map(|op| match op {
                Op::FillPath {
                    paint: Paint::Solid { color, .. },
                    ..
                } => Some(color),
                _ => None,
            })
            .collect();
        let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.03);
        assert!(near(colors[0], [1.0; 4]), "{colors:?}");
        assert!(near(colors[1], [0.0, 0.641, 0.896, 0.5]), "{colors:?}");
    }

    fn swatch(d: &mut Doc, name: &str, color: Color, spot: bool) -> Res<String> {
        d.apply(Command::AddSwatch {
            name: name.into(),
            color,
            spot,
        })
        .map(|mut ids| ids.remove(0))
    }

    fn bound(id: &str, tint: f32) -> Color {
        Color::Swatch {
            swatch: id.into(),
            tint,
            alpha: 1.0,
        }
    }

    fn solid_colors(d: &Doc) -> Vec<[f32; 4]> {
        d.render(0)
            .into_iter()
            .filter_map(|op| match op {
                Op::FillPath {
                    paint: Paint::Solid { color, .. },
                    ..
                } => Some(color),
                _ => None,
            })
            .collect()
    }

    fn near(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05)
    }

    #[test]
    fn swatch_names_are_unique() {
        let mut d = Doc::new();
        let red = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        swatch(&mut d, "Blue", Color::Rgb(0x0000ffff), false).unwrap();
        assert!(swatch(&mut d, "Red", process(0.0, 1.0, 1.0, 0.0), true).is_err());
        let rename = |name: &str| Command::SetSwatch {
            id: red.clone(),
            name: Some(name.into()),
            color: None,
            spot: None,
        };
        assert!(d.apply(rename("Blue")).is_err());
        d.apply(rename("Red")).unwrap();
        let names: Vec<_> = d
            .snapshot()
            .palette
            .swatches
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["Red", "Blue"]);
    }

    #[test]
    fn swatches_are_added_renamed_and_deleted_with_undo() {
        let mut d = Doc::new();
        assert_eq!(d.snapshot().palette.swatches, []);
        let id = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        d.apply(Command::SetSwatch {
            id: id.clone(),
            name: Some("Signal".into()),
            color: None,
            spot: None,
        })
        .unwrap();
        assert_eq!(
            d.snapshot().palette.swatches,
            [Swatch {
                id: id.clone(),
                name: "Signal".into(),
                color: Color::Rgb(0xff0000ff),
                spot: false,
            }]
        );
        d.apply(Command::DeleteSwatch { id: id.clone() }).unwrap();
        assert_eq!(d.snapshot().palette.swatches, []);
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().palette.swatches[0].id, id);
    }

    #[test]
    fn a_fill_bound_to_a_swatch_follows_the_swatch() {
        let (mut d, p) = empty();
        let id = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(bound(&id, 1.0))]),
                ..Props::default()
            },
        );
        assert_eq!(solid_colors(&d), [[1.0, 0.0, 0.0, 1.0]]);
        d.apply(Command::SetSwatch {
            id,
            name: None,
            color: Some(Color::Rgb(0x0000ffff)),
            spot: None,
        })
        .unwrap();
        assert_eq!(solid_colors(&d), [[0.0, 0.0, 1.0, 1.0]]);
    }

    #[test]
    fn spot_swatches_have_a_cmyk_alternate_and_render_their_tint() {
        let (mut d, p) = empty();
        assert!(swatch(&mut d, "HKS 57", Color::Rgb(0xff00ffff), true).is_err());
        let id = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.0), true).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![
                    Fill::solid(bound(&id, 1.0)),
                    Fill::solid(bound(&id, 0.0)),
                ]),
                ..Props::default()
            },
        );
        let c = solid_colors(&d);
        assert!(near(c[0], [0.907, 0.0, 0.501, 1.0]), "{c:?}");
        assert!(near(c[1], [1.0; 4]), "{c:?}");
    }

    #[test]
    fn deleting_a_swatch_detaches_what_uses_it() {
        let (mut d, p) = empty();
        let red = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        let spot = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.2), true).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(Color::Swatch {
                    swatch: red.clone(),
                    tint: 1.0,
                    alpha: 0.5,
                })]),
                strokes: Some(vec![Fill::solid(bound(&spot, 0.5))]),
                ..Props::default()
            },
        );
        d.apply(Command::DeleteSwatch { id: red }).unwrap();
        d.apply(Command::DeleteSwatch { id: spot }).unwrap();
        let style = &page(&d).children[0].style;
        assert_eq!(style.fills, [Fill::solid(0xff000080)]);
        assert_eq!(
            style.strokes,
            [Fill::solid(Color::Cmyk {
                cmyk: [0.0, 0.5, 0.0, 0.1],
                alpha: 1.0
            })]
        );
    }

    #[test]
    fn colours_out_of_range_are_rejected() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let fill = |c| Props {
            fills: Some(vec![Fill::solid(c)]),
            ..Props::default()
        };
        let set = |d: &mut Doc, props| {
            d.apply(Command::Set {
                id: r.clone(),
                props,
            })
        };
        assert!(set(&mut d, fill(process(0.0, 1.5, 0.0, 0.0))).is_err());
        assert!(set(&mut d, fill(bound("x", -0.5))).is_err());
        assert!(swatch(&mut d, "X", process(0.0, 0.0, 0.0, 2.0), false).is_err());
        assert!(set(&mut d, fill(process(0.0, 1.0, 0.0, 0.0))).is_ok());
    }

    fn collection(d: &mut Doc, name: &str) -> (String, String) {
        let ids = d
            .apply(Command::AddCollection { name: name.into() })
            .unwrap();
        (ids[0].clone(), ids[1].clone())
    }

    fn variable(d: &mut Doc, collection: &str, name: &str, value: Value) -> Res<String> {
        d.apply(Command::AddVariable {
            collection: collection.into(),
            name: name.into(),
            value,
        })
        .map(|mut ids| ids.remove(0))
    }

    fn set_value(d: &mut Doc, id: &str, mode: &str, value: Value) -> Res<Vec<String>> {
        d.apply(Command::SetVariable {
            id: id.into(),
            name: None,
            mode: Some(mode.into()),
            value: Some(value),
        })
    }

    fn use_mode(d: &mut Doc, id: &str, collection: &str, mode: Option<&str>) {
        d.apply(Command::UseMode {
            id: id.into(),
            collection: collection.into(),
            mode: mode.map(String::from),
        })
        .unwrap();
    }

    fn var(id: &str) -> Color {
        Color::Variable {
            variable: id.into(),
            alpha: 1.0,
        }
    }

    #[test]
    fn a_fill_bound_to_a_colour_variable_takes_the_mode_of_its_frame_or_page() {
        let (mut d, p) = empty();
        let (c, light) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let bg = variable(&mut d, &c, "Bg", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        set_value(&mut d, &bg, &dark, Value::Color(Color::Rgb(0x0000ffff))).unwrap();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let r = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(var(&bg))]),
                ..Props::default()
            },
        );
        let red = [1.0, 0.0, 0.0, 1.0];
        let blue = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(solid_colors(&d)[1], red);
        use_mode(&mut d, &p, &c, Some(&dark));
        assert_eq!(solid_colors(&d)[1], blue);
        use_mode(&mut d, &f, &c, Some(&light));
        assert_eq!(solid_colors(&d)[1], red);
        let s = d.snapshot();
        assert_eq!(s.pages[0].modes[&c], dark);
        let fr = &s.pages[0].children[0];
        assert_eq!(fr.modes[&c], light);
        assert_eq!(children(fr)[0].style.fills, [Fill::solid(var(&bg))]);
        use_mode(&mut d, &f, &c, None);
        assert_eq!(solid_colors(&d)[1], blue);
    }

    fn bind(d: &mut Doc, id: &str, prop: &str, variable: Option<&str>) -> Res<Vec<String>> {
        d.apply(Command::Bind {
            id: id.into(),
            prop: prop.into(),
            variable: variable.map(String::from),
        })
    }

    #[test]
    fn bound_numbers_follow_their_variable_in_mm_and_percent_until_set_directly() {
        let (mut d, p) = empty();
        let (c, _) = collection(&mut d, "Sizes");
        let big = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Big".into(),
            })
            .unwrap()
            .remove(0);
        let size = variable(&mut d, &c, "Size", Value::Number(10.0)).unwrap();
        let half = variable(&mut d, &c, "Half", Value::Number(50.0)).unwrap();
        set_value(&mut d, &size, &big, Value::Number(20.0)).unwrap();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 100.0]);
        let r = create(&mut d, &f, NewKind::Rect, [5.0, 5.0, 1.0, 1.0]);
        for prop in ["w", "radius", "strokeWeight"] {
            bind(&mut d, &r, prop, Some(&size)).unwrap();
        }
        bind(&mut d, &r, "opacity", Some(&half)).unwrap();
        let rect = |d: &Doc| {
            let pg = page(d);
            let n = &children(&pg.children[0])[0];
            let radius = match n.kind {
                Kind::Shape(Shape::Rect { radius }) => radius,
                _ => -1.0,
            };
            (n.w, radius, n.style.stroke_weight, n.style.opacity)
        };
        let mm = |v: f64| v * MM;
        assert_eq!(rect(&d), (mm(10.0), mm(10.0) as f32, mm(10.0) as f32, 0.5));
        assert_eq!(page(&d).children[0].bindings, Default::default());
        assert_eq!(children(&page(&d).children[0])[0].bindings["w"], size);
        use_mode(&mut d, &f, &c, Some(&big));
        assert_eq!(rect(&d).0, mm(20.0));
        d.apply(Command::Undo).unwrap();
        assert_eq!(rect(&d).0, mm(10.0));
        set(
            &mut d,
            &r,
            Props {
                radius: Some(1.0),
                ..Props::default()
            },
        );
        d.apply(Command::SetFrame {
            id: r.clone(),
            x: 5.0,
            y: 5.0,
            w: 3.0,
            h: 1.0,
            ignore_constraints: false,
        })
        .unwrap();
        set_value(&mut d, &size, &big, Value::Number(30.0)).unwrap();
        use_mode(&mut d, &f, &c, Some(&big));
        assert_eq!(rect(&d), (3.0, 1.0, mm(30.0) as f32, 0.5));
        let pg = page(&d);
        let bindings = &children(&pg.children[0])[0].bindings;
        assert_eq!(
            bindings.keys().collect::<Vec<_>>(),
            ["opacity", "strokeWeight"]
        );
        bind(&mut d, &r, "strokeWeight", None).unwrap();
        assert!(bind(&mut d, &r, "name", Some(&size)).is_err());
        let color = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0))).unwrap();
        assert!(bind(&mut d, &r, "w", Some(&color)).is_err());
    }

    #[test]
    fn collections_modes_and_variables_are_renamed_and_deleted_and_users_keep_their_values() {
        let (mut d, p) = empty();
        let (c, first) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let spot = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.0), true).unwrap();
        let ink = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        let size = variable(&mut d, &c, "Size", Value::Number(2.0)).unwrap();
        assert!(variable(&mut d, &c, "Ink", Value::Number(1.0)).is_err());
        assert!(set_value(&mut d, &ink, &dark, Value::Number(1.0)).is_err());
        assert!(set_value(&mut d, &ink, &dark, Value::Color(var(&ink))).is_err());
        set_value(&mut d, &ink, &dark, Value::Color(bound(&spot, 0.5))).unwrap();
        d.apply(Command::SetCollection {
            id: c.clone(),
            name: "Brand".into(),
        })
        .unwrap();
        d.apply(Command::SetMode {
            collection: c.clone(),
            id: dark.clone(),
            name: "Night".into(),
        })
        .unwrap();
        let s = d.snapshot();
        assert_eq!(s.palette.collections[0].name, "Brand");
        assert_eq!(s.palette.collections[0].modes[1].name, "Night");

        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(Color::Variable {
                    variable: ink.clone(),
                    alpha: 0.5,
                })]),
                ..Props::default()
            },
        );
        bind(&mut d, &r, "radius", Some(&size)).unwrap();
        use_mode(&mut d, &r, &c, Some(&dark));
        d.apply(Command::DeleteVariable { id: ink.clone() })
            .unwrap();
        d.apply(Command::DeleteVariable { id: size.clone() })
            .unwrap();
        let n = &page(&d).children[0];
        assert_eq!(
            n.style.fills,
            [Fill::solid(Color::Swatch {
                swatch: spot.clone(),
                tint: 0.5,
                alpha: 0.5
            })]
        );
        assert!(n.bindings.is_empty());
        assert_eq!(
            n.kind,
            Kind::Shape(Shape::Rect {
                radius: 2.0 * MM as f32
            })
        );

        let ink = variable(&mut d, &c, "Ink", Value::Color(bound(&spot, 1.0))).unwrap();
        d.apply(Command::DeleteSwatch { id: spot }).unwrap();
        assert_eq!(
            d.snapshot().palette.variables[0].values[&first],
            Value::Color(process(0.0, 1.0, 0.0, 0.0))
        );
        use_mode(&mut d, &p, &c, Some(&first));
        assert!(
            d.apply(Command::DeleteMode {
                collection: c.clone(),
                id: first.clone(),
            })
            .is_ok()
        );
        assert_eq!(page(&d).modes, Modes::new());
        assert_eq!(
            d.snapshot().palette.variables[0]
                .values
                .keys()
                .collect::<Vec<_>>(),
            [&dark]
        );
        assert!(
            d.apply(Command::DeleteMode {
                collection: c.clone(),
                id: dark,
            })
            .is_err()
        );
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(var(&ink))]),
                ..Props::default()
            },
        );
        d.apply(Command::DeleteCollection { id: c }).unwrap();
        let s = d.snapshot();
        assert_eq!(s.pages[0].children[0].modes, Modes::new());
        assert!(s.palette.collections.is_empty() && s.palette.variables.is_empty());
        assert_eq!(
            s.pages[0].children[0].style.fills,
            [Fill::solid(process(0.0, 1.0, 0.0, 0.0))]
        );
    }

    #[test]
    fn children_follow_their_constraints_when_the_frame_resizes() {
        use Constraint::*;
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 100.0]);
        let cases = [
            (Min, Max, [10.0, 20.0, 30.0, 40.0], [10.0, 60.0, 30.0, 40.0]),
            (
                Max,
                Min,
                [10.0, 20.0, 30.0, 40.0],
                [110.0, 20.0, 30.0, 40.0],
            ),
            (
                Stretch,
                Stretch,
                [10.0, 20.0, 30.0, 40.0],
                [10.0, 20.0, 130.0, 80.0],
            ),
            (
                Center,
                Center,
                [40.0, 40.0, 20.0, 20.0],
                [90.0, 60.0, 20.0, 20.0],
            ),
            (
                Scale,
                Scale,
                [10.0, 20.0, 30.0, 40.0],
                [20.0, 28.0, 60.0, 56.0],
            ),
        ];
        let ids: Vec<_> = cases
            .iter()
            .map(|&(h, v, rect, _)| {
                let r = create(&mut d, &f, NewKind::Rect, rect);
                set(
                    &mut d,
                    &r,
                    Props {
                        constraints: Some(Constraints {
                            horizontal: h,
                            vertical: v,
                        }),
                        ..Props::default()
                    },
                );
                r
            })
            .collect();
        let g = d
            .apply(Command::Group {
                ids: vec![ids[0].clone()],
                frame: false,
            })
            .unwrap()
            .remove(0);
        set(
            &mut d,
            &g,
            Props {
                constraints: Some(Constraints {
                    horizontal: Max,
                    vertical: Min,
                }),
                ..Props::default()
            },
        );
        d.apply(Command::SetFrame {
            id: f,
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 140.0,
            ignore_constraints: false,
        })
        .unwrap();
        let pg = page(&d);
        let kids = children(&pg.children[0]);
        assert_eq!(frame(&children(&kids[0])[0]), [110.0, 20.0, 30.0, 40.0]);
        for (n, case) in kids[1..].iter().zip(&cases[1..]) {
            assert_eq!(frame(n), case.3, "{:?}", (case.0, case.1));
        }
        assert_eq!(
            kids[0].style.constraints,
            Constraints {
                horizontal: Max,
                vertical: Min
            }
        );
    }

    fn auto(d: &mut Doc, id: &str, props: Props) {
        set(
            d,
            id,
            Props {
                direction: Some(Direction::Horizontal),
                ..props
            },
        );
    }

    fn frames(d: &Doc, id: &str) -> Vec<[f64; 4]> {
        fn find<'a>(nodes: &'a [Node], id: &str) -> Option<&'a Node> {
            nodes
                .iter()
                .find_map(|n| (n.id == id).then_some(n).or_else(|| find(children(n), id)))
        }
        let pg = page(d);
        let f = find(&pg.children, id).unwrap();
        [frame(f)]
            .into_iter()
            .chain(children(f).iter().map(frame))
            .collect()
    }

    fn sizing(horizontal: Size, vertical: Size) -> Option<Sizing> {
        Some(Sizing {
            horizontal,
            vertical,
        })
    }

    #[test]
    fn a_hugging_auto_layout_frame_stacks_its_children_with_gap_and_padding() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [100.0, 100.0, 1.0, 1.0]);
        for s in [10.0, 20.0, 30.0] {
            create(&mut d, &f, NewKind::Rect, [0.0, 0.0, s, s]);
        }
        auto(
            &mut d,
            &f,
            Props {
                gap: Some(5.0),
                padding_top: Some(10.0),
                padding_right: Some(10.0),
                padding_bottom: Some(10.0),
                padding_left: Some(10.0),
                align_cross: Some(Align3::Center),
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        assert_eq!(
            frames(&d, &f),
            [
                [100.0, 100.0, 90.0, 50.0],
                [110.0, 120.0, 10.0, 10.0],
                [125.0, 115.0, 20.0, 20.0],
                [150.0, 110.0, 30.0, 30.0],
            ]
        );
        set(
            &mut d,
            &f,
            Props {
                direction: Some(Direction::Vertical),
                align_cross: Some(Align3::End),
                ..Props::default()
            },
        );
        assert_eq!(
            frames(&d, &f),
            [
                [100.0, 100.0, 50.0, 90.0],
                [130.0, 110.0, 10.0, 10.0],
                [120.0, 125.0, 20.0, 20.0],
                [110.0, 150.0, 30.0, 30.0],
            ]
        );
    }

    #[test]
    fn fill_children_share_the_free_space_and_absolute_ones_keep_their_place() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 200.0, 50.0]);
        let _a = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 30.0, 10.0]);
        let x = create(&mut d, &f, NewKind::Ellipse, [70.0, 30.0, 5.0, 5.0]);
        set(
            &mut d,
            &b,
            Props {
                sizing: sizing(Size::Fill, Size::Fill),
                ..Props::default()
            },
        );
        set(
            &mut d,
            &x,
            Props {
                absolute: Some(true),
                ..Props::default()
            },
        );
        auto(
            &mut d,
            &f,
            Props {
                gap: Some(5.0),
                ..Props::default()
            },
        );
        assert_eq!(
            frames(&d, &f),
            [
                [0.0, 0.0, 200.0, 50.0],
                [0.0, 0.0, 10.0, 10.0],
                [15.0, 0.0, 150.0, 50.0],
                [170.0, 0.0, 30.0, 10.0],
                [70.0, 30.0, 5.0, 5.0],
            ]
        );
        d.apply(Command::SetFrame {
            id: b.clone(),
            x: 15.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            ignore_constraints: false,
        })
        .unwrap();
        set(
            &mut d,
            &f,
            Props {
                align_main: Some(MainAlign::SpaceBetween),
                ..Props::default()
            },
        );
        assert_eq!(
            frames(&d, &f)[1..4],
            [
                [0.0, 0.0, 10.0, 10.0],
                [85.0, 0.0, 10.0, 10.0],
                [170.0, 0.0, 30.0, 10.0]
            ]
        );
    }

    #[test]
    fn nested_hug_frames_grow_their_parents_and_resizing_a_hug_axis_makes_it_fixed() {
        let (mut d, p) = empty();
        let outer = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 1.0, 1.0]);
        let inner = create(&mut d, &outer, NewKind::Frame, [0.0, 0.0, 1.0, 1.0]);
        create(&mut d, &inner, NewKind::Rect, [0.0, 0.0, 10.0, 20.0]);
        create(&mut d, &inner, NewKind::Rect, [0.0, 0.0, 10.0, 20.0]);
        let hug = Props {
            sizing: sizing(Size::Hug, Size::Hug),
            padding_left: Some(1.0),
            ..Props::default()
        };
        auto(&mut d, &inner, hug);
        auto(
            &mut d,
            &outer,
            Props {
                sizing: sizing(Size::Hug, Size::Hug),
                padding_left: Some(1.0),
                ..Props::default()
            },
        );
        assert_eq!(frames(&d, &outer)[0], [0.0, 0.0, 22.0, 20.0]);
        let (c, _) = collection(&mut d, "Space");
        let gap = variable(&mut d, &c, "Gap", Value::Number(10.0)).unwrap();
        bind(&mut d, &inner, "gap", Some(&gap)).unwrap();
        assert_eq!(frames(&d, &outer)[0][2], 22.0 + 10.0 * MM);
        d.apply(Command::SetFrame {
            id: inner.clone(),
            x: 1.0,
            y: 0.0,
            w: 50.0,
            h: 20.0,
            ignore_constraints: false,
        })
        .unwrap();
        assert_eq!(frames(&d, &outer)[0], [0.0, 0.0, 51.0, 20.0]);
        let pg = page(&d);
        let n = &children(&pg.children[0])[0];
        assert_eq!(
            n.layout.sizing,
            Sizing {
                horizontal: Size::Fixed,
                vertical: Size::Hug
            }
        );
    }

    #[test]
    fn shift_a_wraps_layers_in_a_hugging_auto_layout_frame_ordered_by_position() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [50.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &p, NewKind::Rect, [0.0, 5.0, 10.0, 10.0]);
        let f = d
            .apply(Command::AutoLayout {
                ids: vec![a.clone(), b.clone()],
            })
            .unwrap()
            .remove(0);
        let pg = page(&d);
        let n = &pg.children[0];
        assert_eq!(
            (n.id.clone(), n.layout.direction),
            (f.clone(), Direction::Horizontal)
        );
        assert_eq!(ids(children(n)), [b, a]);
        assert_eq!(n.layout.gap, 40.0);
        assert_eq!(
            frames(&d, &f),
            [
                [0.0, 0.0, 60.0, 10.0],
                [0.0, 0.0, 10.0, 10.0],
                [50.0, 0.0, 10.0, 10.0]
            ]
        );

        let g = create(&mut d, &p, NewKind::Frame, [100.0, 100.0, 100.0, 100.0]);
        create(&mut d, &g, NewKind::Rect, [110.0, 150.0, 20.0, 10.0]);
        create(&mut d, &g, NewKind::Rect, [110.0, 120.0, 30.0, 10.0]);
        assert_eq!(
            d.apply(Command::AutoLayout {
                ids: vec![g.clone()]
            })
            .unwrap(),
            std::slice::from_ref(&g)
        );
        let l = &page(&d).children[1].layout;
        assert_eq!((l.direction, l.gap), (Direction::Vertical, 20.0));
        assert_eq!(
            [
                l.padding_top,
                l.padding_right,
                l.padding_bottom,
                l.padding_left
            ],
            [20.0, 60.0, 40.0, 10.0]
        );
        assert_eq!(
            frames(&d, &g),
            [
                [100.0, 100.0, 100.0, 100.0],
                [110.0, 120.0, 30.0, 10.0],
                [110.0, 150.0, 20.0, 10.0]
            ]
        );
    }

    #[test]
    fn resizing_a_frame_without_constraints_leaves_its_children_in_place() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 100.0]);
        let r = create(&mut d, &f, NewKind::Rect, [10.0, 10.0, 10.0, 10.0]);
        set(
            &mut d,
            &r,
            Props {
                constraints: Some(Constraints {
                    horizontal: Constraint::Scale,
                    vertical: Constraint::Max,
                }),
                ..Props::default()
            },
        );
        d.apply(Command::SetFrame {
            id: f.clone(),
            x: -50.0,
            y: 0.0,
            w: 150.0,
            h: 50.0,
            ignore_constraints: true,
        })
        .unwrap();
        assert_eq!(frames(&d, &f)[1], [10.0, 10.0, 10.0, 10.0]);
    }

    fn text(d: &mut Doc, content: &str) -> String {
        let p = page(d).id;
        let t = create(d, &p, NewKind::Text, [0.0, 0.0, 400.0, 400.0]);
        d.apply(Command::SetText {
            id: t.clone(),
            text: content.into(),
        })
        .unwrap();
        t
    }

    fn format(
        d: &mut Doc,
        id: &str,
        range: Option<[usize; 2]>,
        props: TextProps,
    ) -> Res<Vec<String>> {
        d.apply(Command::Format {
            id: id.into(),
            range,
            props,
        })
    }

    fn spans(d: &Doc) -> Vec<Span> {
        match page(d).children.pop().unwrap().kind {
            Kind::Text { spans, .. } => spans,
            k => panic!("not text: {k:?}"),
        }
    }

    fn lens_and(d: &Doc, f: impl Fn(&Attrs) -> f64) -> Vec<(usize, f64)> {
        spans(d).iter().map(|s| (s.len, f(&s.attrs))).collect()
    }

    fn sized(size: f64) -> TextProps {
        TextProps {
            size: Some(size),
            ..TextProps::default()
        }
    }

    #[test]
    fn format_marks_a_range_and_formatting_the_whole_text_replaces_the_marks() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        format(&mut d, &t, Some([0, 5]), sized(20.0)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 20.0), (6, 12.0)]);
        d.apply(Command::SetText {
            id: t.clone(),
            text: "Hello, world".into(),
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(12, 9.0)]);
        d.apply(Command::Undo).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        assert!(format(&mut d, &t, None, sized(0.0)).is_err());
        assert!(format(&mut d, &t, Some([3, 99]), sized(9.0)).is_err());
    }

    #[test]
    fn paragraph_attributes_cover_the_whole_paragraphs_of_a_range() {
        let (mut d, _) = empty();
        let t = text(&mut d, "ab\ncd\nef");
        let centred = TextProps {
            text_align: Some(TextAlign::Center),
            paragraph_spacing: Some(6.0),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([4, 4]), centred).unwrap();
        let s = spans(&d);
        let aligns: Vec<_> = s.iter().map(|s| (s.len, s.attrs.text_align)).collect();
        assert_eq!(
            aligns,
            [
                (3, TextAlign::Left),
                (3, TextAlign::Center),
                (2, TextAlign::Left)
            ]
        );
        assert_eq!(s[1].attrs.paragraph_spacing, 6.0);
    }

    #[test]
    fn a_range_colour_draws_its_glyphs_in_that_colour_and_the_rest_in_the_fills() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        let red = TextProps {
            fill: Some(Color::Rgb(0xff0000ff)),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 2]), red).unwrap();
        let colors: Vec<_> = d
            .render(0)
            .into_iter()
            .filter_map(|op| match op {
                Op::GlyphRun {
                    paint: Paint::Solid { color, .. },
                    glyphs,
                    ..
                } => Some((glyphs.len(), color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            colors,
            [(2, [1.0, 0.0, 0.0, 1.0]), (5, [0.0, 0.0, 0.0, 1.0])]
        );
    }

    fn style(d: &mut Doc, name: &str, size: f64) -> String {
        d.apply(Command::AddTextStyle {
            name: name.into(),
            size,
            line_height: 14.0,
            letter_spacing: 0.0,
            paragraph_spacing: 4.0,
        })
        .unwrap()
        .remove(0)
    }

    #[test]
    fn a_text_style_sets_its_values_where_applied_and_follows_changes_and_variables() {
        let (mut d, p) = empty();
        let t = text(&mut d, "Hello world");
        let body = style(&mut d, "Body", 10.0);
        let styled = |id: &str| TextProps {
            text_style: Some(id.into()),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 5]), styled(&body)).unwrap();
        let s = spans(&d);
        assert_eq!(
            (s[0].len, s[0].attrs.size, s[0].attrs.line_height),
            (5, 10.0, 14.0)
        );
        assert_eq!(s[0].attrs.text_style, body);
        assert_eq!(s[1].attrs.text_style, "");

        d.apply(Command::SetTextStyle {
            id: body.clone(),
            name: None,
            size: Some(11.0),
            line_height: None,
            letter_spacing: None,
            paragraph_spacing: None,
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 11.0), (6, 12.0)]);

        let (c, _) = collection(&mut d, "Type");
        let big = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Big".into(),
            })
            .unwrap()
            .remove(0);
        let v = variable(&mut d, &c, "Body size", Value::Number(8.0)).unwrap();
        set_value(&mut d, &v, &big, Value::Number(16.0)).unwrap();
        bind(&mut d, &body, "size", Some(&v)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 8.0), (6, 12.0)]);
        use_mode(&mut d, &p, &c, Some(&big));
        assert_eq!(lens_and(&d, |a| a.size), [(5, 16.0), (6, 12.0)]);
        assert!(bind(&mut d, &body, "w", Some(&v)).is_err());

        format(&mut d, &t, Some([0, 2]), sized(30.0)).unwrap();
        let s = spans(&d);
        let got: Vec<_> = s
            .iter()
            .map(|s| {
                (
                    s.len,
                    s.attrs.size,
                    s.attrs.line_height,
                    s.attrs.text_style.clone(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (2, 30.0, 14.0, String::new()),
                (3, 16.0, 14.0, body.clone()),
                (6, 12.0, 0.0, String::new())
            ]
        );

        d.apply(Command::DeleteTextStyle { id: body.clone() })
            .unwrap();
        let got: Vec<_> = spans(&d)
            .iter()
            .map(|s| (s.len, s.attrs.size, s.attrs.text_style.clone()))
            .collect();
        assert_eq!(
            got,
            [
                (2, 30.0, String::new()),
                (3, 16.0, String::new()),
                (6, 12.0, String::new())
            ]
        );
        assert!(d.snapshot().palette.text_styles.is_empty());
    }

    #[test]
    fn a_style_applied_to_the_whole_text_also_holds_for_text_typed_into_it() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi");
        let head = style(&mut d, "Head", 24.0);
        format(
            &mut d,
            &t,
            None,
            TextProps {
                text_style: Some(head.clone()),
                ..TextProps::default()
            },
        )
        .unwrap();
        d.apply(Command::SetText {
            id: t.clone(),
            text: String::new(),
        })
        .unwrap();
        d.apply(Command::SetText {
            id: t.clone(),
            text: "New".into(),
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(3, 24.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        let s = spans(&d);
        assert_eq!((s[0].attrs.size, s[0].attrs.line_height), (9.0, 14.0));
        assert_eq!(s[0].attrs.text_style, "");
    }

    #[test]
    fn text_size_binds_to_a_number_variable_in_pt() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        format(&mut d, &t, Some([0, 2]), sized(20.0)).unwrap();
        let (c, _) = collection(&mut d, "Type");
        let v = variable(&mut d, &c, "Size", Value::Number(18.0)).unwrap();
        bind(&mut d, &t, "size", Some(&v)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(8, 18.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        assert!(page(&d).children[0].bindings.is_empty());
    }

    #[test]
    fn copies_keep_their_character_attributes() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        format(&mut d, &t, Some([0, 2]), sized(20.0)).unwrap();
        d.apply(Command::Duplicate { ids: vec![t] }).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(2, 20.0), (6, 12.0)]);
    }
}
