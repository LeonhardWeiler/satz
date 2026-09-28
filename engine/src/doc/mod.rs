use crate::color::{self, Ink};
use crate::color::{Color, ColorMode, Swatch};
use crate::display_list::{CLOSE, LINE, MOVE, Op, Paint, recolor, rect, shift};
use crate::geom::{self, Shape, bounds, contains, fit, near, outline};
use crate::image::{self, ImageInfo};
use crate::inks::Inks;
use crate::layout::{Align3, Direction, Layout, MainAlign, Size, Sizing, arrange};
use crate::pdf::Preset;
use crate::style::{
    Align, Blend, Cap, Constraint, Constraints, Effect, EffectKind, Fill, FillKind, FillStop, Join,
    Style,
};
use crate::text::{
    self, Attrs, Lang, PARAGRAPH, STYLED, Span, TextAlign, TextFrame, TextStyle, Typeface,
    VerticalAlign,
};
use crate::variable::{Collection, Mode, Modes, Palette, Scope, Value, Variable};
use loro::{
    Container, ExpandType, ExportMode, LoroBinaryValue, LoroDoc, LoroMap, LoroText, LoroTree,
    LoroValue, StyleConfig, TextDelta, TreeID, TreeParentId, UndoManager, UpdateOptions,
    ValueOrContainer,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::ops::Range;
use std::rc::Rc;

#[cfg(test)]
mod bench;
mod clipboard;
mod draw;
mod pages;
mod palette;
mod story;

use clipboard::*;
pub use draw::*;
use pages::*;
use story::*;

/// The settings of the document that `Command::SetDocument` changes; `None` keeps one.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub raster_ppi: Option<f64>,
    pub color_mode: Option<ColorMode>,
    /// Pairs the pages into spreads, as in InDesign.
    pub facing_pages: Option<bool>,
    pub ink_limit: Option<f64>,
    pub preset: Option<Preset>,
    pub crop_marks: Option<bool>,
    pub include_bleed: Option<bool>,
}

