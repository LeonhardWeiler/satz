use crate::color::{self, Ink};
use crate::color::{Color, ColorMode, Swatch};
use crate::display_list::{LINE, MOVE, Op, Paint, recolor, rect, shift};
use crate::geom::{self, LineStyle, Shape, bounds, closed, contains, fit, near, outline};
use crate::image::{self, ImageInfo};
use crate::layout::{Align3, Direction, Layout, MainAlign, Size, Sizing, arrange};
use crate::style::{
    Align, Blend, Cap, Constraint, Constraints, Effect, EffectKind, Fill, FillKind, FillStop, Join,
    Style,
};
use crate::text::{
    self, Attrs, Grep, Lang, PARAGRAPH, Position, STYLED, Span, TextAlign, TextCase,
    TextDecoration, TextFrame, TextStyle, Typeface, VerticalAlign, Wrap,
};
use crate::variable::{Collection, Mode, Modes, Palette, Scope, Value, Variable};
pub use boolean::BooleanOp;
#[cfg(feature = "io")]
pub(crate) use boolean::path as path_of;
use harfrust::Feature;
use loro::event::{ContainerDiff, Diff};
use loro::{
    Container, ContainerID, ExpandType, ExportMode, Index, LoroBinaryValue, LoroDoc, LoroMap,
    LoroText, LoroTree, LoroValue, StyleConfig, StyleConfigMap, Subscription, TextDelta,
    TreeExternalDiff, TreeID, TreeParentId, UndoManager, UpdateOptions, ValueOrContainer,
    VersionVector,
};
use serde::de::value::{Error as DeError, MapAccessDeserializer, MapDeserializer, SeqDeserializer};
use serde::de::{DeserializeOwned, Deserializer, IntoDeserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod anchor;
#[cfg(test)]
mod bench;
mod boolean;
mod clipboard;
mod draw;
mod history;
mod link;
mod pages;
mod palette;
mod story;
mod table;
mod toc;

pub use anchor::Anchoring;
use anchor::*;
use clipboard::*;
pub use draw::*;
use pages::*;
use story::*;
pub use table::Table;
use table::*;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Screen,
    X4,
    X1a,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(test, ts(optional_fields))]
pub struct Settings {
    pub raster_ppi: Option<f64>,
    pub color_mode: Option<ColorMode>,
    pub facing_pages: Option<bool>,
    pub ink_limit: Option<f64>,
    pub preset: Option<Preset>,
    pub crop_marks: Option<bool>,
    pub include_bleed: Option<bool>,
    pub image_ppi: Option<f64>,
    pub jpeg_quality: Option<u8>,
    pub date_format: Option<DateFormat>,
    pub link_images: Option<bool>,
}