#[allow(clippy::large_enum_variant)]
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
    /// Places the image `image`, added with `Doc::add_image`, on top of `parent` as
    /// a rectangle [x, y, w, h] filled with it; returns its id.
    PlaceImage {
        parent: String,
        image: String,
        name: String,
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
    /// Replaces a range of the text in UTF-16 code units; inserted text takes the
    /// attributes of the character before it.
    EditText {
        id: String,
        range: [usize; 2],
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
    /// Scales the sizes and spacing in pt of the story of a text layer by `by`.
    ScaleText {
        id: String,
        by: f64,
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
    SetDocument(Settings),
    /// Pastes above the topmost of `above`, or else into the parent the layers were
    /// copied from when that is on `page`, or else onto `page`.
    Paste {
        above: Vec<String>,
        #[serde(default)]
        page: Option<String>,
    },
    /// Adds an empty page after `after`, or at the end, of the size of that page or
    /// the last one; returns its id.
    AddPage {
        after: Option<String>,
    },
    /// Adds a copy of a page with copies of its layers after it; returns its id.
    DuplicatePage {
        id: String,
    },
    /// Sets the trim size and bleed of a page in pt; with `scale` its layers scale
    /// with its size and their text with the square root of its area.
    SetPage {
        id: String,
        width: Option<f64>,
        height: Option<f64>,
        bleed: Option<f64>,
        #[serde(default)]
        scale: bool,
    },
    /// Removes a page other than the last.
    DeletePage {
        id: String,
    },
    /// Moves a page to `index` among the pages.
    MovePage {
        id: String,
        index: usize,
    },
    /// Adds a master of the size of the page `like` or the first page, named with
    /// the next free letter; returns its id.
    AddMaster {
        like: Option<String>,
    },
    SetMaster {
        id: String,
        name: String,
    },
    /// Removes a master; its pages keep their own layers.
    DeleteMaster {
        id: String,
    },
    /// Draws the master under the layers of a page, or none.
    UseMaster {
        page: String,
        master: Option<String>,
    },
    /// Copies a layer of a page's master onto the page, below its own layers, and
    /// hides the master's on that page; returns the copy.
    Override {
        page: String,
        id: String,
    },
    /// Removes overriding copies, or all of a page's, and shows the master layers
    /// they stood for again.
    ResetToMaster {
        ids: Vec<String>,
    },
    /// Threads the text layer `to` after `from`, before the frame that followed it:
    /// the story of `from` flows on in `to`, whose own text ends the story as a
    /// paragraph. Frames with a next frame are fixed, the last may be auto height.
    Thread {
        from: String,
        to: String,
    },
    /// Breaks the thread after a text layer; its story stays with the frames before.
    Unthread {
        id: String,
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
    pub hidden: Option<bool>,
    pub locked: Option<bool>,
    pub clip: Option<bool>,
    pub radius: Option<f32>,
    pub corners: Option<Vec<f32>>,
    pub start: Option<f32>,
    pub sweep: Option<f32>,
    pub inner: Option<f32>,
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
    pub inset_top: Option<f64>,
    pub inset_right: Option<f64>,
    pub inset_bottom: Option<f64>,
    pub inset_left: Option<f64>,
    pub columns: Option<u32>,
    pub gutter: Option<f64>,
    pub vertical_align: Option<VerticalAlign>,
    pub baseline_grid: Option<f64>,
    pub baseline_start: Option<f64>,
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
        if let Some(c) = &self.corners {
            if !matches!(c.len(), 0 | 4) {
                return Err("corners take 4 radii or none".into());
            }
            for &r in c {
                within(Some(r.into()), 0.0, f64::MAX, "radius")?;
            }
        }
        within(self.count.map(f64::from), 3.0, 60.0, "count")?;
        within(f(self.ratio), 0.01, 1.0, "ratio")?;
        within(f(self.start), -360.0, 360.0, "start")?;
        within(f(self.sweep), 0.0, 1.0, "sweep")?;
        within(f(self.inner), 0.0, 1.0, "inner radius")?;
        within(f(self.opacity), 0.0, 1.0, "opacity")?;
        for p in [
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        ] {
            within(p, 0.0, f64::MAX, "padding")?;
        }
        for p in [
            self.inset_top,
            self.inset_right,
            self.inset_bottom,
            self.inset_left,
        ] {
            within(p, 0.0, f64::MAX, "inset")?;
        }
        within(self.columns.map(f64::from), 1.0, 20.0, "columns")?;
        within(self.gutter, 0.0, f64::MAX, "gutter")?;
        within(self.baseline_grid, 0.0, f64::MAX, "baseline grid")?;
        within(self.baseline_start, 0.0, f64::MAX, "baseline start")?;
        within(self.gap, f64::MIN, f64::MAX, "gap")?;
        for e in self.effects.iter().flatten() {
            within(Some(e.radius.into()), 0.0, f64::MAX, "blur")?;
            for v in [e.x, e.y] {
                within(f(Some(v)), f64::MIN, f64::MAX, "effect offset")?;
            }
            e.color.check()?;
        }
        for f in self.fills.iter().chain(&self.strokes).flatten() {
            f.color.check()?;
            if !f.transform.iter().all(|v| v.is_finite()) {
                return Err("paint transforms must be finite".into());
            }
            for s in &f.stops {
                within(Some(s.at.into()), 0.0, 1.0, "stop")?;
                s.color.check()?;
            }
        }
        if self.path.as_ref().is_some_and(|p| !geom::valid(p)) {
            return Err("not a path".into());
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
    pub hyphenate: Option<bool>,
    pub lang: Option<Lang>,
    pub font: Option<Typeface>,
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

/// What a node of the tree is; stored under `KIND`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeKind {
    Page,
    Master,
    /// The root that deleted nodes move under.
    Trash,
    Group,
    Frame,
    Text,
    Shape,
}

impl NodeKind {
    const ALL: [NodeKind; 7] = [
        NodeKind::Page,
        NodeKind::Master,
        NodeKind::Trash,
        NodeKind::Group,
        NodeKind::Frame,
        NodeKind::Text,
        NodeKind::Shape,
    ];

    fn as_str(self) -> &'static str {
        match self {
            NodeKind::Page => "page",
            NodeKind::Master => "master",
            NodeKind::Trash => "trash",
            NodeKind::Group => "group",
            NodeKind::Frame => "frame",
            NodeKind::Text => "text",
            NodeKind::Shape => "shape",
        }
    }
}

pub struct Doc {
    doc: LoroDoc,
    tree: LoroTree,
    undo: UndoManager,
    clipboard: Vec<(Clip, Option<TreeID>)>,
    /// The flows through all text layers and the snapshot, from the end of the last
    /// command on.
    flows: RefCell<Option<Rc<HashMap<TreeID, Flow>>>>,
    snapshot: RefCell<Option<Rc<Snapshot>>>,
    /// Each story as last set, by its first frame.
    sets: RefCell<HashMap<TreeID, Set>>,
    /// The version of the last change other than text set again for a new font.
    version: String,
}

type Res<T> = Result<T, String>;

const MM: f64 = 72.0 / 25.4;

/// Pixels per inch that images need to print sharp.
const PRINT_PPI: f64 = 300.0;

/// The origin of commits that add images, which undo leaves alone: the bytes stay
/// for whatever uses them now or after an undo.
const IMAGE_ORIGIN: &str = "image";

/// The origin of commits that set text again for a font added, which undo leaves alone.
const LAYOUT_ORIGIN: &str = "layout";

/// Properties a variable can bind to: a font variable to `font`, a number variable
/// to the others; lengths count in mm, opacity in %, text size in pt.
const BINDABLE: [&str; 12] = [
    "font",
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

/// The keys of a text layer that belong to its story and move with it.
const STORY: [&str; 11] = [
    "size",
    "lineHeight",
    "letterSpacing",
    "paragraphSpacing",
    "fill",
    "textStyle",
    "textAlign",
    "hyphenate",
    "lang",
    "fills",
    "font",
];

/// Keys of a node that give its kind or link it to other nodes: a text layer to the
/// next frame of its thread, a page to its master and the master layers it
/// overrides, a page layer to the master layer it overrides, and a layer on the
/// left page of a master spread to the one on the right it was copied from.
const KIND: &str = "kind";
const NEXT: &str = "next";
const MASTER: &str = "master";
const DETACHED: &str = "detached";
const OVERRIDE_OF: &str = "overrideOf";
const LEFT_OF: &str = "leftOf";
/// Figma's selection blue at 30 %.
const SELECTION: [f32; 4] = [0.051, 0.6, 1.0, 0.3];

const WHITE: u32 = 0xffffffff;

const BLACK: u32 = 0x000000ff;

const SAMPLE: &str = "Satz sets type in the browser. The engine shapes this paragraph \
with harfrust, breaks it into lines with the Knuth-Plass algorithm and justifies \
every line but the last to the width of its frame. The canvas and the PDF draw \
the same glyphs from the same font.";

/// Draws siblings; a mask masks the siblings above it.
/// How far the master spread `m` moves to show its side on the page `p`: a left
/// page shows the master's left page.
fn undo_manager(doc: &LoroDoc) -> UndoManager {
    let mut undo = UndoManager::new(doc);
    undo.add_exclude_origin_prefix(IMAGE_ORIGIN);
    undo.add_exclude_origin_prefix(LAYOUT_ORIGIN);
    undo
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

fn check_frame([x, y, w, h]: [f64; 4]) -> Res<()> {
    if ![x, y, w, h].iter().all(|v| v.is_finite()) {
        return Err("frames must be finite".into());
    }
    if !(w >= 0.0 && h >= 0.0) {
        return Err("width and height must not be negative".into());
    }
    Ok(())
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

impl Doc {
    fn with(doc: LoroDoc) -> Doc {
        doc.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::After,
        }));
        let tree = doc.get_tree("nodes");
        tree.enable_fractional_index(0);
        Doc {
            undo: undo_manager(&doc),
            version: format!("{:?}", doc.oplog_frontiers()),
            doc,
            tree,
            clipboard: Vec::new(),
            flows: RefCell::new(None),
            snapshot: RefCell::new(None),
            sets: RefCell::new(HashMap::new()),
        }
    }

    /// An empty document of `pages` pages `w` × `h` pt with 3 mm bleed.
    pub fn blank(w: f64, h: f64, pages: usize, facing: bool, mode: ColorMode) -> Doc {
        let mut d = Doc::with(LoroDoc::new());
        let page = d.tree.create(None).unwrap();
        let m = d.meta(page);
        m.insert(KIND, NodeKind::Page.as_str()).unwrap();
        m.insert("width", w).unwrap();
        m.insert("height", h).unwrap();
        m.insert("bleed", 3.0 * MM).unwrap();
        d.apply(Command::SetDocument(Settings {
            raster_ppi: Some(300.0),
            color_mode: Some(mode),
            facing_pages: Some(facing),
            ..Settings::default()
        }))
        .unwrap();
        for _ in 1..pages {
            d.apply(Command::AddPage { after: None }).unwrap();
        }
        d.undo = undo_manager(&d.doc);
        d.finish(vec![], false).unwrap();
        d
    }

    pub fn new() -> Doc {
        let mut d = Doc::blank(148.0 * MM, 210.0 * MM, 1, true, ColorMode::Rgb);
        let page = d.build_snapshot().pages[0].id.clone();
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
                NewKind::Line,
                [112.0, 62.0, 30.0, 0.0],
                Props {
                    strokes: Some(vec![Fill::solid(WHITE)]),
                    stroke_weight: Some(2.0),
                    arrow_end: Some(true),
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
        d.apply(Command::SetFrame {
            id: text.clone(),
            x: 15.0 * MM,
            y: 95.0 * MM,
            w: 118.0 * MM,
            h: 70.0 * MM,
            ignore_constraints: false,
        })
        .unwrap();
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
                hyphenate: Some(true),
                ..TextProps::default()
            },
        })
        .unwrap();
        d.undo = undo_manager(&d.doc);
        d.finish(vec![], false).unwrap();
        d
    }

    /// The document for `load`, without its history and so without the images
    /// that only the trash or undo still use.
    pub fn save(&self) -> Vec<u8> {
        let mut ids = Vec::new();
        for r in self.tree.roots() {
            if self.kind(r) != Some(NodeKind::Trash) {
                self.walk(r, &mut ids);
            }
        }
        let used: HashSet<String> = ids
            .into_iter()
            .filter_map(|id| value(&self.meta(id), "fills"))
            .filter_map(|v| serde_json::from_value::<Vec<Fill>>(serde_json::to_value(v).ok()?).ok())
            .flatten()
            .filter_map(|f| f.image)
            .collect();
        let images = self.doc.get_map("images");
        let unused: Vec<_> = images
            .keys()
            .filter(|h| !used.contains(h.as_str()))
            .collect();
        let doc = if unused.is_empty() {
            self.doc.clone()
        } else {
            let doc = self.doc.fork();
            let images = doc.get_map("images");
            for h in unused {
                images.delete(&h).expect("delete");
            }
            doc.commit();
            doc
        };
        doc.export(ExportMode::shallow_snapshot(&doc.state_frontiers()))
            .expect("export")
    }

    /// Adds a PNG or JPEG file to the images of the document, for `PlaceImage`;
    /// undo does not take it out again.
    pub fn add_image(&mut self, bytes: &[u8]) -> Res<ImageInfo> {
        let bytes = LoroBinaryValue::from(bytes.to_vec());
        let info = image::register(bytes.clone())?;
        let images = self.doc.get_map("images");
        if images.get(&info.hash).is_none() {
            self.doc.commit();
            self.doc.set_next_commit_origin(IMAGE_ORIGIN);
            images
                .insert(&info.hash, LoroValue::Binary(bytes))
                .map_err(err)?;
            self.doc.commit();
            self.version = format!("{:?}", self.doc.oplog_frontiers());
        }
        Ok(info)
    }

    /// The image `hash` of the document.
    fn image(&self, hash: &str) -> Res<ImageInfo> {
        match self.doc.get_map("images").get(hash) {
            Some(_) => image::info(hash).ok_or_else(|| format!("image {hash} does not decode")),
            None => Err(format!("no image {hash}")),
        }
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }

    pub fn load(bytes: &[u8]) -> Res<Doc> {
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(|_| "not a Satz document")?;
        let d = Doc::with(doc);
        let kinds: Vec<_> = d.tree.roots().into_iter().map(|r| d.kind(r)).collect();
        if !kinds.contains(&Some(NodeKind::Page))
            || kinds
                .iter()
                .any(|k| !matches!(k, Some(NodeKind::Page | NodeKind::Master | NodeKind::Trash)))
        {
            return Err("not a Satz document".into());
        }
        for v in d
            .doc
            .get_map("images")
            .get_value()
            .into_map()
            .unwrap_or_default()
            .values()
        {
            if let LoroValue::Binary(bytes) = v {
                image::register(bytes.clone())?;
            }
        }
        d.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        d.finish(vec![], false)?;
        Ok(d)
    }

    pub fn apply(&mut self, cmd: Command) -> Res<Vec<String>> {
        self.invalidate();
        let history = matches!(cmd, Command::Undo | Command::Redo);
        let out = self.run(cmd);
        debug_assert!(
            out.is_ok() || self.doc.get_pending_txn_len() == 0,
            "a command wrote before it failed"
        );
        let out = self.finish(out?, history)?;
        self.version = format!("{:?}", self.doc.oplog_frontiers());
        Ok(out)
    }

    /// Carries out a command; it checks everything before it writes, so that one that
    /// fails leaves the document as it was.
    fn run(&mut self, cmd: Command) -> Res<Vec<String>> {
        match cmd {
            Command::Create {
                parent,
                kind,
                x,
                y,
                w,
                h,
            } => Ok(vec![self.create(&parent, kind, [x, y, w, h])?.to_string()]),
            Command::PlaceImage {
                parent,
                image,
                name,
                x,
                y,
                w,
                h,
            } => self.place_image(parent, image, name, [x, y, w, h]),
            Command::SetFrame {
                id,
                x,
                y,
                w,
                h,
                ignore_constraints,
            } => self.reframe(id, x, y, w, h, ignore_constraints),
            Command::SetText { id, text } => self.set_text(id, text),
            Command::EditText { id, range, text } => self.edit_text(id, range, text),
            Command::Format { id, range, props } => self.format(id, range, props),
            Command::ScaleText { id, by } => {
                self.scale_text(self.node(&id)?, by)?;
                Ok(vec![])
            }
            Command::AddTextStyle {
                name,
                size,
                line_height,
                letter_spacing,
                paragraph_spacing,
            } => self.add_text_style(name, size, line_height, letter_spacing, paragraph_spacing),
            Command::SetTextStyle {
                id,
                name,
                size,
                line_height,
                letter_spacing,
                paragraph_spacing,
            } => self.set_text_style(
                id,
                name,
                size,
                line_height,
                letter_spacing,
                paragraph_spacing,
            ),
            Command::DeleteTextStyle { id } => self.delete_text_style(id),
            Command::Set { id, props } => self.set_props(id, props),
            Command::SetPath { id, path } => self.set_path(id, path),
            Command::Delete { ids } => self.delete(ids),
            Command::Group { ids, frame } => {
                Ok(vec![self.group(&self.sorted(&ids)?, frame)?.to_string()])
            }
            Command::Ungroup { ids } => self.ungroup(ids),
            Command::Mask { ids } => self.mask(ids),
            Command::AutoLayout { ids } => self.auto_layout(ids),
            Command::Move { ids, parent, index } => self.move_layers(ids, parent, index),
            Command::Order { ids, to } => self.order(ids, to),
            Command::Undo => {
                self.undo.group_end();
                self.undo.undo().map_err(err)?;
                Ok(vec![])
            }
            Command::Redo => {
                self.undo.group_end();
                self.undo.redo().map_err(err)?;
                Ok(vec![])
            }
            Command::BeginUndoGroup => {
                self.undo.group_end();
                self.undo.group_start().map_err(err)?;
                Ok(vec![])
            }
            Command::EndUndoGroup => {
                self.undo.group_end();
                Ok(vec![])
            }
            Command::Duplicate { ids } => self.duplicate(ids),
            Command::SetDocument(s) => self.set_document(s),
            Command::AddSwatch { name, color, spot } => self.add_swatch(name, color, spot),
            Command::SetSwatch {
                id,
                name,
                color,
                spot,
            } => self.set_swatch(id, name, color, spot),
            Command::DeleteSwatch { id } => self.delete_swatch(id),
            Command::SetCollection { id, name } => self.set_collection(id, name),
            Command::DeleteCollection { id } => self.delete_collection(id),
            Command::SetMode {
                collection,
                id,
                name,
            } => self.set_mode(collection, id, name),
            Command::DeleteMode { collection, id } => self.delete_mode(collection, id),
            Command::DeleteVariable { id } => {
                self.delete_variable(&id)?;
                Ok(vec![])
            }
            Command::AddCollection { name } => self.add_collection(name),
            Command::AddMode { collection, name } => self.add_mode(collection, name),
            Command::AddVariable {
                collection,
                name,
                value,
            } => self.add_variable(collection, name, value),
            Command::SetVariable {
                id,
                name,
                mode,
                value,
            } => self.set_variable(id, name, mode, value),
            Command::Bind { id, prop, variable } => self.bind(id, prop, variable),
            Command::UseMode {
                id,
                collection,
                mode,
            } => self.use_mode(id, collection, mode),
            Command::Copy { ids } => self.copy(ids),
            Command::Paste { above, page } => self.paste_clipboard(above, page),
            Command::AddPage { after } => self.add_page(after),
            Command::DuplicatePage { id } => self.duplicate_page(id),
            Command::SetPage {
                id,
                width,
                height,
                bleed,
                scale,
            } => self.set_page(id, width, height, bleed, scale),
            Command::DeletePage { id } => self.delete_page(id),
            Command::MovePage { id, index } => self.move_page(id, index),
            Command::AddMaster { like } => self.add_master(like),
            Command::SetMaster { id, name } => self.set_master(id, name),
            Command::DeleteMaster { id } => self.delete_master(id),
            Command::UseMaster { page, master } => self.use_master(page, master),
            Command::Override { page, id } => self.override_layer(page, id),
            Command::ResetToMaster { ids } => self.reset_to_master(ids),
            Command::Thread { from, to } => self.thread_after(from, to),
            Command::Unthread { id } => self.unthread(id),
        }
    }

    /// Adds a layer of the kind `kind` with Figma's defaults on top of `parent`.
    fn create(&self, parent: &str, kind: NewKind, frame: [f64; 4]) -> Res<TreeID> {
        let p = self.container(parent)?;
        check_frame(frame)?;
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
            path: Some(path),
            ..Props::default()
        };
        let line = vec![MOVE, 0.0, 0.0, LINE, 1.0, 0.0];
        let (name, shape, props) = match kind {
            NewKind::Rect => (
                NodeKind::Shape,
                "rect",
                Props {
                    radius: Some(0.0),
                    ..closed
                },
            ),
            NewKind::Ellipse => (NodeKind::Shape, "ellipse", closed),
            NewKind::Polygon => (
                NodeKind::Shape,
                "polygon",
                Props {
                    count: Some(3),
                    ..closed
                },
            ),
            NewKind::Star => (
                NodeKind::Shape,
                "star",
                Props {
                    count: Some(5),
                    ratio: Some(0.382),
                    ..closed
                },
            ),
            NewKind::Line => (NodeKind::Shape, "path", open(line)),
            NewKind::Path => (NodeKind::Shape, "path", open(Vec::new())),
            NewKind::Text => {
                m.insert_container("text", LoroText::new()).map_err(err)?;
                m.insert("size", 12.0).map_err(err)?;
                (
                    NodeKind::Text,
                    "",
                    Props {
                        fills: Some(vec![Fill::solid(Color::black(mode))]),
                        sizing: Some(Sizing {
                            horizontal: Size::Hug,
                            vertical: Size::Hug,
                        }),
                        ..Props::default()
                    },
                )
            }
            NewKind::Frame => (
                NodeKind::Frame,
                "",
                Props {
                    fills: Some(vec![Fill::solid(Color::white(mode))]),
                    clip: Some(true),
                    ..closed
                },
            ),
        };
        m.insert(KIND, name.as_str()).map_err(err)?;
        if !shape.is_empty() {
            m.insert("shape", shape).map_err(err)?;
        }
        self.set(id, props)?;
        self.set_frame(id, frame)?;
        Ok(id)
    }

    fn finish(&self, out: Vec<String>, history: bool) -> Res<Vec<String>> {
        let palette = self.palette();
        for p in self.tree.roots() {
            if !history && matches!(self.kind(p), Some(NodeKind::Page | NodeKind::Master)) {
                self.settle(p, &Modes::new(), &palette)?;
                self.lay_out(p)?;
            }
        }
        self.doc.commit();
        self.flows.replace(Some(Rc::new(self.flow_all())));
        self.snapshot.replace(Some(Rc::new(self.build_snapshot())));
        Ok(out)
    }

    fn invalidate(&self) {
        self.flows.take();
        self.snapshot.take();
    }

    fn layout(&self, id: TreeID) -> Layout {
        serde_json::to_value(self.meta(id).get_deep_value())
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    /// Fits hugging text and lays out the auto layout frames in `id`'s subtree,
    /// innermost first.
    fn lay_out(&self, id: TreeID) -> Res<()> {
        let kids = self.children(id);
        for &c in &kids {
            self.lay_out(c)?;
        }
        if self.kind(id) == Some(NodeKind::Text) {
            return self.fit(id);
        }
        let l = self.layout(id);
        let Some((horizontal, pad)) = l.axes().filter(|_| self.kind(id) == Some(NodeKind::Frame))
        else {
            return Ok(());
        };
        let flow: Vec<TreeID> = kids
            .into_iter()
            .filter(|&c| !self.layout(c).absolute)
            .collect();
        if flow.is_empty() {
            // With nothing left to hug, a frame keeps its size instead of shrinking
            // to its padding.
            let fixed = |s| if s == Size::Hug { Size::Fixed } else { s };
            let sizing = Sizing {
                horizontal: fixed(l.sizing.horizontal),
                vertical: fixed(l.sizing.vertical),
            };
            if sizing != l.sizing {
                self.meta(id).insert("sizing", loro(sizing)?).map_err(err)?;
            }
            return Ok(());
        }
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
        let fill = |c: TreeID| {
            let cl = self.layout(c);
            [horizontal, !horizontal].map(|a| cl.size(a) == Size::Fill)
        };
        let hug = [horizontal, !horizontal].map(|a| l.size(a) == Size::Hug);
        // A child whose size follows the one it gets, like text that fills the width
        // and hugs its height, sends the frame round again.
        for _ in 0..4 {
            let old = self.bounds(id);
            let [[m0, ms], [c0, cs]] = axes(old);
            let children: Vec<_> = flow
                .iter()
                .map(|&c| {
                    let [[_, a], [_, b]] = axes(self.bounds(c));
                    ([a, b], fill(c))
                })
                .collect();
            let (size, boxes) = arrange(&l, pad, [ms, cs], hug, &children);
            let frame = unaxes([[m0, size[0]], [c0, size[1]]]);
            if frame != old {
                self.set_frame(id, frame)?;
            }
            let mut again = false;
            for (&c, [[a0, a], [b0, b]]) in flow.iter().zip(boxes) {
                let old = self.bounds(c);
                let new = unaxes([[m0 + a0, a], [c0 + b0, b]]);
                if new != old {
                    self.set_frame(c, new)?;
                    if new[2..] != old[2..] {
                        self.lay_out(c)?;
                        again |= self.bounds(c)[2..] != new[2..];
                    }
                }
            }
            if !again {
                break;
            }
        }
        Ok(())
    }

    fn constraints(&self, id: TreeID) -> Constraints {
        value(&self.meta(id), "constraints")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
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

    fn color_mode(&self) -> ColorMode {
        self.setting("colorMode", ColorMode::Rgb)
    }

    /// The document setting `key`, or `default` when it is not set.
    fn setting<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        value(&self.doc.get_map("document"), key)
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or(default)
    }

    fn set(&self, id: TreeID, props: Props) -> Res<()> {
        props.check()?;
        let images = |fills: &Option<Vec<Fill>>| {
            fills
                .iter()
                .flatten()
                .filter(|f| f.kind == FillKind::Image)
                .count()
        };
        if images(&props.strokes) > 0 {
            return Err("strokes cannot be images".into());
        }
        if images(&props.fills) > 0 && self.kind(id) == Some(NodeKind::Text) {
            return Err("text cannot be filled with an image".into());
        }
        for f in props
            .fills
            .iter()
            .flatten()
            .filter(|f| f.kind == FillKind::Image)
        {
            self.image(f.image.as_deref().unwrap_or_default())?;
        }
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
        check_frame([x, y, w, h])?;
        let [least_w, least_h] = self.least_size(id);
        let (w, h) = (w.max(least_w), h.max(least_h));
        let [ox, oy, ow, oh] = self.bounds(id);
        match self.kind(id) {
            Some(NodeKind::Group) => {
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
            Some(NodeKind::Frame) if follow => {
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

    /// The least [w, h] of `id`: 0.01 mm for a box, which would vanish at 0, but 0 for a
    /// path, whose bounds follow its points, and for a side of a text that hugs it.
    fn least_size(&self, id: TreeID) -> [f64; 2] {
        let least = 0.01 * MM;
        match self.kind(id) {
            Some(NodeKind::Shape) if value(&self.meta(id), "shape") == Some("path".into()) => {
                [0.0; 2]
            }
            Some(NodeKind::Shape | NodeKind::Frame) => [least; 2],
            Some(NodeKind::Text) => {
                let s = self.layout(id).sizing;
                [s.horizontal, s.vertical].map(|s| if s == Size::Hug { 0.0 } else { least })
            }
            _ => [0.0; 2],
        }
    }

    /// Moves the layer `id` to where it keeps its place on the spread once it is on
    /// the page of `to`.
    fn onto(&self, id: TreeID, to: TreeID) -> Res<()> {
        let places = self.places();
        let place = |n: TreeID| places.iter().find(|q| q.id == self.root(n));
        if let (Some(a), Some(b)) = (place(id), place(to))
            && a.spread == b.spread
            && a.x != b.x
        {
            let [x, y, w, h] = self.bounds(id);
            self.set_frame(id, [x + a.x - b.x, y, w, h])?;
        }
        Ok(())
    }

    /// Wraps `ids`, in document order, in a group or frame at the place of the topmost.
    fn group(&self, ids: &[TreeID], frame: bool) -> Res<TreeID> {
        let top = *ids.last().ok_or("nothing to group")?;
        let parent = self.tree.parent(top).ok_or("no parent")?;
        let index = self.index(top) + 1;
        for &id in ids {
            self.onto(id, top)?;
        }
        let bounds = union(ids.iter().map(|&id| self.bounds(id)));
        let g = self.tree.create_at(parent, index).map_err(err)?;
        let m = self.meta(g);
        if frame {
            m.insert(KIND, NodeKind::Frame.as_str()).map_err(err)?;
            m.insert("clip", true).map_err(err)?;
            self.set_frame(g, bounds)?;
        } else {
            m.insert(KIND, NodeKind::Group.as_str()).map_err(err)?;
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
        if self.kind(id) == Some(NodeKind::Group) {
            return union(self.children(id).into_iter().map(|c| self.bounds(c)));
        }
        let m = self.meta(id);
        ["x", "y", "w", "h"].map(|k| num(&m, k))
    }

    fn prune(&self, parent: Option<TreeParentId>) -> Res<()> {
        if let Some(TreeParentId::Node(p)) = parent
            && self.kind(p) == Some(NodeKind::Group)
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
            .find(|&r| self.kind(r) == Some(NodeKind::Trash))
        {
            Some(t) => t,
            None => {
                let t = self.tree.create(None).map_err(err)?;
                self.meta(t)
                    .insert(KIND, NodeKind::Trash.as_str())
                    .map_err(err)?;
                t
            }
        };
        self.tree.mov(id, trash).map_err(err)
    }

    /// The page or other root that `id` is on.
    fn root(&self, mut id: TreeID) -> TreeID {
        while let Some(TreeParentId::Node(p)) = self.tree.parent(id) {
            id = p;
        }
        id
    }

    fn node(&self, id: &str) -> Res<TreeID> {
        let t = TreeID::try_from(id).map_err(err)?;
        let mut root = t;
        while self.tree.contains(root) && self.tree.is_node_deleted(&root) == Ok(false) {
            match self.tree.parent(root) {
                Some(TreeParentId::Node(p)) => root = p,
                _ if matches!(self.kind(root), Some(NodeKind::Page | NodeKind::Master)) => {
                    return Ok(t);
                }
                _ => break,
            }
        }
        Err(format!("no node {id}"))
    }

    /// A node that is not a page or master.
    fn layer(&self, id: &str) -> Res<TreeID> {
        let n = self.node(id)?;
        match self.tree.parent(n) {
            Some(TreeParentId::Node(_)) => Ok(n),
            _ => Err(format!("{id} is not a layer")),
        }
    }

    /// A node that holds layers.
    fn container(&self, id: &str) -> Res<TreeID> {
        let n = self.node(id)?;
        match self.kind(n) {
            Some(NodeKind::Page | NodeKind::Master | NodeKind::Group | NodeKind::Frame) => Ok(n),
            _ => Err("not a container".into()),
        }
    }

    fn nodes(&self, ids: &[String]) -> Res<Vec<TreeID>> {
        ids.iter().map(|id| self.layer(id)).collect()
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

    fn name(&self, id: TreeID) -> String {
        value(&self.meta(id), "name")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    fn kind(&self, id: TreeID) -> Option<NodeKind> {
        let v = value(&self.meta(id), KIND)?.into_string().ok()?;
        NodeKind::ALL.into_iter().find(|k| k.as_str() == *v)
    }

    fn place_image(
        &self,
        parent: String,
        image: String,
        name: String,
        frame: [f64; 4],
    ) -> Res<Vec<String>> {
        self.image(&image)?;
        let id = self.create(&parent, NewKind::Rect, frame)?;
        self.set(
            id,
            Props {
                name: Some(name),
                fills: Some(vec![Fill::image(&image)]),
                ..Props::default()
            },
        )?;
        Ok(vec![id.to_string()])
    }

    fn reframe(
        &self,
        id: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        ignore_constraints: bool,
    ) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        check_frame([x, y, w, h])?;
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
        if self.kind(id) == Some(NodeKind::Text) && sizing.vertical != Size::Hug {
            sizing.horizontal = match sizing.horizontal {
                Size::Hug => Size::Fixed,
                s => s,
            };
        }
        if sizing != old {
            self.meta(id).insert("sizing", loro(sizing)?).map_err(err)?;
        }
        self.resize(id, [x, y, w, h], !ignore_constraints)?;
        Ok(vec![])
    }

    fn set_props(&self, id: String, mut props: Props) -> Res<Vec<String>> {
        let id = self.node(&id)?;
        if let Some(s) = props.sizing {
            let threaded = self.next_of(id).is_some() || self.prev_of(id).is_some();
            if threaded && s.horizontal == Size::Hug {
                return Err("a threaded text frame cannot be auto width".into());
            }
            if self.next_of(id).is_some() && s.vertical == Size::Hug {
                return Err("only the last frame of a thread can be auto height".into());
            }
        }
        let set = serde_json::to_value(&props).map_err(err)?;
        // Text that hugs its width hugs its height: choosing hug for the width
        // makes it auto width, a fixed height makes it fixed.
        if let Some(s) = props.sizing.as_mut().filter(|s| {
            self.kind(id) == Some(NodeKind::Text)
                && s.horizontal == Size::Hug
                && s.vertical != Size::Hug
        }) {
            if self.layout(id).sizing.horizontal == Size::Hug {
                s.horizontal = Size::Fixed;
            } else {
                s.vertical = Size::Hug;
            }
        }
        self.set(id, props)?;
        self.unbind(id, |p| !set[p].is_null())?;
        Ok(vec![])
    }

    fn set_path(&self, id: String, path: Vec<f32>) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        if !geom::valid(&path) {
            return Err("not a path".into());
        }
        self.set(
            id,
            Props {
                path: Some(fit(&path, [0.0, 0.0, 1.0, 1.0])),
                ..Props::default()
            },
        )?;
        self.set_frame(id, bounds(&path).map(f64::from))?;
        Ok(vec![])
    }

    fn delete(&self, ids: Vec<String>) -> Res<Vec<String>> {
        for id in self.nodes(&ids)? {
            let parent = self.tree.parent(id);
            self.unlink_all(id)?;
            self.remove(id)?;
            self.prune(parent)?;
        }
        Ok(vec![])
    }

    fn ungroup(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let mut out = Vec::new();
        for g in self.nodes(&ids)? {
            if !matches!(self.kind(g), Some(NodeKind::Group | NodeKind::Frame)) {
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
        Ok(out)
    }

    fn mask(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let ids = self.sorted(&ids)?;
        let lowest = *ids.first().ok_or("nothing to mask")?;
        Ok(if ids.len() == 1 {
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
        })
    }

    fn auto_layout(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let ids = self.sorted(&ids)?;
        let single = match ids[..] {
            [f] => {
                self.kind(f) == Some(NodeKind::Frame) && self.layout(f).direction == Direction::None
            }
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
        Ok(vec![f.to_string()])
    }

    fn move_layers(&self, ids: Vec<String>, parent: String, index: usize) -> Res<Vec<String>> {
        let p = self.container(&parent)?;
        let ids = self.sorted(&ids)?;
        let mut up = Some(p);
        while let Some(n) = up {
            if ids.contains(&n) {
                return Err("cannot move a layer into itself".into());
            }
            up = self.tree.parent(n).and_then(parent_node);
        }
        let olds: Vec<_> = ids.iter().map(|&id| self.tree.parent(id)).collect();
        for &id in &ids {
            self.onto(id, p)?;
        }
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
        Ok(vec![])
    }

    fn order(&self, ids: Vec<String>, to: Order) -> Res<Vec<String>> {
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
        Ok(vec![])
    }
}

impl Default for Doc {
    fn default() -> Self {
        Doc::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) use crate::display_list::{CUBIC, Paint};

    pub(super) fn page(d: &Doc) -> Page {
        d.build_snapshot().pages.remove(0)
    }

    pub(super) fn page_ops(d: &Doc) -> Vec<Op> {
        d.render(&page(d).id)
    }

    pub(super) fn hits(d: &Doc, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        d.hit(&page(d).id, x, y, tolerance)
    }

    pub(super) fn create(d: &mut Doc, parent: &str, kind: NewKind, frame: [f64; 4]) -> String {
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

    pub(super) fn empty() -> (Doc, String) {
        let mut d = Doc::new();
        let p = page(&d);
        let ids = p.children.iter().map(|n| n.id.clone()).collect();
        d.apply(Command::Delete { ids }).unwrap();
        (d, p.id)
    }

    pub(super) fn frame(n: &Node) -> [f64; 4] {
        [n.x, n.y, n.w, n.h]
    }

    pub(super) fn ids(nodes: &[Node]) -> Vec<String> {
        nodes.iter().map(|n| n.id.clone()).collect()
    }

    pub(super) fn children(n: &Node) -> &[Node] {
        match &n.kind {
            Kind::Group { children } | Kind::Frame { children, .. } => children,
            _ => &[],
        }
    }

    pub(super) fn set(d: &mut Doc, id: &str, props: Props) {
        d.apply(Command::Set {
            id: id.into(),
            props,
        })
        .unwrap();
    }

    pub(super) fn ppi(raster_ppi: f64) -> Command {
        Command::SetDocument(Settings {
            raster_ppi: Some(raster_ppi),
            ..Settings::default()
        })
    }

    pub(super) fn cmyk(d: &mut Doc) {
        d.apply(Command::SetDocument(Settings {
            color_mode: Some(ColorMode::Cmyk),
            ..Settings::default()
        }))
        .unwrap();
    }

    pub(super) fn process(c: f32, m: f32, y: f32, k: f32) -> Color {
        Color::Cmyk {
            cmyk: [c, m, y, k],
            alpha: 1.0,
        }
    }

    pub(super) fn swatch(d: &mut Doc, name: &str, color: Color, spot: bool) -> Res<String> {
        d.apply(Command::AddSwatch {
            name: name.into(),
            color,
            spot,
        })
        .map(|mut ids| ids.remove(0))
    }

    pub(super) fn bound(id: &str, tint: f32) -> Color {
        Color::Swatch {
            swatch: id.into(),
            tint,
            alpha: 1.0,
        }
    }

    pub(super) fn solid_colors(d: &Doc) -> Vec<[f32; 4]> {
        page_ops(d)
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

    pub(super) fn near_rgba(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05)
    }

    pub(super) fn collection(d: &mut Doc, name: &str) -> (String, String) {
        let ids = d
            .apply(Command::AddCollection { name: name.into() })
            .unwrap();
        (ids[0].clone(), ids[1].clone())
    }

    pub(super) fn variable(d: &mut Doc, collection: &str, name: &str, value: Value) -> Res<String> {
        d.apply(Command::AddVariable {
            collection: collection.into(),
            name: name.into(),
            value,
        })
        .map(|mut ids| ids.remove(0))
    }

    pub(super) fn set_value(d: &mut Doc, id: &str, mode: &str, value: Value) -> Res<Vec<String>> {
        d.apply(Command::SetVariable {
            id: id.into(),
            name: None,
            mode: Some(mode.into()),
            value: Some(value),
        })
    }

    pub(super) fn use_mode(d: &mut Doc, id: &str, collection: &str, mode: Option<&str>) {
        d.apply(Command::UseMode {
            id: id.into(),
            collection: collection.into(),
            mode: mode.map(String::from),
        })
        .unwrap();
    }

    pub(super) fn var(id: &str) -> Color {
        Color::Variable {
            variable: id.into(),
            alpha: 1.0,
        }
    }

    pub(super) fn bind(
        d: &mut Doc,
        id: &str,
        prop: &str,
        variable: Option<&str>,
    ) -> Res<Vec<String>> {
        d.apply(Command::Bind {
            id: id.into(),
            prop: prop.into(),
            variable: variable.map(String::from),
        })
    }

    pub(super) fn auto(d: &mut Doc, id: &str, props: Props) {
        set(
            d,
            id,
            Props {
                direction: Some(Direction::Horizontal),
                ..props
            },
        );
    }

    pub(super) fn frames(d: &Doc, id: &str) -> Vec<[f64; 4]> {
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

    pub(super) fn sizing(horizontal: Size, vertical: Size) -> Option<Sizing> {
        Some(Sizing {
            horizontal,
            vertical,
        })
    }

    pub(super) fn text(d: &mut Doc, content: &str) -> String {
        let p = page(d).id;
        let t = create(d, &p, NewKind::Text, [0.0, 0.0, 400.0, 400.0]);
        d.apply(Command::SetText {
            id: t.clone(),
            text: content.into(),
        })
        .unwrap();
        t
    }

    pub(super) fn format(
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

    pub(super) fn spans(d: &Doc) -> Vec<Span> {
        match page(d).children.pop().unwrap().kind {
            Kind::Text { content, .. } => content.spans.clone(),
            k => panic!("not text: {k:?}"),
        }
    }

    pub(super) fn lens_and<T>(d: &Doc, f: impl Fn(&Attrs) -> T) -> Vec<(usize, T)> {
        spans(d).iter().map(|s| (s.len, f(&s.attrs))).collect()
    }

    pub(super) fn sized(size: f64) -> TextProps {
        TextProps {
            size: Some(size),
            ..TextProps::default()
        }
    }

    pub(super) fn style(d: &mut Doc, name: &str, size: f64) -> String {
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

    pub(super) fn first_glyph(d: &Doc) -> [f32; 2] {
        page_ops(d)
            .into_iter()
            .find_map(|op| match op {
                Op::GlyphRun { positions, .. } => Some([positions[0], positions[1]]),
                _ => None,
            })
            .unwrap()
    }

    /// Auto line height of the bundled font at 12 pt.
    pub(super) const LEADING: f64 = 16.45;

    pub(super) fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.05
    }

    pub(super) fn set_frame(d: &mut Doc, id: &str, [x, y, w, h]: [f64; 4]) {
        d.apply(Command::SetFrame {
            id: id.into(),
            x,
            y,
            w,
            h,
            ignore_constraints: false,
        })
        .unwrap();
    }

    pub(super) fn set_text(d: &mut Doc, id: &str, text: &str) {
        d.apply(Command::SetText {
            id: id.into(),
            text: text.into(),
        })
        .unwrap();
    }

    pub(super) fn node_sizing(d: &Doc, id: &str) -> Sizing {
        fn find(nodes: &[Node], id: &str) -> Option<Sizing> {
            nodes.iter().find_map(|n| {
                (n.id == id)
                    .then_some(n.layout.sizing)
                    .or_else(|| find(children(n), id))
            })
        }
        find(&page(d).children, id).unwrap()
    }

    pub(super) fn edit(d: &mut Doc, id: &str, range: [usize; 2], text: &str) -> Res<Vec<String>> {
        d.apply(Command::EditText {
            id: id.into(),
            range,
            text: text.into(),
        })
    }

    pub(super) fn add_page(d: &mut Doc, after: Option<&str>) -> String {
        d.apply(Command::AddPage {
            after: after.map(String::from),
        })
        .unwrap()
        .remove(0)
    }

    pub(super) fn page_ids(d: &Doc) -> Vec<String> {
        d.build_snapshot().pages.into_iter().map(|p| p.id).collect()
    }

    pub(super) fn facing(d: &mut Doc, on: bool) {
        d.apply(Command::SetDocument(Settings {
            facing_pages: Some(on),
            ..Settings::default()
        }))
        .unwrap();
    }

    pub(super) fn sides(d: &Doc) -> Vec<Option<Side>> {
        d.snapshot().pages.iter().map(|p| p.side).collect()
    }

    pub(super) fn set_width(d: &mut Doc, id: &str, width: f64) {
        d.apply(Command::SetPage {
            id: id.into(),
            width: Some(width),
            height: None,
            bleed: None,
            scale: false,
        })
        .unwrap();
    }

    /// The least and greatest x of each path filled in `ops`.
    pub(super) fn filled_x(ops: &[Op]) -> Vec<[f32; 2]> {
        ops.iter()
            .filter_map(|o| match o {
                Op::FillPath { path, .. } => {
                    let mut xs = Vec::new();
                    let mut i = 0;
                    while i < path.len() {
                        let n = match path[i] {
                            CLOSE => 0,
                            CUBIC => 3,
                            _ => 1,
                        };
                        xs.extend((0..n).map(|k| path[i + 1 + 2 * k]));
                        i += 1 + 2 * n;
                    }
                    let min = xs.iter().copied().fold(f32::MAX, f32::min);
                    Some([min, xs.iter().copied().fold(f32::MIN, f32::max)])
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn add_master(d: &mut Doc) -> String {
        d.apply(Command::AddMaster { like: None })
            .unwrap()
            .remove(0)
    }

    pub(super) fn use_master(d: &mut Doc, page: &str, master: Option<&str>) -> Res<Vec<String>> {
        d.apply(Command::UseMaster {
            page: page.into(),
            master: master.map(String::from),
        })
    }

    pub(super) fn items(ops: &[Op]) -> Vec<u32> {
        ops.iter()
            .filter_map(|o| match o {
                Op::BeginItem { item } => Some(*item),
                _ => None,
            })
            .collect()
    }

    /// The x range of each clip pushed in `ops`.
    pub(super) fn clips_x(ops: &[Op]) -> Vec<[f32; 2]> {
        let clips = ops.iter().filter_map(|o| match o {
            Op::PushClip { path, .. } => Some(Op::FillPath {
                paint: Paint::Solid {
                    color: [0.0; 4],
                    ink: Ink::Rgb,
                },
                path: path.clone(),
            }),
            _ => None,
        });
        filled_x(&clips.collect::<Vec<_>>())
    }

    /// A master 100 pt wide with a layer on its left and one on its right page, used by
    /// a right page 1 and a left page 2 of its width.
    pub(super) fn master_spread() -> (Doc, [String; 5]) {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let m = add_master(&mut d);
        for p in [&p1, &p2, &m] {
            set_width(&mut d, p, 100.0);
        }
        let left = create(&mut d, &m, NewKind::Rect, [-90.0, 0.0, 10.0, 10.0]);
        let right = create(&mut d, &m, NewKind::Ellipse, [10.0, 0.0, 10.0, 10.0]);
        for p in [&p1, &p2] {
            use_master(&mut d, p, Some(&m)).unwrap();
        }
        (d, [p1, p2, m, left, right])
    }

    /// The text of the glyph runs of `ops`.
    pub(super) fn run_texts(ops: &[Op]) -> Vec<String> {
        ops.iter()
            .filter_map(|o| match o {
                Op::GlyphRun { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// A fixed text frame on `page` at `frame`.
    pub(super) fn fixed_text(d: &mut Doc, page: &str, frame: [f64; 4]) -> String {
        let t = create(d, page, NewKind::Text, [frame[0], frame[1], 0.0, 0.0]);
        set_frame(d, &t, frame);
        t
    }

    pub(super) fn thread(d: &mut Doc, from: &str, to: &str) -> Res<Vec<String>> {
        d.apply(Command::Thread {
            from: from.into(),
            to: to.into(),
        })
    }

    /// The story, start, end, previous and next frame, and overset of a text layer.
    pub(super) fn flow(
        d: &Doc,
        id: &str,
    ) -> (String, usize, usize, Option<String>, Option<String>, bool) {
        let s = d.snapshot();
        let all: Vec<&Node> = s
            .pages
            .iter()
            .chain(&s.masters)
            .flat_map(|p| &p.children)
            .collect();
        let n = all.into_iter().find(|n| n.id == id).unwrap();
        match &n.kind {
            Kind::Text {
                content,
                story,
                start,
                end,
                prev,
                next,
                overset,
                ..
            } => {
                assert!(story == id || prev.is_some(), "{story} heads {id}");
                (
                    content.text.clone(),
                    *start,
                    *end,
                    prev.clone(),
                    next.clone(),
                    *overset,
                )
            }
            k => panic!("not text: {k:?}"),
        }
    }

    /// Page 1 with frame `a` of two lines holding "Hi\nHi\nHi\nHi", threaded into the
    /// large frame `b` on page 2: (doc, page 1, page 2, a, b).
    pub(super) fn two_pages() -> (Doc, String, String, String, String) {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 2.0 * LEADING + 1.0]);
        let b = fixed_text(&mut d, &p2, [10.0, 10.0, 100.0, 300.0]);
        set_text(&mut d, &a, "Hi\nHi\nHi\nHi");
        thread(&mut d, &a, &b).unwrap();
        (d, p1, p2, a, b)
    }

    /// The M1 booklet: 8 facing A5 pages on a master spread with a page number at the
    /// outer foot of each side, one story of 41 k characters threaded through a frame
    /// on every page, coloured by a variable that each page shows in a mode of its own.
    pub(super) struct Booklet {
        d: Doc,
        pages: Vec<String>,
        master: String,
        frames: Vec<String>,
        story: String,
        /// The story's colour on each page.
        inks: Vec<[f32; 4]>,
    }

    pub(super) fn booklet() -> Booklet {
        let (mut d, first) = empty();
        let mut pages = vec![first];
        for _ in 1..8 {
            pages.push(add_page(&mut d, None));
        }
        let (w, h) = (148.0 * MM, 210.0 * MM);
        let master = add_master(&mut d);
        for x in [-w + 12.0 * MM, w - 15.0 * MM] {
            let t = create(&mut d, &master, NewKind::Text, [x, h - 12.0 * MM, 0.0, 0.0]);
            set_text(&mut d, &t, &text::PAGE_NUMBER.to_string());
        }
        let (c, first_mode) = collection(&mut d, "Tone");
        let mut modes = vec![first_mode];
        for i in 1..8 {
            let name = format!("Page {}", i + 1);
            let add = Command::AddMode {
                collection: c.clone(),
                name,
            };
            modes.push(d.apply(add).unwrap().remove(0));
        }
        let rgb: Vec<u32> = (0..8).map(|i| (i * 30) << 24 | 0xff).collect();
        let ink = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(rgb[0]))).unwrap();
        let mut frames = Vec::new();
        for ((p, m), &c0) in pages.iter().zip(&modes).zip(&rgb) {
            set_value(&mut d, &ink, m, Value::Color(Color::Rgb(c0))).unwrap();
            use_master(&mut d, p, Some(&master)).unwrap();
            use_mode(&mut d, p, &c, Some(m));
            let frame = [15.0 * MM, 15.0 * MM, w - 30.0 * MM, h - 35.0 * MM];
            frames.push(fixed_text(&mut d, p, frame));
        }
        let story = (1..=143)
            .map(|i| format!("{i}. {SAMPLE}"))
            .collect::<Vec<_>>()
            .join("\n");
        set_text(&mut d, &frames[0], &story);
        let props = TextProps {
            fill: Some(var(&ink)),
            hyphenate: Some(false),
            ..TextProps::default()
        };
        format(&mut d, &frames[0], Some([0, story.len()]), props).unwrap();
        for f in frames.windows(2) {
            thread(&mut d, &f[0], &f[1]).unwrap();
        }
        let inks = rgb
            .iter()
            .map(|c| Color::Rgb(*c).rgba(&scope_none()))
            .collect();
        Booklet {
            d,
            pages,
            master,
            frames,
            story,
            inks,
        }
    }

    pub(super) fn scope_none() -> Scope<'static> {
        static PALETTE: std::sync::LazyLock<Palette> = std::sync::LazyLock::new(Palette::default);
        static MODES: std::sync::LazyLock<Modes> = std::sync::LazyLock::new(Modes::new);
        Scope {
            palette: &PALETTE,
            modes: &MODES,
        }
    }

    pub(super) const MONO: &[u8] = include_bytes!("../../fonts/DMMono-Regular.ttf");

    pub(super) fn runs(d: &Doc) -> Vec<(u32, Vec<u16>, Vec<f32>)> {
        page_ops(d)
            .into_iter()
            .filter_map(|op| match op {
                Op::GlyphRun {
                    font,
                    glyphs,
                    positions,
                    ..
                } => Some((font, glyphs, positions)),
                _ => None,
            })
            .collect()
    }

    pub(super) fn in_font(face: Typeface) -> TextProps {
        TextProps {
            font: Some(face),
            ..TextProps::default()
        }
    }

    pub(super) fn problems(d: &Doc) -> Vec<(String, Problem)> {
        let issues = d.build_snapshot().preflight.into_iter();
        issues.map(|i| (i.layer, i.problem)).collect()
    }

    pub(super) fn place(d: &mut Doc, page: &str, w: u32, h: u32) -> (String, String) {
        let hash = d.add_image(&image::tests::png(w, h)).unwrap().hash;
        let id = d
            .apply(Command::PlaceImage {
                parent: page.into(),
                image: hash.clone(),
                name: "photo.png".into(),
                x: 28.0,
                y: 164.0,
                w: f64::from(w) * 72.0 / 300.0,
                h: f64::from(h) * 72.0 / 300.0,
            })
            .unwrap()
            .remove(0);
        (id, hash)
    }

    /// The text layers of the pages and masters with the UTF-16 length of their story.
    fn texts(d: &Doc) -> Vec<(String, usize)> {
        fn walk(nodes: &[Node], out: &mut Vec<(String, usize)>) {
            for n in nodes {
                match &n.kind {
                    Kind::Text { content, .. } => {
                        out.push((n.id.clone(), content.text.encode_utf16().count()))
                    }
                    Kind::Group { children } | Kind::Frame { children, .. } => walk(children, out),
                    Kind::Shape(_) => {}
                }
            }
        }
        let s = d.build_snapshot();
        let mut out = Vec::new();
        for p in s.pages.iter().chain(&s.masters) {
            walk(&p.children, &mut out);
        }
        out
    }

    /// `n` random commands that change text and what sets it, on two threaded pages
    /// with a master, a text style and a size variable in 2 modes; `check` after each.
    pub(super) fn random_commands(seed: u64, n: usize, check: impl Fn(&Doc)) {
        let (mut d, p1, p2, a, _) = two_pages();
        let master = add_master(&mut d);
        let number = create(&mut d, &master, NewKind::Text, [0.0, 500.0, 0.0, 0.0]);
        set_text(&mut d, &number, &text::PAGE_NUMBER.to_string());
        use_master(&mut d, &p2, Some(&master)).unwrap();
        let body = style(&mut d, "Body", 11.0);
        let (c, m1) = collection(&mut d, "Size");
        let m2 = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Large".into(),
            })
            .unwrap()
            .remove(0);
        let size = variable(&mut d, &c, "Size", Value::Number(10.0)).unwrap();
        set_value(&mut d, &size, &m2, Value::Number(16.0)).unwrap();
        bind(&mut d, &a, "size", Some(&size)).unwrap();
        let mut pages = vec![p1, p2];
        let mut r = seed;
        let mut rand = |n: usize| {
            r ^= r << 13;
            r ^= r >> 7;
            r ^= r << 17;
            (r % n as u64) as usize
        };
        check(&d);
        for _ in 0..n {
            let t = texts(&d);
            let (id, len) = t[rand(t.len())].clone();
            let (x, y) = (rand(len + 1), rand(len + 1));
            let range = [x.min(y), x.max(y)];
            let page = pages[rand(pages.len())].clone();
            let frame = [
                rand(200) as f64,
                rand(400) as f64,
                20.0 + rand(300) as f64,
                20.0 + rand(300) as f64,
            ];
            let cmd = match rand(14) {
                0 | 1 => Command::EditText {
                    id,
                    range,
                    text: ["X", "a b ", "\n", ""][rand(4)].into(),
                },
                2 => Command::Format {
                    id,
                    range: Some(range),
                    props: sized(6.0 + rand(20) as f64),
                },
                3 => Command::Format {
                    id,
                    range: Some(range),
                    props: TextProps {
                        text_style: Some(body.clone()),
                        ..TextProps::default()
                    },
                },
                4 => Command::SetFrame {
                    id,
                    x: frame[0],
                    y: frame[1],
                    w: frame[2],
                    h: frame[3],
                    ignore_constraints: false,
                },
                5 => Command::Thread {
                    from: id,
                    to: t[rand(t.len())].0.clone(),
                },
                6 => Command::Unthread { id },
                7 => Command::SetTextStyle {
                    id: body.clone(),
                    name: None,
                    size: Some(6.0 + rand(20) as f64),
                    line_height: None,
                    letter_spacing: None,
                    paragraph_spacing: None,
                },
                8 => {
                    let face = text::typeface(MONO).unwrap();
                    if rand(2) == 0 {
                        d.add_font(MONO).unwrap();
                        check(&d);
                    }
                    Command::Format {
                        id,
                        range: Some(range),
                        props: in_font(face),
                    }
                }
                9 => Command::UseMode {
                    id: page,
                    collection: c.clone(),
                    mode: [None, Some(m1.clone()), Some(m2.clone())][rand(3)].clone(),
                },
                10 => Command::DuplicatePage { id: page },
                11 => Command::SetPage {
                    id: page,
                    width: Some(200.0 + rand(400) as f64),
                    height: None,
                    bleed: None,
                    scale: false,
                },
                12 => Command::Undo,
                _ => Command::Redo,
            };
            if let Ok(ids) = d.apply(cmd) {
                pages.extend(ids.into_iter().filter(|i| d.page(i).is_ok()));
            }
            check(&d);
        }
    }

    #[test]
    fn the_kept_snapshot_is_the_one_built_afresh() {
        random_commands(1, 200, |d| assert!(*d.snapshot() == d.build_snapshot()));
    }

    #[test]
    fn a_blank_doc_has_empty_pages_of_the_size_and_nothing_to_undo() {
        let d = Doc::blank(100.0, 200.0, 3, true, ColorMode::Cmyk);
        let s = d.snapshot();
        assert_eq!(s.pages.len(), 3);
        assert!(s.facing_pages);
        assert_eq!(s.color_mode, ColorMode::Cmyk);
        assert!(
            s.pages
                .iter()
                .all(|p| p.children.is_empty() && p.width == 100.0 && p.height == 200.0)
        );
        assert!(!d.undo.can_undo());
    }

    #[test]
    fn default_doc_has_a_page_with_rect_text_and_clipped_frame() {
        let s = Doc::new().snapshot();
        assert_eq!(s.pages.len(), 1);
        let p = &s.pages[0];
        assert!((p.width - 419.53).abs() < 0.01);
        assert!((p.bleed - 8.50).abs() < 0.01);
        assert_eq!(
            p.children[0].kind,
            Kind::Shape(Shape::Rect {
                radius: 0.0,
                corners: vec![]
            })
        );
        assert_eq!(p.children[0].style.fills, [Fill::solid(0xe8452cff)]);
        assert!(
            matches!(&p.children[1].kind, Kind::Text { content, .. } if content.spans[0].attrs.size == 14.0)
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
        assert_eq!(
            n.kind,
            Kind::Shape(Shape::Rect {
                radius: 0.0,
                corners: vec![]
            })
        );
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
        let before = d.build_snapshot().pages;
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
        let grouped = d.build_snapshot().pages;
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
        assert_eq!(frame(&page(&d).children[0]), [-5.0, 0.0, 10.0, 0.01 * MM]);
    }

    #[test]
    fn a_command_that_fails_leaves_the_document_as_it_was() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let f = create(&mut d, &p, NewKind::Frame, [20.0, 0.0, 10.0, 10.0]);
        let t = create(&mut d, &p, NewKind::Text, [0.0, 40.0, 0.0, 0.0]);
        let (c, _) = collection(&mut d, "Numbers");
        let v = variable(&mut d, &c, "Half", Value::Number(50.0)).unwrap();
        bind(&mut d, &r, "opacity", Some(&v)).unwrap();
        let m = add_master(&mut d);
        let mr = create(&mut d, &m, NewKind::Rect, [0.0; 4]);
        use_master(&mut d, &p, Some(&m)).unwrap();
        let copy = d
            .apply(Command::Override {
                page: p.clone(),
                id: mr,
            })
            .unwrap()
            .remove(0);
        let set = |id: &str, props| Command::Set {
            id: id.into(),
            props,
        };
        let frame = |id: &str, [x, y, w, h]: [f64; 4]| Command::SetFrame {
            id: id.into(),
            x,
            y,
            w,
            h,
            ignore_constraints: false,
        };
        let new = |parent: &str, [x, y, w, h]: [f64; 4]| Command::Create {
            parent: parent.into(),
            kind: NewKind::Rect,
            x,
            y,
            w,
            h,
        };
        let path = |path: Vec<f32>| Command::SetPath {
            id: r.clone(),
            path,
        };
        let fill = |fill: Fill| {
            set(
                &r,
                Props {
                    fills: Some(vec![fill]),
                    ..Props::default()
                },
            )
        };
        let bad = [
            Command::SetDocument(Settings {
                raster_ppi: Some(150.0),
                color_mode: Some(ColorMode::Cmyk),
                ink_limit: Some(500.0),
                ..Settings::default()
            }),
            set(
                &r,
                Props {
                    opacity: Some(2.0),
                    ..Props::default()
                },
            ),
            set(
                &f,
                Props {
                    gap: Some(f64::NAN),
                    ..Props::default()
                },
            ),
            set(
                &r,
                Props {
                    effects: Some(vec![Effect {
                        x: f32::INFINITY,
                        ..Effect::default()
                    }]),
                    ..Props::default()
                },
            ),
            fill(Fill {
                transform: [f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0],
                ..Fill::default()
            }),
            fill(Fill {
                stops: vec![FillStop {
                    at: 2.0,
                    color: Color::Rgb(BLACK),
                }],
                ..Fill::default()
            }),
            new(&p, [0.0, 0.0, -1.0, 1.0]),
            new(&p, [f64::NAN, 0.0, 1.0, 1.0]),
            new(&r, [0.0, 0.0, 1.0, 1.0]),
            frame(&r, [f64::NAN, 0.0, 10.0, 10.0]),
            frame(&r, [0.0, f64::INFINITY, 10.0, 10.0]),
            frame(&p, [0.0, 0.0, 10.0, 10.0]),
            path(vec![LINE, 1.0, 1.0]),
            path(vec![MOVE, f32::NAN, 0.0]),
            path(vec![MOVE, 0.0]),
            Command::SetDocument(Settings {
                raster_ppi: Some(5000.0),
                facing_pages: Some(true),
                ..Settings::default()
            }),
            Command::SetPage {
                id: p.clone(),
                width: Some(100.0),
                height: Some(-1.0),
                bleed: None,
                scale: false,
            },
            Command::ScaleText {
                id: t.clone(),
                by: 0.0,
            },
            set(
                &r,
                Props {
                    corners: Some(vec![1.0, 2.0]),
                    ..Props::default()
                },
            ),
            set(
                &r,
                Props {
                    sweep: Some(2.0),
                    ..Props::default()
                },
            ),
            set(
                &r,
                Props {
                    corners: Some(vec![1.0, 2.0, f32::NAN, 0.0]),
                    ..Props::default()
                },
            ),
            Command::ScaleText {
                id: r.clone(),
                by: 2.0,
            },
            Command::ResetToMaster {
                ids: vec![copy, r.clone()],
            },
            Command::Delete {
                ids: vec![r.clone(), p.clone()],
            },
            Command::Format {
                id: t.clone(),
                range: None,
                props: sized(f64::NAN),
            },
            Command::AddTextStyle {
                name: "Body".into(),
                size: f64::INFINITY,
                line_height: 0.0,
                letter_spacing: 0.0,
                paragraph_spacing: 0.0,
            },
            Command::DeleteMode {
                collection: c.clone(),
                id: "none".into(),
            },
            Command::SetVariable {
                id: v.clone(),
                name: None,
                mode: None,
                value: Some(Value::Number(f64::NAN)),
            },
        ];
        let before = d.snapshot();
        for cmd in bad {
            let what = format!("{cmd:?}");
            assert!(d.apply(cmd).is_err(), "{what}");
            assert_eq!(d.doc.get_pending_txn_len(), 0, "{what}");
            assert!(d.snapshot() == before, "{what}");
        }
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

    #[test]
    fn the_booklet_numbers_each_page_at_its_outer_foot() {
        let b = booklet();
        for (i, p) in b.pages.iter().enumerate() {
            let number = (i + 1).to_string();
            let x =
                b.d.render(p)
                    .into_iter()
                    .find_map(|o| match o {
                        Op::GlyphRun {
                            text, positions, ..
                        } if text == number => Some(positions[0]),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("no number on page {number}"));
            let outer = if i % 2 == 0 {
                x > 100.0 * MM as f32
            } else {
                x < 50.0 * MM as f32
            };
            assert!(outer, "page {number} has its number at {x}");
        }
    }

    #[test]
    fn the_booklet_story_runs_on_through_every_page() {
        let b = booklet();
        let mut at = 0;
        for f in &b.frames {
            let (text, start, end, ..) = flow(&b.d, f);
            assert_eq!(text, b.story);
            assert_eq!(start, at);
            assert!(end > start + 1000, "{f} sets {start}..{end}");
            at = end;
        }
        assert!(flow(&b.d, b.frames.last().unwrap()).5);
    }

    #[test]
    fn each_booklet_page_colours_the_story_in_its_own_mode() {
        let b = booklet();
        for (p, ink) in b.pages.iter().zip(&b.inks) {
            let colors: Vec<[f32; 4]> =
                b.d.render(p)
                    .into_iter()
                    .filter_map(|o| match o {
                        Op::GlyphRun {
                            paint: Paint::Solid { color, .. },
                            text,
                            ..
                        } if text.len() > 3 => Some(color),
                        _ => None,
                    })
                    .collect();
            assert!(!colors.is_empty());
            assert!(colors.iter().all(|c| c == ink), "{p}: {colors:?}");
        }
    }

    #[test]
    fn deleting_the_page_with_the_head_of_the_booklet_story_hands_it_to_the_next_frame() {
        let mut b = booklet();
        b.d.apply(Command::DeletePage {
            id: b.pages[0].clone(),
        })
        .unwrap();
        let (text, start, _, prev, next, _) = flow(&b.d, &b.frames[1]);
        assert_eq!((text, start, prev), (b.story.clone(), 0, None));
        assert_eq!(next.as_ref(), Some(&b.frames[2]));
        b.d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&b.d, &b.frames[1]).3.as_ref(), Some(&b.frames[0]));
        assert_eq!(flow(&b.d, &b.frames[0]).0, b.story);
    }

    #[test]
    fn deleting_a_master_whose_frame_heads_a_thread_takes_it_off_the_pages_until_undone() {
        let mut b = booklet();
        let left = fixed_text(&mut b.d, &b.master, [-100.0, 20.0, 60.0, LEADING + 1.0]);
        let right = fixed_text(&mut b.d, &b.master, [40.0, 20.0, 60.0, 300.0]);
        set_text(&mut b.d, &left, "Running\nhead");
        thread(&mut b.d, &left, &right).unwrap();
        b.d.apply(Command::DeleteMaster {
            id: b.master.clone(),
        })
        .unwrap();
        let s = b.d.snapshot();
        assert!(s.masters.is_empty());
        assert!(s.pages.iter().all(|p| p.master.is_none()));
        assert!(!run_texts(&b.d.render(&b.pages[1])).contains(&"2".to_string()));
        b.d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&b.d, &right).3, Some(left));
        assert!(run_texts(&b.d.render(&b.pages[1])).contains(&"2".to_string()));
    }

    #[test]
    fn the_booklet_pdf_reads_as_one_story_with_a_number_on_every_page() {
        let b = booklet();
        let path = std::env::temp_dir().join(format!("satz-booklet-{}.pdf", std::process::id()));
        std::fs::write(&path, b.d.pdf("Satz", "2026-09-28T12:00:00Z")).unwrap();
        let mut read = String::new();
        for i in 1..=8 {
            let out = std::process::Command::new("mutool")
                .args(["draw", "-q", "-F", "text", "-o", "-"])
                .arg(&path)
                .arg(i.to_string())
                .output()
                .expect("mutool runs");
            let text = String::from_utf8(out.stdout).unwrap();
            let lines: Vec<&str> = text.lines().map(str::trim).collect();
            assert!(
                lines.contains(&i.to_string().as_str()),
                "page {i}: {lines:?}"
            );
            let own: Vec<&str> = lines.into_iter().filter(|l| *l != i.to_string()).collect();
            read.push_str(&own.join(" "));
            read.push(' ');
        }
        std::fs::remove_file(&path).unwrap();
        let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let (read, story) = (words(&read), words(&b.story));
        assert!(read.len() > 10_000, "{}", read.len());
        assert!(story.starts_with(&read), "{}", &read[..200]);
    }

    #[test]
    fn a_key_typed_into_the_booklet_story_is_set_and_drawn_within_budget() {
        let mut b = booklet();
        let (d, frame) = (&mut b.d, &b.frames[4]);
        let at = flow(d, frame).1 + 10;
        let spread = [&b.pages[3], &b.pages[4]];
        let start = std::time::Instant::now();
        edit(d, frame, [at, at], "X").unwrap();
        d.snapshot();
        let shown = d.text_frame(frame, at + 1).unwrap();
        for p in spread {
            d.render(p);
            d.text_overlay(&shown, at + 1, at + 1, 1.0, p).unwrap();
        }
        d.caret(&shown, at + 1).unwrap();
        let took = start.elapsed();
        eprintln!("typing took {took:?}");
        assert!(took.as_millis() < 1000, "typing took {took:?}");
    }

    #[test]
    fn a_saved_booklet_loads_as_it_was_without_undo_history() {
        let mut b = booklet();
        b.d.apply(Command::Delete {
            ids: vec![b.frames[7].clone()],
        })
        .unwrap();
        let mut loaded = Doc::load(&b.d.save()).unwrap();
        let (before, after) = (b.d.build_snapshot(), loaded.snapshot());
        assert!(!after.can_undo && !after.can_redo);
        assert_eq!(
            Snapshot {
                can_undo: false,
                ..before
            },
            *after
        );
        for p in &b.pages {
            assert_eq!(b.d.render(p), loaded.render(p));
        }
        loaded
            .apply(Command::SetText {
                id: b.frames[0].clone(),
                text: "Hi".into(),
            })
            .unwrap();
        loaded.apply(Command::Undo).unwrap();
        assert_eq!(loaded.snapshot().stories[&b.frames[0]].text, b.story);
    }

    #[test]
    fn a_file_that_is_not_a_satz_document_does_not_load() {
        assert!(Doc::load(b"not a satz file").is_err());
        let other = LoroDoc::new();
        other.get_map("x").insert("a", 1).unwrap();
        assert!(Doc::load(&other.export(ExportMode::Snapshot).unwrap()).is_err());
    }

    #[test]
    fn a_placed_image_fills_its_rectangle() {
        let (mut d, p) = empty();
        let (id, hash) = place(&mut d, &p, 600, 300);
        let n = &page(&d).children[0];
        assert_eq!((n.id.as_str(), n.name.as_str()), (id.as_str(), "photo.png"));
        assert_eq!(frame(n), [28.0, 164.0, 144.0, 72.0]);
        assert_eq!(n.style.fills, [Fill::image(&hash)]);
        assert!(close(n.ppi.unwrap(), 300.0));
        assert_eq!(d.build_snapshot().images[&hash], image::Space::Rgb);
        let image = image::id(&hash).unwrap();
        assert!(page_ops(&d).contains(&Op::Image {
            image,
            transform: [144.0, 0.0, 0.0, 72.0, 28.0, 164.0],
        }));
    }

    #[test]
    fn an_image_enlarged_below_300_ppi_is_a_preflight_issue_while_it_shows() {
        let (mut d, p) = empty();
        let (id, hash) = place(&mut d, &p, 600, 300);
        set_frame(&mut d, &id, [20.0, 100.0, 288.0, 72.0]);
        assert_eq!(problems(&d), [(id.clone(), Problem::LowPpi { ppi: 150.0 })]);
        cmyk(&mut d);
        let proofed = d.plate(&p, false).iter().any(
            |o| matches!(o, Op::Image { image, .. } if *image == image::id(&hash).unwrap() | image::CMY),
        );
        assert!(proofed);
        assert_eq!(
            problems(&d),
            [
                (id.clone(), Problem::Rgb),
                (id.clone(), Problem::LowPpi { ppi: 150.0 })
            ]
        );
        set(
            &mut d,
            &id,
            Props {
                fills: Some(vec![Fill {
                    visible: false,
                    ..Fill::image(&hash)
                }]),
                ..Props::default()
            },
        );
        assert_eq!(page(&d).children[0].ppi, None);
        assert_eq!(problems(&d), []);
    }

    #[test]
    fn images_go_into_the_saved_document_and_undo_leaves_them_there() {
        let (mut d, p) = empty();
        let (_, hash) = place(&mut d, &p, 60, 30);
        d.apply(Command::Undo).unwrap();
        assert!(page(&d).children.is_empty());
        assert!(d.image(&hash).is_ok());
        d.apply(Command::Redo).unwrap();
        let saved = d.save();
        let ppi = std::thread::spawn(move || {
            let d = Doc::load(&saved).unwrap();
            page(&d).children[0].ppi
        });
        assert!(close(ppi.join().unwrap().unwrap(), 300.0));
    }

    #[test]
    fn a_saved_document_leaves_out_the_images_no_layer_uses() {
        let (mut d, p) = empty();
        let (kept, a) = place(&mut d, &p, 60, 30);
        let (gone, b) = place(&mut d, &p, 40, 30);
        d.apply(Command::Delete { ids: vec![gone] }).unwrap();
        let saved = d.save();
        assert!(d.image(&b).is_ok());
        let loaded = std::thread::spawn(move || {
            let d = Doc::load(&saved).unwrap();
            (
                d.image(&a).is_ok(),
                d.image(&b).is_ok(),
                page(&d).children[0].id.clone(),
            )
        });
        assert_eq!(loaded.join().unwrap(), (true, false, kept));
    }

    #[test]
    fn image_fills_need_an_image_of_the_document_and_a_layer_other_than_text() {
        let (mut d, p) = empty();
        let (id, hash) = place(&mut d, &p, 6, 3);
        let mut apply = |id: &str, props| {
            d.apply(Command::Set {
                id: id.into(),
                props,
            })
        };
        let fills = |f| Props {
            fills: Some(vec![f]),
            ..Props::default()
        };
        assert!(apply(&id, fills(Fill::image("0123456789abcdef"))).is_err());
        let strokes = Props {
            strokes: Some(vec![Fill::image(&hash)]),
            ..Props::default()
        };
        assert!(apply(&id, strokes).is_err());
        let t = text(&mut d, "Hi");
        let mut apply = |id: &str, props| {
            d.apply(Command::Set {
                id: id.into(),
                props,
            })
        };
        assert!(apply(&t, fills(Fill::image(&hash))).is_err());
        assert!(d.add_image(b"GIF89a").is_err());
        let unknown = Command::PlaceImage {
            parent: p,
            image: "0123456789abcdef".into(),
            name: "gone.png".into(),
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        };
        assert!(d.apply(unknown).is_err());
    }
}