#[allow(clippy::large_enum_variant)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(test, ts(optional_fields = nullable))]
pub enum Command {
    Create {
        parent: String,
        kind: NewKind,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    PlaceImage {
        parent: String,
        image: String,
        name: String,
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
        #[serde(default)]
        #[cfg_attr(test, ts(as = "Option<bool>", optional))]
        crop: bool,
    },
    SetText {
        id: String,
        text: String,
    },
    EditText {
        id: String,
        range: [usize; 2],
        text: String,
    },
    Format {
        id: String,
        range: Option<[usize; 2]>,
        #[serde(flatten)]
        props: TextProps,
    },
    ScaleText {
        id: String,
        by: f64,
    },
    AddTextStyle {
        name: String,
        #[serde(flatten)]
        props: TextProps,
    },
    SetTextStyle {
        id: String,
        name: Option<String>,
        grep: Option<Vec<Grep>>,
        #[serde(flatten)]
        props: TextProps,
    },
    DeleteTextStyle {
        id: String,
    },
    Toc {
        id: String,
        styles: Vec<String>,
    },
    Anchor {
        id: String,
        text: String,
        at: usize,
    },
    Unanchor {
        id: String,
    },
    TableInsert {
        id: String,
        column: bool,
        at: usize,
    },
    TableDelete {
        id: String,
        column: bool,
        at: usize,
    },
    MergeCells {
        ids: Vec<String>,
    },
    SplitCell {
        id: String,
    },
    SizeTable {
        id: String,
        columns: Vec<f64>,
        rows: Vec<f64>,
    },
    Set {
        id: String,
        #[serde(flatten)]
        props: Props,
    },
    SetPath {
        id: String,
        path: Vec<f32>,
    },
    FitImage {
        id: String,
        fit: Fit,
    },
    Flip {
        id: String,
        vertical: bool,
    },
    Flatten {
        id: String,
    },
    Boolean {
        ids: Vec<String>,
        op: BooleanOp,
    },
    FillArea {
        id: String,
        x: f64,
        y: f64,
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
    Mask {
        ids: Vec<String>,
    },
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
    Relink {
        image: String,
        to: String,
        name: String,
    },
    Embed {
        images: Vec<String>,
    },
    Paste {
        above: Vec<String>,
        #[serde(default)]
        page: Option<String>,
    },
    AddPage {
        after: Option<String>,
    },
    SetSection {
        id: String,
        section: Option<Section>,
    },
    DuplicatePage {
        id: String,
    },
    SetPage {
        id: String,
        width: Option<f64>,
        height: Option<f64>,
        bleed: Option<f64>,
        #[serde(default)]
        #[cfg_attr(test, ts(as = "Option<bool>", optional))]
        scale: bool,
    },
    SetGrids {
        id: String,
        grids: Vec<Grid>,
    },
    SetGuides {
        id: String,
        guides: Guides,
    },
    DeletePage {
        id: String,
    },
    MovePages {
        ids: Vec<String>,
        index: usize,
    },
    AddMaster {
        like: Option<String>,
        #[serde(default)]
        #[cfg_attr(test, ts(as = "Option<bool>", optional))]
        layers: bool,
    },
    SetMaster {
        id: String,
        name: String,
    },
    DeleteMaster {
        id: String,
    },
    UseMaster {
        page: String,
        master: Option<String>,
    },
    Override {
        page: String,
        id: String,
    },
    ResetToMaster {
        ids: Vec<String>,
    },
    Thread {
        from: String,
        to: String,
    },
    ThreadBefore {
        id: String,
        to: String,
    },
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
    DeleteSwatch {
        id: String,
    },
    AddCollection {
        name: String,
    },
    SetCollection {
        id: String,
        name: String,
    },
    DeleteCollection {
        id: String,
    },
    SetMode {
        collection: String,
        id: String,
        name: String,
    },
    DeleteMode {
        collection: String,
        id: String,
    },
    DeleteVariable {
        id: String,
    },
    AddMode {
        collection: String,
        name: String,
    },
    AddVariable {
        collection: String,
        name: String,
        value: Value,
    },
    SetVariable {
        id: String,
        name: Option<String>,
        mode: Option<String>,
        value: Option<Value>,
    },
    Bind {
        id: String,
        prop: String,
        variable: Option<String>,
    },
    UseMode {
        id: String,
        collection: String,
        mode: Option<String>,
    },
    Restore {
        peer: String,
        counter: i32,
    },
    Undo,
    Redo,
    BeginUndoGroup,
    EndUndoGroup,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Fit {
    Cover,
    Contain,
    Frame,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, ts(optional_fields))]
pub struct Props {
    pub name: Option<String>,
    pub hidden: Option<bool>,
    pub locked: Option<bool>,
    pub keep_ratio: Option<bool>,
    pub rotation: Option<f64>,
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
    pub stroke_sides: Option<Vec<f32>>,
    pub join: Option<Join>,
    pub cap: Option<Cap>,
    pub line_style: Option<LineStyle>,
    pub arrow_start: Option<bool>,
    pub arrow_end: Option<bool>,
    pub opacity: Option<f32>,
    pub blend: Option<Blend>,
    pub effects: Option<Vec<Effect>>,
    pub mask: Option<bool>,
    pub overprint_fill: Option<bool>,
    pub overprint_stroke: Option<bool>,
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
    pub max_lines: Option<u32>,
    pub trim: Option<bool>,
    pub round_baselines: Option<bool>,
    pub column_rule: Option<text::Rule>,
    pub min_height: Option<f64>,
    pub wrap: Option<Wrap>,
    pub wrap_offset_x: Option<f64>,
    pub wrap_offset_y: Option<f64>,
    pub anchored: Option<Anchoring>,
}

const MAX_LENGTH: f64 = 20000.0 * MM;
const MAX_PAGE: f64 = 14400.0;
const MAX_BLEED: f64 = 100.0 * MM;
const MAX_TEXT: f64 = 5000.0;

fn within(v: Option<f64>, lo: f64, hi: f64, what: &str) -> Res<()> {
    match v {
        Some(v) if !(lo..=hi).contains(&v) => {
            let r = |v: f64| (v * 100.0).round() / 100.0;
            Err(format!("{what} must be from {} to {}", r(lo), r(hi)))
        }
        _ => Ok(()),
    }
}

impl Props {
    fn check(&self) -> Res<()> {
        let f = |v: Option<f32>| v.map(f64::from);
        within(f(self.stroke_weight), 0.0, MAX_LENGTH, "stroke weight")?;
        if let Some(c) = &self.stroke_sides {
            if !matches!(c.len(), 0 | 4) {
                return Err("stroke sides take 4 weights or none".into());
            }
            for &w in c {
                within(Some(w.into()), 0.0, MAX_LENGTH, "stroke weight")?;
            }
        }
        within(f(self.radius), 0.0, MAX_LENGTH, "radius")?;
        if let Some(c) = &self.corners {
            if !matches!(c.len(), 0 | 4) {
                return Err("corners take 4 radii or none".into());
            }
            for &r in c {
                within(Some(r.into()), 0.0, MAX_LENGTH, "radius")?;
            }
        }
        within(self.count.map(f64::from), 3.0, 60.0, "count")?;
        within(f(self.ratio), 0.01, 1.0, "ratio")?;
        within(f(self.start), -360.0, 360.0, "start")?;
        within(f(self.sweep), 0.0, 1.0, "sweep")?;
        within(f(self.inner), 0.0, 1.0, "inner radius")?;
        within(f(self.opacity), 0.0, 1.0, "opacity")?;
        within(self.rotation, -360.0, 360.0, "rotation")?;
        for p in [
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        ] {
            within(p, 0.0, MAX_LENGTH, "padding")?;
        }
        for p in [
            self.inset_top,
            self.inset_right,
            self.inset_bottom,
            self.inset_left,
        ] {
            within(p, 0.0, MAX_LENGTH, "inset")?;
        }
        within(self.columns.map(f64::from), 1.0, 20.0, "columns")?;
        within(self.gutter, 0.0, MAX_LENGTH, "gutter")?;
        within(self.min_height, 0.0, MAX_LENGTH, "min height")?;
        if let Some(r) = &self.column_rule {
            within(f(Some(r.weight)), 0.0, MAX_LENGTH, "rule weight")?;
            r.color.check()?;
        }
        within(self.wrap_offset_x, 0.0, MAX_LENGTH, "wrap offset")?;
        within(self.wrap_offset_y, 0.0, MAX_LENGTH, "wrap offset")?;
        if self.baseline_grid != Some(0.0) {
            within(self.baseline_grid, 1.0, MAX_LENGTH, "baseline grid")?;
        }
        within(self.baseline_start, 0.0, MAX_LENGTH, "baseline start")?;
        within(self.max_lines.map(f64::from), 0.0, 1000.0, "max lines")?;
        within(self.gap, -MAX_LENGTH, MAX_LENGTH, "gap")?;
        for e in self.effects.iter().flatten() {
            within(Some(e.radius.into()), 0.0, MAX_LENGTH, "blur")?;
            for v in [e.x, e.y] {
                within(f(Some(v)), -MAX_LENGTH, MAX_LENGTH, "effect offset")?;
            }
            e.color.check()?;
        }
        for f in self.fills.iter().chain(&self.strokes).flatten() {
            f.color.check()?;
            if !f.transform.iter().all(|v| v.is_finite()) {
                return Err("paint transforms must be finite".into());
            }
            for v in f.adjust {
                within(Some(v.into()), -1.0, 1.0, "image adjustment")?;
            }
            if matches!(f.kind, FillKind::Linear | FillKind::Radial) && f.stops.len() < 2 {
                return Err("a gradient needs 2 stops or more".into());
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

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, ts(optional_fields = nullable))]
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
    pub paragraph_indent: Option<f64>,
    pub indent_left: Option<f64>,
    pub indent_right: Option<f64>,
    pub space_before: Option<f64>,
    pub text_case: Option<TextCase>,
    pub text_decoration: Option<TextDecoration>,
    pub features: Option<Vec<String>>,
    pub axes: Option<BTreeMap<String, f64>>,
    pub condition: Option<String>,
    pub position: Option<Position>,
    pub baseline_shift: Option<f64>,
    pub drop_lines: Option<u32>,
    pub drop_chars: Option<u32>,
    pub keep_lines: Option<u32>,
    pub keep_together: Option<bool>,
    pub keep_next: Option<bool>,
    pub tabs: Option<Vec<text::Tab>>,
    pub list: Option<text::List>,
    #[serde(
        default,
        deserialize_with = "some",
        skip_serializing_if = "Option::is_none"
    )]
    pub shading: Option<Option<Color>>,
}

fn some<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

impl TextProps {
    fn check(&self) -> Res<()> {
        check_text(
            self.size,
            self.line_height,
            self.letter_spacing,
            self.paragraph_spacing,
        )?;
        for v in [
            self.paragraph_indent,
            self.indent_left,
            self.indent_right,
            self.space_before,
        ] {
            within(v, 0.0, MAX_LENGTH, "indents and spacing")?;
        }
        within(
            self.baseline_shift,
            -MAX_LENGTH,
            MAX_LENGTH,
            "baseline shift",
        )?;
        if self.drop_lines.is_some_and(|v| v > 20)
            || self.drop_chars.is_some_and(|v| !(1..=20).contains(&v))
        {
            return Err("a drop cap takes up to 20 lines and 1 to 20 characters".into());
        }
        if self.keep_lines.is_some_and(|v| !(1..=20).contains(&v)) {
            return Err("keep 1 to 20 lines together".into());
        }
        if self
            .tabs
            .iter()
            .flatten()
            .any(|t| !(0.0..=MAX_LENGTH).contains(&t.at) || t.leader.chars().count() > 8)
        {
            return Err(
                "a tab stop sits at 0 pt or more with a leader of up to 8 characters".into(),
            );
        }
        if let Some(f) = self
            .features
            .iter()
            .flatten()
            .find(|f| f.parse::<Feature>().is_err())
        {
            return Err(format!("{f} is not an OpenType feature"));
        }
        if (self.axes.iter().flatten())
            .any(|(t, v)| t.len() != 4 || !t.is_ascii() || !v.is_finite())
        {
            return Err("an axis has a 4 letter tag and a finite value".into());
        }
        self.fill
            .iter()
            .chain(self.shading.iter().flatten())
            .try_for_each(Color::check)
    }
}

fn check_text(
    size: Option<f64>,
    line_height: Option<f64>,
    letter_spacing: Option<f64>,
    paragraph_spacing: Option<f64>,
) -> Res<()> {
    within(size, 0.1, MAX_TEXT, "text size")?;
    within(line_height, 0.0, MAX_LENGTH, "line height")?;
    within(letter_spacing, -100.0, 10000.0, "letter spacing")?;
    within(paragraph_spacing, 0.0, MAX_LENGTH, "paragraph spacing")
}

#[cfg_attr(test, derive(ts_rs::TS))]
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
    Table,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    Forward,
    Backward,
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeKind {
    Page,
    Master,
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
    grouped: bool,
    spares: RefCell<Option<Vec<TreeID>>>,
    clipboard: Vec<(Clip, Option<TreeID>)>,
    flows: RefCell<Option<Rc<HashMap<TreeID, Flow>>>>,
    snapshot: RefCell<Option<Rc<Snapshot>>>,
    checked: RefCell<Option<Rc<Snapshot>>>,
    palette: RefCell<Option<Rc<Palette>>>,
    numbers: RefCell<Option<HashMap<TreeID, String>>>,
    prevs: RefCell<Option<Rc<HashMap<TreeID, TreeID>>>>,
    sets: RefCell<HashMap<TreeID, Set>>,
    fits: RefCell<HashMap<TreeID, Measured>>,
    shown: RefCell<HashMap<TreeID, Shown>>,
    frames: RefCell<HashMap<TreeID, (TreeID, TextFrame)>>,
    changed: Arc<Mutex<Option<HashSet<TreeID>>>>,
    painted: RefCell<Option<Rc<Palette>>>,
    _changes: Subscription,
    sheets: RefCell<Sheets>,
    reflowed: RefCell<Option<HashSet<TreeID>>>,
    version: String,
    notes: RefCell<Vec<String>>,
    message: Cell<Option<&'static str>>,
    today: Cell<[u32; 3]>,
}

type Res<T> = Result<T, String>;

fn today() -> [u32; 3] {
    if cfg!(target_arch = "wasm32") {
        let now = js_sys::Date::new_0();
        [now.get_full_year(), now.get_month() + 1, now.get_date()]
    } else {
        [1970, 1, 1]
    }
}

type Sheets = (Rc<HashMap<TreeID, Flow>>, HashMap<TreeID, Rc<Page>>);

pub const MM: f64 = 72.0 / 25.4;

const PRINT_PPI: f64 = 300.0;

const IMAGE_ORIGIN: &str = "image";

const LAYOUT_ORIGIN: &str = "layout";

const SPARES: usize = 64;

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

const STORY: [&str; 31] = [
    "condition",
    "toc",
    "shading",
    "size",
    "lineHeight",
    "letterSpacing",
    "paragraphSpacing",
    "paragraphIndent",
    "indentLeft",
    "indentRight",
    "spaceBefore",
    "textCase",
    "textDecoration",
    "features",
    "axes",
    "position",
    "baselineShift",
    "fill",
    "textStyle",
    "textAlign",
    "hyphenate",
    "lang",
    "dropLines",
    "dropChars",
    "keepLines",
    "keepTogether",
    "keepNext",
    "tabs",
    "list",
    "fills",
    "font",
];

const KIND: &str = "kind";
const NEXT: &str = "next";
const TOC: &str = "toc";
const MASTER: &str = "master";
const DETACHED: &str = "detached";
const GRIDS: &str = "grids";
const GUIDES: &str = "guides";
const SECTION: &str = "section";
const OVERRIDE_OF: &str = "overrideOf";
const LEFT_OF: &str = "leftOf";
const MODES: &str = "modes";
const BINDINGS: &str = "bindings";
const CONSTRAINTS: &str = "constraints";
const SIZING: &str = "sizing";
const SELECTION: [f32; 4] = [0.302, 0.486, 1.0, 0.3];

const WHITE: u32 = 0xffffffff;

const BLACK: u32 = 0x000000ff;

const SAMPLE: &str = "Satz sets type in the browser. The engine shapes this paragraph \
with harfrust, breaks it into lines with the Knuth-Plass algorithm and justifies \
every line but the last to the width of its frame. The canvas and the PDF draw \
the same glyphs from the same font.";

fn undo_manager(doc: &LoroDoc) -> UndoManager {
    let mut undo = UndoManager::new(doc);
    undo.add_exclude_origin_prefix(IMAGE_ORIGIN);
    undo.add_exclude_origin_prefix(LAYOUT_ORIGIN);
    undo
}

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

fn touched(d: &ContainerDiff, nodes: &mut HashSet<TreeID>) -> bool {
    if let Diff::Tree(t) = &d.diff {
        return t.diff.iter().all(|i| {
            nodes.insert(i.target);
            let parents = match i.action {
                TreeExternalDiff::Create { parent, .. } => [parent, parent],
                TreeExternalDiff::Move {
                    parent, old_parent, ..
                } => [parent, old_parent],
                TreeExternalDiff::Delete { old_parent, .. } => [old_parent, old_parent],
            };
            parents
                .into_iter()
                .all(|p| parent_node(p).map(|p| nodes.insert(p)).is_some())
        });
    }
    let node = d.path.iter().find_map(|(_, i)| match i {
        Index::Node(n) => Some(*n),
        _ => None,
    });
    node.map(|n| nodes.insert(n)).is_some()
}

fn in_palette(d: &ContainerDiff) -> bool {
    let root = d.path.first().map_or(d.target, |(c, _)| c);
    matches!(root, ContainerID::Root { name, .. }
        if ["swatches", "collections", "variables", "textStyles"].contains(&name.as_str()))
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
    within(Some(x), -MAX_LENGTH, MAX_LENGTH, "x")?;
    within(Some(y), -MAX_LENGTH, MAX_LENGTH, "y")?;
    within(Some(w), 0.0, MAX_LENGTH, "width")?;
    within(Some(h), 0.0, MAX_LENGTH, "height")
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

fn field<T: DeserializeOwned>(v: &LoroValue, key: &str) -> Option<T> {
    match v {
        LoroValue::Map(m) => from_loro(m.get(key)?.clone()),
        _ => None,
    }
}

fn from_loro<T: DeserializeOwned>(v: LoroValue) -> Option<T> {
    T::deserialize(De(v)).ok()
}

struct De(LoroValue);

impl IntoDeserializer<'_, DeError> for De {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> Deserializer<'de> for De {
    type Error = DeError;

    fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, DeError> {
        match self.0 {
            LoroValue::Null | LoroValue::Container(_) => v.visit_unit(),
            LoroValue::Bool(b) => v.visit_bool(b),
            LoroValue::Double(d) => v.visit_f64(d),
            LoroValue::I64(i) => v.visit_i64(i),
            LoroValue::Binary(b) => v.visit_bytes(&b),
            LoroValue::String(s) => v.visit_str(&s),
            LoroValue::List(l) => v.visit_seq(SeqDeserializer::new(l.iter().cloned().map(De))),
            LoroValue::Map(m) => v.visit_map(MapDeserializer::new(
                m.iter().map(|(k, x)| (k.as_str(), De(x.clone()))),
            )),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, DeError> {
        match self.0 {
            LoroValue::Null => v.visit_none(),
            _ => v.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        v: V,
    ) -> Result<V::Value, DeError> {
        v.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        v: V,
    ) -> Result<V::Value, DeError> {
        match self.0 {
            LoroValue::String(s) => v.visit_enum(s.as_str().into_deserializer()),
            LoroValue::Map(m) => MapAccessDeserializer::new(MapDeserializer::new(
                m.iter().map(|(k, x)| (k.as_str(), De(x.clone()))),
            ))
            .deserialize_enum(name, variants, v),
            _ => self.deserialize_any(v),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

impl Doc {
    fn with(doc: LoroDoc) -> Doc {
        let mut styles = StyleConfigMap::new();
        styles.insert(
            anchor::OBJECT.into(),
            StyleConfig {
                expand: ExpandType::None,
            },
        );
        doc.config_text_style(styles);
        doc.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::After,
        }));
        doc.set_record_timestamp(true);
        doc.set_change_merge_interval(60);
        let tree = doc.get_tree("nodes");
        tree.enable_fractional_index(0);
        let changed = Arc::new(Mutex::new(None));
        let c = changed.clone();
        let _changes = doc.subscribe_root(Arc::new(move |e| {
            let mut c = c.lock().unwrap();
            if let Some(nodes) = c.as_mut()
                && !e.events.iter().all(|d| in_palette(d) || touched(d, nodes))
            {
                *c = None;
            }
        }));
        Doc {
            changed,
            painted: RefCell::new(None),
            _changes,
            sheets: RefCell::default(),
            reflowed: RefCell::default(),
            notes: RefCell::default(),
            message: Cell::default(),
            undo: undo_manager(&doc),
            grouped: false,
            spares: RefCell::default(),
            version: format!("{:?}", doc.oplog_frontiers()),
            doc,
            tree,
            clipboard: Vec::new(),
            flows: RefCell::new(None),
            snapshot: RefCell::new(None),
            checked: RefCell::new(None),
            palette: RefCell::new(None),
            prevs: RefCell::new(None),
            numbers: RefCell::new(None),
            sets: RefCell::new(HashMap::new()),
            fits: RefCell::new(HashMap::new()),
            shown: RefCell::default(),
            frames: RefCell::default(),
            today: Cell::new(today()),
        }
    }

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

    pub fn sample() -> Doc {
        let mut d = Doc::default();
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
            add(
                &page,
                NewKind::Ellipse,
                [8.0, 56.0, 66.0, 16.0],
                Props {
                    mask: Some(true),
                    ..named("Mask")
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
            crop: false,
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

    pub fn save(&self) -> Vec<u8> {
        let used = self.used_images();
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
        doc.export(ExportMode::state_only(None)).expect("export")
    }

    fn used_images(&self) -> HashSet<String> {
        let mut ids = Vec::new();
        for r in self.tree.roots() {
            if self.kind(r) != Some(NodeKind::Trash) {
                self.walk(r, &mut ids);
            }
        }
        let mut used = HashSet::new();
        for id in ids {
            if let Some(LoroValue::List(fills)) = value(&self.meta(id), "fills") {
                for f in fills.iter() {
                    if let LoroValue::Map(f) = f
                        && let Some(LoroValue::String(h)) = f.get("image")
                    {
                        used.insert(h.to_string());
                    }
                }
            }
        }
        used
    }

    pub fn set_profile(&mut self, icc: Option<Vec<u8>>) -> Res<()> {
        if let Some(icc) = &icc {
            color::check_profile(icc)?;
        }
        self.invalidate();
        let m = self.doc.get_map("document");
        match icc {
            Some(icc) => m.insert("profile", LoroValue::Binary(icc.into())),
            None => m.delete("profile"),
        }
        .map_err(err)?;
        self.finish(vec![], false)?;
        self.version = format!("{:?}", self.doc.oplog_frontiers());
        Ok(())
    }

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
        let d = Doc::read(bytes)?;
        d.register_images()?;
        d.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        d.finish(vec![], false)?;
        Ok(d)
    }

    #[allow(clippy::disallowed_methods)]
    pub(super) fn read(bytes: &[u8]) -> Res<Doc> {
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
        d.check_file()?;
        Ok(d)
    }

    pub fn retain_images(&self) -> Res<()> {
        image::retain(&self.register_images()?);
        Ok(())
    }

    fn register_images(&self) -> Res<Vec<String>> {
        let mut hashes = vec![];
        let images = self.doc.get_map("images").get_value().into_map();
        for (h, v) in images.unwrap_or_default().iter() {
            match (image::info(h), v) {
                (Some(i), _) => hashes.push(i.hash),
                (None, LoroValue::Binary(bytes)) => {
                    hashes.push(image::register(bytes.clone())?.hash)
                }
                _ => {}
            }
        }
        Ok(hashes)
    }

    pub fn version_vector(&self) -> Vec<u8> {
        self.doc.oplog_vv().encode()
    }

    pub fn updates(&self, since: &[u8]) -> Vec<u8> {
        let since = VersionVector::decode(since).ok();
        since
            .filter(|v| self.doc.oplog_vv().includes_vv(v))
            .and_then(|v| self.doc.export(ExportMode::updates_owned(v)).ok())
            .unwrap_or_else(|| (self.doc).export(ExportMode::Snapshot).expect("export"))
    }

    #[allow(clippy::disallowed_methods)]
    pub fn sync(&mut self, bytes: &[u8]) -> Res<()> {
        let meta = LoroDoc::decode_import_blob_meta(bytes, false).map_err(err)?;
        if meta.mode.is_snapshot() {
            *self = Doc::load(bytes)?;
            return Ok(());
        }
        self.doc.import(bytes).map_err(err)?;
        self.register_images()?;
        self.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        self.finish(vec![], false)?;
        self.version = format!("{:?}", self.doc.oplog_frontiers());
        Ok(())
    }

    fn check_file(&self) -> Res<()> {
        let json = |v: LoroValue| serde_json::to_value(v).map_err(err);
        let text =
            |v: serde_json::Value| serde_json::from_value::<TextProps>(v).map_err(err)?.check();
        let mut all = Vec::new();
        for r in self.tree.roots() {
            self.walk(r, &mut all);
        }
        for n in all {
            let v = json(self.meta(n).get_deep_value())?;
            serde_json::from_value::<Props>(v.clone())
                .map_err(err)?
                .check()?;
            text(v.clone())?;
            match self.kind(n) {
                Some(NodeKind::Page | NodeKind::Master) => {
                    serde_json::from_value::<pages::Sheet>(v)
                        .map_err(err)?
                        .check()?
                }
                Some(NodeKind::Group | NodeKind::Trash) => {}
                _ => check_frame(self.bounds(n))?,
            }
            for d in self.own_text(n).map(|t| t.to_delta()).unwrap_or_default() {
                if let TextDelta::Insert {
                    attributes: Some(a),
                    ..
                } = d
                {
                    text(json(a.into())?)?;
                }
            }
        }
        json(self.doc.get_list("textStyles").get_deep_value())?
            .as_array()
            .into_iter()
            .flatten()
            .try_for_each(|s| text(s.clone()))
    }

    pub fn apply(&mut self, cmd: Command) -> Res<Vec<String>> {
        if self.doc.is_detached() {
            return Err("a version is previewed".into());
        }
        if matches!(cmd, Command::BeginUndoGroup | Command::EndUndoGroup) {
            return self.run(cmd);
        }
        self.invalidate();
        self.notes.take();
        self.message.take();
        let history = matches!(cmd, Command::Undo | Command::Redo);
        let alone = !history && !self.grouped;
        if history {
            self.spares.take();
        } else if alone {
            self.refill()?;
        }
        if alone {
            self.undo.group_start().map_err(err)?;
        }
        let out = self.run(cmd);
        debug_assert!(
            out.is_ok() || self.doc.get_pending_txn_len() == 0,
            "a command wrote before it failed"
        );
        let out = out.and_then(|out| self.finish(out, history));
        if alone {
            self.undo.group_end();
        }
        let out = out?;
        self.version = format!("{:?}", self.doc.oplog_frontiers());
        Ok(out)
    }

    pub fn notes(&self) -> Vec<String> {
        self.notes.take()
    }

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
                crop,
            } => self.reframe(id, x, y, w, h, crop),
            Command::SetText { id, text } => self.set_text(id, story::clean(&text)),
            Command::EditText { id, range, text } => self.edit_text(id, range, story::clean(&text)),
            Command::Format { id, range, props } => self.format(id, range, props),
            Command::ScaleText { id, by } => {
                self.scale_text(self.node(&id)?, by)?;
                Ok(vec![])
            }
            Command::AddTextStyle { name, props } => self.add_text_style(name, props),
            Command::SetTextStyle {
                id,
                name,
                grep,
                props,
            } => self.set_text_style(id, name, grep, props),
            Command::DeleteTextStyle { id } => self.delete_text_style(id),
            Command::Toc { id, styles } => self.toc(id, styles),
            Command::Anchor { id, text, at } => self.anchor(id, text, at),
            Command::Unanchor { id } => self.unanchor(id),
            Command::TableInsert { id, column, at } => self.table_insert(id, column, at),
            Command::TableDelete { id, column, at } => self.table_delete(id, column, at),
            Command::MergeCells { ids } => self.merge_cells(ids),
            Command::SplitCell { id } => self.split_cell(id),
            Command::SizeTable { id, columns, rows } => {
                self.size_table(id, Table { columns, rows })
            }
            Command::Set { id, props } => self.set_props(id, props),
            Command::SetPath { id, path } => self.set_path(id, path),
            Command::FitImage { id, fit } => self.fit_image(id, fit),
            Command::Flip { id, vertical } => self.flip(id, vertical),
            Command::Flatten { id } => self.flatten(id),
            Command::Boolean { ids, op } => self.boolean(ids, op),
            Command::FillArea { id, x, y } => self.fill_area(id, x, y),
            Command::Delete { ids } => self.delete(ids),
            Command::Group { ids, frame } => {
                Ok(vec![self.group(&self.sorted(&ids)?, frame)?.to_string()])
            }
            Command::Ungroup { ids } => self.ungroup(ids),
            Command::Mask { ids } => self.mask(ids),
            Command::AutoLayout { ids } => self.auto_layout(ids),
            Command::Move { ids, parent, index } => self.move_layers(ids, parent, index),
            Command::Order { ids, to } => self.order(ids, to),
            Command::Restore { peer, counter } => self.restore(peer, counter),
            Command::Undo => {
                self.undo.group_end();
                self.grouped = false;
                self.undo.undo().map_err(err)?;
                Ok(vec![])
            }
            Command::Redo => {
                self.undo.group_end();
                self.grouped = false;
                self.undo.redo().map_err(err)?;
                Ok(vec![])
            }
            Command::BeginUndoGroup => {
                self.undo.group_end();
                self.undo.group_start().map_err(err)?;
                self.grouped = true;
                Ok(vec![])
            }
            Command::EndUndoGroup => {
                self.undo.group_end();
                self.grouped = false;
                Ok(vec![])
            }
            Command::Duplicate { ids } => self.duplicate(ids),
            Command::SetDocument(s) => self.set_document(s),
            Command::Relink { image, to, name } => self.relink_image(image, to, name),
            Command::Embed { images } => self.embed(images),
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
            Command::SetSection { id, section } => self.set_section(id, section),
            Command::DuplicatePage { id } => self.duplicate_page(id),
            Command::SetPage {
                id,
                width,
                height,
                bleed,
                scale,
            } => self.set_page(id, width, height, bleed, scale),
            Command::SetGrids { id, grids } => self.set_grids(id, grids),
            Command::SetGuides { id, guides } => self.set_guides(id, guides),
            Command::DeletePage { id } => self.delete_page(id),
            Command::MovePages { ids, index } => self.move_pages(ids, index),
            Command::AddMaster { like, layers } => self.add_master(like, layers),
            Command::SetMaster { id, name } => self.set_master(id, name),
            Command::DeleteMaster { id } => self.delete_master(id),
            Command::UseMaster { page, master } => self.use_master(page, master),
            Command::Override { page, id } => self.override_layer(page, id),
            Command::ResetToMaster { ids } => self.reset_to_master(ids),
            Command::Thread { from, to } => self.thread_after(from, to),
            Command::ThreadBefore { id, to } => self.thread_before(id, to),
            Command::Unthread { id } => self.unthread(id),
        }
    }

    fn create(&self, parent: &str, kind: NewKind, frame: [f64; 4]) -> Res<TreeID> {
        let p = self.container(parent)?;
        check_frame(frame)?;
        let id = self.new_node(p, None)?;
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
            NewKind::Table => (
                NodeKind::Frame,
                "",
                Props {
                    name: Some("Table".into()),
                    fills: Some(vec![]),
                    clip: Some(false),
                    ..Props::default()
                },
            ),
        };
        m.insert(KIND, name.as_str()).map_err(err)?;
        if !shape.is_empty() {
            m.insert("shape", shape).map_err(err)?;
        }
        self.set(id, props)?;
        self.set_frame(id, frame)?;
        if kind == NewKind::Table {
            self.fill_table(id)?;
        }
        Ok(id)
    }

    fn commit(&self) {
        if let Some(m) = self.message.get() {
            self.doc.set_next_commit_message(m);
        }
        self.doc.commit();
    }

    fn finish(&self, out: Vec<String>, history: bool) -> Res<Vec<String>> {
        let icc = value(&self.doc.get_map("document"), "profile");
        if color::use_profile(match &icc {
            Some(LoroValue::Binary(b)) => Some(b),
            _ => None,
        }) {
            image::forget(&[]);
        }
        if self.doc.get_pending_txn_len() > 0 {
            self.commit();
        }
        self.purge();
        self.numbers.take();
        self.check_links();
        debug_assert!(
            self.prevs
                .borrow()
                .as_ref()
                .is_none_or(|p| **p == self.links()),
            "a kept link back differs from the thread"
        );
        let palette = self.palette();
        self.repaint(&palette);
        let mut done = HashSet::new();
        let mut rounds = 0;
        let reach = loop {
            let reach = self.reach();
            let todo: Vec<_> = (self.tree.roots().into_iter())
                .filter(|p| !history && !done.contains(p) && self.is_sheet(*p))
                .filter(|p| reach.as_ref().is_none_or(|r| r.roots.contains(p)))
                .collect();
            if todo.is_empty() {
                break reach;
            }
            for p in todo {
                self.settle(p, &Modes::new(), &palette)?;
                self.lay_out(p)?;
                done.insert(p);
            }
            let heads = reach
                .as_ref()
                .map_or_else(|| self.heads(), |r| r.heads.clone());
            let moved = self.place_anchored(&heads)?;
            let changed = moved | (!history && self.refresh_tocs()?);
            debug_assert!(!changed || rounds < 8, "layout did not settle in 8 rounds");
            if changed && rounds < 8 {
                rounds += 1;
                done.clear();
            }
            self.commit();
            self.purge();
        };
        #[cfg(debug_assertions)]
        if !history && reach.is_some() {
            for p in self.tree.roots().into_iter().filter(|&p| self.is_sheet(p)) {
                self.settle(p, &Modes::new(), &palette)?;
                self.lay_out(p)?;
            }
            self.place_anchored(&self.heads())?;
            assert_eq!(
                self.doc.get_pending_txn_len(),
                0,
                "a page left out lays out otherwise"
            );
        }
        self.palette.replace(Some(palette));
        let flows = self.flow_all(reach.as_ref());
        debug_assert!(
            reach.is_none() || flows == self.flow_all(None),
            "a kept flow differs from one set again"
        );
        self.flows.replace(Some(Rc::new(flows)));
        let snap = Rc::new(self.build_snapshot());
        self.palette.take();
        self.checked.replace(Some(snap.clone()));
        self.snapshot.replace(Some(snap));
        Ok(out)
    }

    fn invalidate(&self) {
        self.numbers.take();
        self.flows.take();
        self.snapshot.take();
    }

    fn layout(&self, id: TreeID) -> Layout {
        from_loro(self.meta(id).get_deep_value()).unwrap_or_default()
    }

    fn lay_out(&self, id: TreeID) -> Res<()> {
        let kids = self.children(id);
        for &c in &kids {
            self.lay_out(c)?;
        }
        match self.kind(id) {
            Some(NodeKind::Text) => return self.fit(id),
            Some(NodeKind::Frame) => {
                if let Some(t) = self.table(id) {
                    return self.lay_out_table(id, t);
                }
            }
            _ => return Ok(()),
        }
        let mut l = self.layout(id);
        let Some((horizontal, pad)) = l.axes() else {
            return Ok(());
        };
        let flow: Vec<TreeID> = kids
            .into_iter()
            .filter(|&c| !self.layout(c).absolute)
            .collect();
        let mut fixed = Vec::new();
        for (a, side) in [(true, "width"), (false, "height")] {
            if l.size(a) == Size::Hug && flow.iter().all(|&c| self.layout(c).size(a) == Size::Fill)
            {
                *if a {
                    &mut l.sizing.horizontal
                } else {
                    &mut l.sizing.vertical
                } = Size::Fixed;
                fixed.push(side);
            }
        }
        if !fixed.is_empty() {
            self.meta(id).insert(SIZING, loro(l.sizing)?).map_err(err)?;
            if !flow.is_empty() {
                let note = format!(
                    "{} of {} set to fixed: its layers fill it",
                    fixed.join(" and "),
                    Some(self.name(id))
                        .filter(|n| !n.is_empty())
                        .unwrap_or("Frame".into())
                );
                self.notes
                    .borrow_mut()
                    .push(note[..1].to_uppercase() + &note[1..]);
            }
        }
        if flow.is_empty() {
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
        let apart = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).any(|(a, b)| (a - b).abs() > 1e-9);
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
            if apart(frame, old) {
                self.set_frame(id, frame)?;
            }
            let mut again = false;
            for (&c, [[a0, a], [b0, b]]) in flow.iter().zip(boxes) {
                let old = self.bounds(c);
                let new = unaxes([[m0 + a0, a], [c0 + b0, b]]);
                if apart(new, old) {
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
        self.json(id, CONSTRAINTS)
    }

    fn json<T: DeserializeOwned + Default>(&self, id: TreeID, key: &str) -> T {
        value(&self.meta(id), key)
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    fn list<T: DeserializeOwned>(&self, name: &str) -> Vec<T> {
        from_loro(self.doc.get_list(name).get_deep_value()).unwrap_or_default()
    }

    fn find<T: DeserializeOwned>(&self, name: &str, id: &str) -> Res<(usize, T)> {
        let all: Vec<serde_json::Value> = self.list(name);
        let i = all
            .iter()
            .position(|v| v["id"] == id)
            .ok_or_else(|| format!("no {} {id}", name.trim_end_matches('s')))?;
        Ok((i, serde_json::from_value(all[i].clone()).map_err(err)?))
    }

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
            match v {
                LoroValue::Map(v) if k == "values" || k == "bindings" => {
                    if value(&m, k).is_some() {
                        m.delete(k).map_err(err)?;
                    }
                    let n = m.ensure_mergeable_map(k).map_err(err)?;
                    for k in n
                        .keys()
                        .filter(|k| !v.contains_key(k.as_str()))
                        .collect::<Vec<_>>()
                    {
                        n.delete(&k).map_err(err)?;
                    }
                    for (k, v) in v.iter() {
                        if value(&n, k).as_ref() != Some(v) {
                            n.insert(k, v.clone()).map_err(err)?;
                        }
                    }
                }
                _ if value(&m, k).as_ref() != Some(v) => {
                    m.insert(k, v.clone()).map_err(err)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn color_mode(&self) -> ColorMode {
        self.setting("colorMode", ColorMode::Rgb)
    }

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

    fn reparent(&self, ids: &[TreeID], to: TreeID, index: usize) -> Res<()> {
        let places = self.places();
        let place = |n: TreeID| places.iter().find(|q| q.id == self.root(n));
        let back = geom::invert(self.turn(to));
        let angle = self.angle(to);
        let mut moves = Vec::new();
        for &id in ids {
            let p = self
                .tree
                .parent(id)
                .and_then(parent_node)
                .ok_or("no parent")?;
            let [x, y, w, h] = self.bounds(id);
            let c = [x + w / 2.0, y + h / 2.0];
            let [sx, sy] = geom::apply(self.turn(p), c);
            let spread = match (place(id), place(to)) {
                (Some(a), Some(b)) if a.spread == b.spread => a.x - b.x,
                _ => 0.0,
            };
            let [nx, ny] = geom::apply(back, [sx + spread, sy]);
            let (dx, dy) = (nx - c[0], ny - c[1]);
            let r = num(&self.meta(id), "rotation") + self.angle(p) - angle;
            let r = if r.abs() > 360.0 { r % 360.0 } else { r };
            moves.push((id, [x + dx, y + dy, w, h], r));
        }
        let mut groups: Vec<(TreeID, [f64; 4])> = Vec::new();
        for id in ids
            .iter()
            .filter_map(|&id| self.tree.parent(id)?.tree_id())
            .chain([to])
        {
            let mut g = Some(id);
            while let Some(n) = g.filter(|&n| self.kind(n) == Some(NodeKind::Group)) {
                if !groups.iter().any(|&(o, _)| o == n) {
                    groups.push((n, self.bounds(n)));
                }
                g = self.tree.parent(n).and_then(parent_node);
            }
        }
        groups.sort_by_key(|&(g, _)| std::cmp::Reverse(self.depth(g)));
        for (i, (id, frame, r)) in moves.into_iter().enumerate() {
            self.tree.mov_to(id, to, index + i).map_err(err)?;
            self.set_frame(id, frame)?;
            if r != num(&self.meta(id), "rotation") {
                self.meta(id).insert("rotation", r).map_err(err)?;
            }
        }
        for (g, [x, y, w, h]) in groups {
            let r = num(&self.meta(g), "rotation");
            let [nx, ny, nw, nh] = self.bounds(g);
            if r == 0.0 || self.children(g).is_empty() {
                continue;
            }
            let d = [nx + nw / 2.0 - x - w / 2.0, ny + nh / 2.0 - y - h / 2.0];
            let [tx, ty] = geom::apply(geom::rotation(r, [0.0; 4]), d);
            self.set_frame(g, [nx + tx - d[0], ny + ty - d[1], nw, nh])?;
        }
        Ok(())
    }

    fn depth(&self, mut id: TreeID) -> usize {
        let mut n = 0;
        while let Some(TreeParentId::Node(p)) = self.tree.parent(id) {
            id = p;
            n += 1;
        }
        n
    }

    fn angle(&self, mut id: TreeID) -> f64 {
        let mut a = num(&self.meta(id), "rotation");
        while let Some(TreeParentId::Node(p)) = self.tree.parent(id) {
            id = p;
            a += num(&self.meta(id), "rotation");
        }
        a
    }

    fn group(&self, ids: &[TreeID], frame: bool) -> Res<TreeID> {
        let top = *ids.last().ok_or("nothing to group")?;
        let parent = self.tree.parent(top).ok_or("no parent")?;
        let index = self.index(top) + 1;
        let g = self.new_node(parent, Some(index))?;
        let m = self.meta(g);
        let kind = if frame {
            NodeKind::Frame
        } else {
            NodeKind::Group
        };
        m.insert(KIND, kind.as_str()).map_err(err)?;
        let olds: Vec<_> = ids.iter().map(|&id| self.tree.parent(id)).collect();
        self.reparent(ids, g, 0)?;
        if frame {
            m.insert("clip", true).map_err(err)?;
            self.resize(g, union(ids.iter().map(|&id| self.bounds(id))), false)?;
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

    fn remove(&self, id: TreeID) -> Res<()> {
        self.unlink_all(id)?;
        let trash = self.trash()?;
        self.prevs.take();
        self.tree.mov(id, trash).map_err(err)
    }

    fn trash(&self) -> Res<TreeID> {
        if let Some(t) = self.trash_root() {
            return Ok(t);
        }
        let t = self.tree.create(None).map_err(err)?;
        self.meta(t)
            .insert(KIND, NodeKind::Trash.as_str())
            .map_err(err)?;
        Ok(t)
    }

    fn refill(&self) -> Res<()> {
        let mut spares = self.spares.borrow_mut();
        if spares.is_none() {
            let trash = self.trash_root();
            *spares = Some(
                trash
                    .and_then(|t| self.tree.children(t))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|&c| self.meta(c).is_empty() && self.tree.children_num(c) == Some(0))
                    .collect(),
            );
        }
        let spares = spares.as_mut().unwrap();
        if spares.len() >= SPARES / 2 {
            return Ok(());
        }
        self.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        let trash = self.trash()?;
        while spares.len() < SPARES {
            spares.push(self.tree.create(trash).map_err(err)?);
        }
        self.doc.commit();
        Ok(())
    }

    fn trash_root(&self) -> Option<TreeID> {
        self.tree
            .roots()
            .into_iter()
            .find(|&r| self.kind(r) == Some(NodeKind::Trash))
    }

    fn new_node(&self, parent: impl Into<TreeParentId>, index: Option<usize>) -> Res<TreeID> {
        let parent = parent.into();
        let i = index.unwrap_or_else(|| self.tree.children_num(parent).unwrap_or(0));
        match self.spares.borrow_mut().as_mut().and_then(Vec::pop) {
            Some(id) => self.tree.mov_to(id, parent, i).map(|()| id),
            None => self.tree.create_at(parent, i),
        }
        .map_err(err)
    }

    pub(super) fn turn(&self, mut id: TreeID) -> [f64; 6] {
        let mut m = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        loop {
            let r = num(&self.meta(id), "rotation");
            if r != 0.0 {
                m = geom::then(m, geom::rotation(r, self.bounds(id)));
            }
            match self.tree.parent(id) {
                Some(TreeParentId::Node(p)) => id = p,
                _ => return m,
            }
        }
    }

    pub fn turn_of(&self, id: &str) -> Res<[f32; 6]> {
        Ok(self.turn(self.node(id)?).map(|v| v as f32))
    }

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

    fn layer(&self, id: &str) -> Res<TreeID> {
        let n = self.node(id)?;
        match self.tree.parent(n) {
            Some(TreeParentId::Node(_)) => Ok(n),
            _ => Err(format!("{id} is not a layer")),
        }
    }

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
        let mut out = Vec::new();
        let mut stack: Vec<_> = self.tree.roots().into_iter().rev().collect();
        while let Some(n) = stack.pop() {
            if ids.contains(&n) {
                out.push(n);
            } else {
                stack.extend(self.children(n).into_iter().rev());
            }
        }
        Ok(out)
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

    fn reframe(&self, id: String, x: f64, y: f64, w: f64, h: f64, crop: bool) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        check_frame([x, y, w, h])?;
        let fills: Vec<Fill> = value(&self.meta(id), "fills")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default();
        let cropped = crop && fills.iter().any(|f| f.kind == FillKind::Image);
        if cropped && (w <= 0.0 || h <= 0.0) {
            return Err("an image cannot be cropped to nothing".into());
        }
        let was = self.bounds(id);
        let [.., ow, oh] = was;
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
            self.meta(id).insert(SIZING, loro(sizing)?).map_err(err)?;
        }
        if self.table(id).is_some() {
            self.scale_table(id, w, h)?;
        }
        self.resize(id, [x, y, w, h], !crop)?;
        if cropped {
            let r = num(&self.meta(id), "rotation");
            let new = self.bounds(id);
            let unit = |[x, y, w, h]: [f64; 4]| [w, 0.0, 0.0, h, x, y];
            let fills: Vec<Fill> = fills
                .into_iter()
                .map(|f| match f.kind {
                    FillKind::Image => {
                        let m = f.transform.map(f64::from);
                        let page = geom::then(geom::then(m, unit(was)), geom::rotation(r, was));
                        let back = geom::then(
                            geom::invert(geom::rotation(r, new)),
                            geom::invert(unit(new)),
                        );
                        Fill {
                            transform: geom::then(page, back).map(|v| v as f32),
                            ..f
                        }
                    }
                    _ => f,
                })
                .collect();
            self.meta(id).insert("fills", loro(fills)?).map_err(err)?;
        }
        Ok(vec![])
    }

    fn set_props(&self, id: String, mut props: Props) -> Res<Vec<String>> {
        let id = self.node(&id)?;
        if props.anchored.is_some() && value(&self.meta(id), ANCHORED).is_none() {
            return Err("not anchored".into());
        }
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
        let [x, y, w, h] = self.bounds(id).map(|v| v as f32);
        let [l, t, nw, nh] = bounds(&path);
        let scale = |v: f32, s: f32| if s > 0.0 { v / s } else { 0.0 };
        let areas: Vec<f32> = self.json(id, "areas");
        let seeds = geom::map(&areas, |[u, v]| [x + u * w, y + v * h]);
        let seeds = if seeds.is_empty() {
            seeds
        } else {
            boolean::follow(&self.lines_of(id)?, &path, &seeds)
        };
        let areas = geom::map(&seeds, |[px, py]| [scale(px - l, nw), scale(py - t, nh)]);
        self.set(
            id,
            Props {
                path: Some(fit(&path, [0.0, 0.0, 1.0, 1.0])),
                ..Props::default()
            },
        )?;
        if !areas.is_empty() {
            self.meta(id)
                .insert("areas", loro(serde_json::to_value(areas).map_err(err)?)?)
                .map_err(err)?;
        }
        self.set_frame(id, bounds(&path).map(f64::from))?;
        Ok(vec![])
    }

    fn flatten(&self, id: String) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        let m = self.meta(id);
        if value(&m, "shape") == Some("path".into()) && num(&m, "rotation") == 0.0 {
            return Ok(vec![]);
        }
        let path = self.outline_of(id)?;
        self.make_path(id, &path)?;
        Ok(vec![])
    }

    pub fn path_of(&self, id: &str) -> Res<Vec<f32>> {
        self.outline_of(self.layer(id)?)
    }

    fn fit_image(&self, id: String, fit: Fit) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        let mut fills: Vec<Fill> = self.json(id, "fills");
        let [x, y, w, h] = self.bounds(id);
        let mut aspect = None;
        for f in fills.iter_mut().filter(|f| f.kind == FillKind::Image) {
            let info = self.image(f.image.as_deref().unwrap_or_default())?;
            let a = f64::from(info.width) / f64::from(info.height);
            let k = a / (w / h);
            let [sw, sh] = match fit {
                Fit::Frame => [1.0, 1.0],
                _ if (k > 1.0) == (fit == Fit::Cover) => [k, 1.0],
                _ => [1.0, 1.0 / k],
            };
            f.transform = [sw, 0.0, 0.0, sh, (1.0 - sw) / 2.0, (1.0 - sh) / 2.0].map(|v| v as f32);
            aspect = Some(a);
        }
        let a = aspect.ok_or("no image fill")?;
        if w <= 0.0 || h <= 0.0 {
            return Err("an image fits only into a box".into());
        }
        if fit == Fit::Frame {
            self.set_frame(id, [x, y, w, w / a])?;
        }
        self.set(
            id,
            Props {
                fills: Some(fills),
                ..Props::default()
            },
        )?;
        Ok(vec![])
    }

    fn flip(&self, id: String, vertical: bool) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        let [x, y, w, h] = self.bounds(id);
        self.mirror(
            id,
            if vertical { y + h / 2.0 } else { x + w / 2.0 },
            vertical,
        )?;
        Ok(vec![])
    }

    fn mirror(&self, id: TreeID, at: f64, vertical: bool) -> Res<()> {
        let m = self.meta(id);
        let unit = if vertical {
            [1.0, 0.0, 0.0, -1.0, 0.0, 1.0]
        } else {
            [-1.0, 0.0, 0.0, 1.0, 1.0, 0.0]
        };
        let paints = |key: &str| -> Option<Vec<Fill>> {
            let fills: Vec<Fill> = self.json(id, key);
            (!fills.is_empty()).then(|| {
                fills
                    .into_iter()
                    .map(|f| Fill {
                        transform: geom::then(f.transform.map(f64::from), unit).map(|v| v as f32),
                        ..f
                    })
                    .collect()
            })
        };
        let effects: Vec<Effect> = self.json(id, "effects");
        let sides: Vec<f32> = self.json(id, "strokeSides");
        let mut props = Props {
            fills: paints("fills"),
            strokes: paints("strokes"),
            stroke_sides: (sides.len() == 4).then(|| {
                let order = if vertical { [2, 1, 0, 3] } else { [0, 3, 2, 1] };
                order.map(|i| sides[i]).to_vec()
            }),
            effects: (!effects.is_empty()).then(|| {
                effects
                    .into_iter()
                    .map(|e| Effect {
                        x: if vertical { e.x } else { -e.x },
                        y: if vertical { -e.y } else { e.y },
                        ..e
                    })
                    .collect()
            }),
            ..Props::default()
        };
        if self.kind(id) == Some(NodeKind::Shape) {
            let v = serde_json::to_value(m.get_value()).map_err(err)?;
            let flipped = |p: &[f32]| {
                geom::map(
                    p,
                    |[u, v]| if vertical { [u, 1.0 - v] } else { [1.0 - u, v] },
                )
            };
            match serde_json::from_value(v).map_err(err)? {
                Shape::Rect { corners, .. } if corners.len() == 4 => {
                    let order = if vertical { [3, 2, 1, 0] } else { [1, 0, 3, 2] };
                    props.corners = Some(order.map(|i| corners[i]).to_vec());
                }
                Shape::Ellipse { start, sweep, .. } => {
                    let from = if vertical { 0.0 } else { 180.0 };
                    props.start = Some((from - start - 360.0 * sweep.min(1.0)).rem_euclid(360.0));
                }
                s @ (Shape::Polygon { count } | Shape::Star { count, .. })
                    if vertical && count % 2 == 1 =>
                {
                    m.insert("shape", "path").map_err(err)?;
                    props.path = Some(flipped(&outline(&s, [0.0, 0.0, 1.0, 1.0])));
                }
                Shape::Path { path, areas } => {
                    props.path = Some(flipped(&path));
                    if !areas.is_empty() {
                        m.insert(
                            "areas",
                            loro(serde_json::to_value(flipped(&areas)).map_err(err)?)?,
                        )
                        .map_err(err)?;
                    }
                }
                _ => {}
            }
        }
        self.set(id, props)?;
        let r = num(&m, "rotation");
        if r != 0.0 {
            m.insert("rotation", -r).map_err(err)?;
        }
        for c in self.children(id) {
            self.mirror(c, at, vertical)?;
        }
        if self.kind(id) == Some(NodeKind::Group) {
            return Ok(());
        }
        let [x, y, w, h] = self.bounds(id);
        let frame = if vertical {
            [x, 2.0 * at - y - h, w, h]
        } else {
            [2.0 * at - x - w, y, w, h]
        };
        self.unbind(id, |p| p == if vertical { "y" } else { "x" })?;
        let m = self.meta(id);
        for (k, v) in ["x", "y"].into_iter().zip(frame) {
            m.insert(k, v).map_err(err)?;
        }
        Ok(())
    }

    fn outline_of(&self, id: TreeID) -> Res<Vec<f32>> {
        let frame = self.bounds(id).map(|v| v as f32);
        let path = match self.kind(id) {
            Some(NodeKind::Shape) => {
                let v = serde_json::to_value(self.meta(id).get_value()).map_err(err)?;
                let shape: Shape = serde_json::from_value(v).map_err(err)?;
                outline(&shape, frame)
            }
            Some(NodeKind::Text) => self.glyphs(&id.to_string(), frame)?,
            _ => return Err("only shapes and text flatten".into()),
        };
        let rotation = num(&self.meta(id), "rotation");
        let [a, b, c, d, e, f] = geom::rotation(rotation, frame.map(f64::from)).map(|v| v as f32);
        let path = geom::map(&path, |[u, v]| [a * u + c * v + e, b * u + d * v + f]);
        if !geom::valid(&path) {
            return Err("not a path".into());
        }
        Ok(path)
    }

    fn make_path(&self, id: TreeID, path: &[f32]) -> Res<()> {
        let m = self.meta(id);
        if self.kind(id) == Some(NodeKind::Text) {
            m.delete("text").map_err(err)?;
            m.insert(KIND, NodeKind::Shape.as_str()).map_err(err)?;
            self.set(
                id,
                Props {
                    sizing: Some(Sizing::default()),
                    ..Props::default()
                },
            )?;
        }
        m.insert("shape", "path").map_err(err)?;
        m.insert("rotation", 0.0).map_err(err)?;
        m.delete("areas").map_err(err)?;
        self.set(
            id,
            Props {
                path: Some(fit(path, [0.0, 0.0, 1.0, 1.0])),
                ..Props::default()
            },
        )?;
        self.set_frame(id, bounds(path).map(f64::from))
    }

    fn glyphs(&self, id: &str, frame: [f32; 4]) -> Res<Vec<f32>> {
        fn find<'a>(nodes: &'a [Rc<Node>], id: &str) -> Option<&'a Node> {
            nodes.iter().find_map(|n| match &n.kind {
                _ if n.id == id => Some(&**n),
                Kind::Group { children } | Kind::Frame { children, .. } => find(children, id),
                _ => None,
            })
        }
        let snap = self.snapshot();
        let n = snap
            .pages
            .iter()
            .chain(&snap.masters)
            .find_map(|p| find(&p.children, id))
            .ok_or("no such layer")?;
        let Kind::Text {
            content,
            frame: tf,
            prev: None,
            next: None,
            placed,
            ..
        } = &n.kind
        else {
            return Err("threaded text does not flatten".into());
        };
        let s = Scope {
            palette: &snap.palette,
            modes: &n.active_modes,
        };
        let style = Style {
            fills: vec![Fill::solid(Color::gray(ColorMode::Rgb))],
            ..Style::default()
        };
        Ok(
            text::draw(&content.text, &content.spans, &style, frame, tf, &s, placed)
                .into_iter()
                .flat_map(|op| match op {
                    Op::GlyphRun {
                        font,
                        size,
                        glyphs,
                        positions,
                        ..
                    } => text::outline(font, size, &glyphs, &positions),
                    Op::FillPath { path, .. } => path,
                    _ => Vec::new(),
                })
                .collect(),
        )
    }

    fn delete(&self, ids: Vec<String>) -> Res<Vec<String>> {
        for id in self.nodes(&ids)? {
            let parent = self.tree.parent(id);
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
            let parent = self
                .tree
                .parent(g)
                .and_then(parent_node)
                .ok_or("no parent")?;
            let kids = self.children(g);
            self.reparent(&kids, parent, self.index(g))?;
            out.extend(kids.iter().map(|c| c.to_string()));
            self.remove(g)?;
        }
        Ok(out)
    }

    fn mask(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let ids = self.sorted(&ids)?;
        let highest = *ids.last().ok_or("nothing to mask")?;
        Ok(if ids.len() == 1 {
            let on = value(&self.meta(highest), "mask").and_then(|v| v.into_bool().ok());
            self.meta(highest)
                .insert("mask", on != Some(true))
                .map_err(err)?;
            vec![highest.to_string()]
        } else {
            let g = self.group(&ids, false)?;
            self.meta(g).insert("name", "Mask group").map_err(err)?;
            self.meta(highest).insert("mask", true).map_err(err)?;
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
                clip: (!single).then_some(false),
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
        let others = self
            .children(p)
            .into_iter()
            .filter(|c| !ids.contains(c))
            .count();
        self.reparent(&ids, p, index.min(others))?;
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
        Doc::blank(148.0 * MM, 210.0 * MM, 1, true, ColorMode::Rgb)
    }
}

#[cfg(test)]
mod tests;
