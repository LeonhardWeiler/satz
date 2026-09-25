use crate::color::Ink;
use crate::color::{Color, ColorMode, Swatch};
use crate::display_list::{CLOSE, LINE, MOVE, Op, Paint, rect, shift};
use crate::geom::{Shape, bounds, contains, fit, near, outline};
use crate::layout::{Align3, Direction, Layout, MainAlign, Size, Sizing, arrange};
use crate::style::{
    Align, Blend, Cap, Constraint, Constraints, Effect, EffectKind, Fill, FillKind, FillStop, Join,
    Style,
};
use crate::text::{
    self, Attrs, Lang, PARAGRAPH, STYLED, Span, TextAlign, TextFrame, TextStyle, VerticalAlign,
};
use crate::variable::{Collection, Mode, Modes, Palette, Scope, Value, Variable};
use loro::{
    Container, ExpandType, LoroDoc, LoroMap, LoroText, LoroTree, LoroValue, StyleConfig, TextDelta,
    TreeID, TreeParentId, UndoManager, UpdateOptions, ValueOrContainer,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::ops::Range;
use std::rc::Rc;

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
        /// Pairs the pages into spreads, as in InDesign.
        #[serde(default)]
        facing_pages: Option<bool>,
    },
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
    /// Sets the trim size and bleed of a page in pt.
    SetPage {
        id: String,
        width: Option<f64>,
        height: Option<f64>,
        bleed: Option<f64>,
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
    pub hyphenate: Option<bool>,
    pub lang: Option<Lang>,
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
    pub masters: Vec<Page>,
    pub facing_pages: bool,
    /// The ids of the pages of each spread from left to right: with facing pages the
    /// first page alone on the right, then pairs; else each page alone.
    pub spreads: Vec<Vec<String>>,
    /// The story of each thread by its first frame.
    pub stories: BTreeMap<String, Rc<Story>>,
    /// Resolution at which the PDF rasterizes shadows and blurs.
    pub raster_ppi: f64,
    pub color_mode: ColorMode,
    #[serde(flatten)]
    pub palette: Palette,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// A page or a master; `name` is a master's, `master` the one a page uses and
/// `detached` its master layers it overrides.
#[derive(Debug, PartialEq, Serialize)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub bleed: f64,
    /// The side of its spread a page is on with facing pages.
    pub side: Option<Side>,
    /// Where the page's left edge sits on its spread, whose spine is at 0.
    pub x: f64,
    pub master: Option<String>,
    pub detached: Vec<String>,
    pub modes: Modes,
    pub children: Vec<Node>,
}

/// The text of a thread and its runs of equal attributes.
#[derive(Debug, PartialEq, Serialize)]
pub struct Story {
    pub text: String,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Left,
    Right,
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
    /// The master layer this page layer overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_of: Option<String>,
    #[serde(flatten)]
    pub style: Style,
    #[serde(flatten)]
    pub layout: Layout,
    #[serde(flatten)]
    pub kind: Kind,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Kind {
    Shape(Shape),
    /// `content` is the story of the thread, whose first frame is `story`; the layer
    /// sets it from `start` up to `end` in UTF-16, and `overset` when it is the last
    /// frame and text is left.
    Text {
        #[serde(skip)]
        content: Rc<Story>,
        #[serde(flatten)]
        frame: TextFrame,
        story: String,
        start: usize,
        end: usize,
        prev: Option<String>,
        next: Option<String>,
        overset: bool,
        /// `start` as a byte offset.
        #[serde(skip)]
        from: usize,
    },
    Group {
        children: Vec<Node>,
    },
    Frame {
        clip: bool,
        children: Vec<Node>,
    },
}

/// Where a page sits on its spread; see `Doc::places`.
struct Place {
    id: TreeID,
    spread: usize,
    side: Option<Side>,
    x: f64,
}

/// Where the story of a thread, from the frame `head`, flows through a frame: the
/// bytes `start..end`, its neighbours, and text left over at the end of the thread.
struct Flow {
    head: TreeID,
    story: Rc<Story>,
    start: usize,
    end: usize,
    /// Where the text left over for the next frame starts.
    rest: Option<usize>,
    prev: Option<TreeID>,
    next: Option<TreeID>,
    overset: bool,
}

pub struct Doc {
    doc: LoroDoc,
    tree: LoroTree,
    undo: UndoManager,
    clipboard: Vec<(Clip, Option<TreeID>)>,
    /// The flows through all text layers, from the end of the last command on.
    flows: RefCell<Option<Rc<HashMap<TreeID, Flow>>>>,
}

struct Clip {
    /// The copied layer, so that copies of threaded frames thread among themselves.
    id: TreeID,
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
/// The keys of a text layer that belong to its story and move with it.
const STORY: [&str; 10] = [
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
];
/// Figma's selection blue at 30 %.
const SELECTION: [f32; 4] = [0.051, 0.6, 1.0, 0.3];
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
            flows: RefCell::new(None),
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
            facing_pages: Some(true),
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
        d.undo = UndoManager::new(&d.doc);
        d
    }

    pub fn apply(&mut self, cmd: Command) -> Res<Vec<String>> {
        self.flows.take();
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
                        m.insert_container("text", LoroText::new()).map_err(err)?;
                        m.insert("size", 12.0).map_err(err)?;
                        (
                            "text",
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
                if self.kind(id) == "text" && sizing.vertical != Size::Hug {
                    sizing.horizontal = match sizing.horizontal {
                        Size::Hug => Size::Fixed,
                        s => s,
                    };
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
            Command::EditText {
                id,
                range: [a, b],
                text,
            } => {
                let t = self.text(self.node(&id)?)?;
                if !(a <= b && b <= t.len_utf16()) {
                    return Err("range outside the text".into());
                }
                if b > a {
                    t.delete_utf16(a, b - a).map_err(err)?;
                }
                if !text.is_empty() {
                    t.insert_utf16(a, &text).map_err(err)?;
                }
                vec![]
            }
            Command::Format { id, range, props } => {
                let n = self.story(self.node(&id)?);
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
                    "text" if self.story(n) == n => self.detach(n, None, |s| s == id),
                    _ => Ok(()),
                })?;
                self.doc.get_list("textStyles").delete(i, 1).map_err(err)?;
                vec![]
            }
            Command::Set { id, mut props } => {
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
                self.unbind(id, |p| !set[p].is_null())?;
                // Text that hugs its width hugs its height: choosing hug for the width
                // makes it auto width, a fixed height makes it fixed.
                if let Some(s) = props.sizing.as_mut().filter(|s| {
                    self.kind(id) == "text" && s.horizontal == Size::Hug && s.vertical != Size::Hug
                }) {
                    if self.layout(id).sizing.horizontal == Size::Hug {
                        s.horizontal = Size::Fixed;
                    } else {
                        s.vertical = Size::Hug;
                    }
                }
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
                    self.unlink_all(id)?;
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
                if !matches!(self.kind(p).as_str(), "page" | "master" | "group" | "frame") {
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
                let mut clips = Vec::new();
                for id in self.sorted(&ids)?.into_iter().rev() {
                    let parent = self.tree.parent(id).ok_or("no parent")?;
                    let clip = self.clip(id);
                    let copy = self.paste(&clip, parent, self.index(id) + 1)?;
                    self.meta(copy).delete("overrideOf").map_err(err)?;
                    out.push(copy.to_string());
                    clips.push((clip, copy));
                }
                self.rethread(&clips)?;
                out.reverse();
                out
            }
            Command::SetDocument {
                raster_ppi,
                color_mode,
                facing_pages,
            } => {
                let m = self.doc.get_map("document");
                match facing_pages {
                    Some(true) if !self.facing_pages() => {
                        m.insert("facingPages", true).map_err(err)?;
                        self.split_masters()?;
                    }
                    Some(false) if self.facing_pages() => {
                        self.join_masters()?;
                        m.insert("facingPages", false).map_err(err)?;
                    }
                    _ => {}
                }
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
                let n = if prop == "size" { self.story(n) } else { n };
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
            Command::Paste { above, page } => {
                let above = self.sorted(&above)?.pop();
                let page = page.map(|p| self.sheet(&p)).transpose()?;
                let mut out = Vec::new();
                let mut pasted = Vec::new();
                for (i, (clip, from)) in self.clipboard.iter().enumerate() {
                    let (parent, index) = match above {
                        Some(a) => (
                            self.tree.parent(a).ok_or("no parent")?,
                            self.index(a) + 1 + i,
                        ),
                        None => {
                            let p = from
                                .filter(|&p| {
                                    self.node(&p.to_string()).is_ok()
                                        && page.is_none_or(|g| self.root(p) == g)
                                })
                                .or(page)
                                .or_else(|| self.pages().first().copied())
                                .ok_or("no page")?;
                            (p.into(), self.children(p).len())
                        }
                    };
                    let copy = self.paste(clip, parent, index)?;
                    self.meta(copy).delete("overrideOf").map_err(err)?;
                    out.push(copy.to_string());
                    pasted.push((clip, copy));
                }
                self.rethread(&pasted)?;
                out
            }
            Command::AddPage { after } => {
                let pages = self.pages();
                let like = match after {
                    Some(a) => self.page(&a)?,
                    None => *pages.last().ok_or("no page")?,
                };
                let p = self.tree.create(None).map_err(err)?;
                self.tree.mov_after(p, like).map_err(err)?;
                let (m, from) = (self.meta(p), self.meta(like));
                m.insert("kind", "page").map_err(err)?;
                for k in ["width", "height", "bleed"] {
                    m.insert(k, num(&from, k)).map_err(err)?;
                }
                if let Some(master) = value(&from, "master") {
                    m.insert("master", master).map_err(err)?;
                }
                vec![p.to_string()]
            }
            Command::DuplicatePage { id } => {
                let from = self.page(&id)?;
                let p = self.tree.create(None).map_err(err)?;
                self.tree.mov_after(p, from).map_err(err)?;
                let m = self.meta(p);
                for (k, v) in self.meta(from).get_value().into_map().unwrap().iter() {
                    m.insert(k, v.clone()).map_err(err)?;
                }
                let mut clips = Vec::new();
                for (i, c) in self.children(from).into_iter().enumerate() {
                    let clip = self.clip(c);
                    let copy = self.paste(&clip, p.into(), i)?;
                    clips.push((clip, copy));
                }
                self.rethread(&clips)?;
                vec![p.to_string()]
            }
            Command::SetPage {
                id,
                width,
                height,
                bleed,
            } => {
                let m = self.meta(self.sheet(&id)?);
                for (k, v) in [("width", width), ("height", height), ("bleed", bleed)] {
                    match v {
                        Some(v) if !(v >= 0.0 && v.is_finite()) => {
                            return Err(format!("{k} must not be negative"));
                        }
                        Some(v) => m.insert(k, v).map_err(err)?,
                        None => {}
                    }
                }
                vec![]
            }
            Command::DeletePage { id } => {
                let p = self.page(&id)?;
                if self.pages().len() == 1 {
                    return Err("a document keeps one page".into());
                }
                self.unlink_all(p)?;
                self.remove(p)?;
                vec![]
            }
            Command::MovePage { id, index } => {
                let p = self.page(&id)?;
                let others: Vec<_> = self.pages().into_iter().filter(|&o| o != p).collect();
                match others.get(index) {
                    Some(&o) => self.tree.mov_before(p, o).map_err(err)?,
                    None => self
                        .tree
                        .mov_after(p, *others.last().ok_or("no other page")?)
                        .map_err(err)?,
                }
                vec![]
            }
            Command::AddMaster { like } => {
                let like = match like {
                    Some(l) => self.sheet(&l)?,
                    None => *self.pages().first().ok_or("no page")?,
                };
                let names: Vec<String> = self.masters().into_iter().map(|m| self.name(m)).collect();
                let name = ('A'..='Z')
                    .map(|c| format!("{c}-Master"))
                    .find(|n| !names.contains(n))
                    .ok_or("no free master name")?;
                let p = self.tree.create(None).map_err(err)?;
                let (m, from) = (self.meta(p), self.meta(like));
                m.insert("kind", "master").map_err(err)?;
                m.insert("name", name).map_err(err)?;
                for k in ["width", "height", "bleed"] {
                    m.insert(k, num(&from, k)).map_err(err)?;
                }
                vec![p.to_string()]
            }
            Command::SetMaster { id, name } => {
                let m = self.master(&id)?;
                if self
                    .masters()
                    .into_iter()
                    .any(|o| o != m && self.name(o) == name)
                {
                    return Err(format!("a master named {name} exists"));
                }
                self.meta(m).insert("name", name).map_err(err)?;
                vec![]
            }
            Command::DeleteMaster { id } => {
                let m = self.master(&id)?;
                for p in self.pages() {
                    if self.master_of(p) == Some(m) {
                        self.meta(p).delete("master").map_err(err)?;
                    }
                }
                self.unlink_all(m)?;
                self.remove(m)?;
                vec![]
            }
            Command::UseMaster { page, master } => {
                let p = self.meta(self.page(&page)?);
                match master {
                    Some(m) => p
                        .insert("master", self.master(&m)?.to_string())
                        .map_err(err)?,
                    None => p.delete("master").map_err(err)?,
                }
                vec![]
            }
            Command::Override { page, id } => {
                let p = self.page(&page)?;
                let item = self.node(&id)?;
                let master = self.master_of(p).ok_or("the page has no master")?;
                if self.tree.parent(item) != Some(master.into()) {
                    return Err("not a layer of the page's master".into());
                }
                let mut detached = self.detached(p);
                if detached.contains(&id) {
                    return Err("already overridden".into());
                }
                let copy = self.paste(&self.clip(item), p.into(), 0)?;
                self.meta(copy)
                    .insert("overrideOf", id.clone())
                    .map_err(err)?;
                if self
                    .places()
                    .iter()
                    .any(|q| q.id == p && q.side == Some(Side::Left))
                {
                    let [x, y, w, h] = self.bounds(copy);
                    self.set_frame(copy, [x + num(&self.meta(master), "width"), y, w, h])?;
                }
                detached.push(id);
                self.meta(p)
                    .insert("detached", loro(detached)?)
                    .map_err(err)?;
                vec![copy.to_string()]
            }
            Command::ResetToMaster { ids } => {
                for id in ids {
                    if let Ok(p) = self.page(&id) {
                        let copies: Vec<TreeID> = self
                            .children(p)
                            .into_iter()
                            .filter(|&c| value(&self.meta(c), "overrideOf").is_some())
                            .collect();
                        for c in copies {
                            self.remove(c)?;
                        }
                        self.meta(p).delete("detached").map_err(err)?;
                        continue;
                    }
                    let c = self.node(&id)?;
                    let Some(of) = value(&self.meta(c), "overrideOf")
                        .and_then(|v| v.into_string().ok())
                        .map(|s| s.to_string())
                    else {
                        return Err(format!("{id} overrides no master layer"));
                    };
                    let p = self.root(c);
                    self.remove(c)?;
                    let mut detached = self.detached(p);
                    detached.retain(|d| *d != of);
                    self.meta(p)
                        .insert("detached", loro(detached)?)
                        .map_err(err)?;
                }
                vec![]
            }
            Command::Thread { from, to } => {
                let (a, b) = (self.node(&from)?, self.node(&to)?);
                if self.kind(a) != "text" || self.kind(b) != "text" {
                    return Err("only text frames thread".into());
                }
                if a == b || self.next_of(b).is_some() || self.prev_of(b).is_some() {
                    return Err("the frame is threaded already".into());
                }
                let (ra, rb) = (self.root(a), self.root(b));
                if ra != rb && (self.kind(ra) != "page" || self.kind(rb) != "page") {
                    return Err("frames thread within the pages or within one master".into());
                }
                let own = self.own_text(b)?;
                if own.len_utf16() > 0 {
                    let story = self.text(a)?;
                    let at = story.len_utf16();
                    story.insert_utf16(at, "\n").map_err(err)?;
                    let mut delta = vec![TextDelta::Retain {
                        retain: at + 1,
                        attributes: None,
                    }];
                    delta.extend(own.to_delta());
                    story.apply_delta(&delta).map_err(err)?;
                    own.delete_utf16(0, own.len_utf16()).map_err(err)?;
                }
                if let Some(c) = self.next_of(a) {
                    self.meta(b).insert("next", c.to_string()).map_err(err)?;
                }
                self.meta(a).insert("next", b.to_string()).map_err(err)?;
                let fills = value(&self.meta(self.story(a)), "fills");
                if let Some(f) = fills {
                    self.meta(b).insert("fills", f).map_err(err)?;
                }
                for f in [a, b] {
                    let old = self.layout(f).sizing;
                    let last = self.next_of(f).is_none();
                    let sizing = Sizing {
                        horizontal: match old.horizontal {
                            Size::Hug => Size::Fixed,
                            s => s,
                        },
                        vertical: match old.vertical {
                            Size::Hug if !last || old.horizontal == Size::Hug => Size::Fixed,
                            s => s,
                        },
                    };
                    if sizing != old {
                        self.meta(f).insert("sizing", loro(sizing)?).map_err(err)?;
                    }
                }
                vec![]
            }
            Command::Unthread { id } => {
                let a = self.node(&id)?;
                if self.next_of(a).is_some() {
                    self.meta(a).delete("next").map_err(err)?;
                }
                vec![]
            }
        };
        self.finish(out, history)
    }

    fn finish(&self, out: Vec<String>, history: bool) -> Res<Vec<String>> {
        let palette = self.palette();
        for p in self.tree.roots() {
            if !history && matches!(self.kind(p).as_str(), "page" | "master") {
                self.settle(p, &Modes::new(), &palette)?;
                self.lay_out(p)?;
            }
        }
        self.doc.commit();
        self.flows.replace(Some(Rc::new(self.flow_all())));
        Ok(out)
    }

    /// The story of the text layer `n` and the lines of it set in its frame.
    fn frame_lines(&self, n: TreeID) -> Res<(Rc<Story>, Vec<text::Line>)> {
        let flows = self.flows();
        let f = flows.get(&n).ok_or("not a text node")?;
        let tf = self.text_frame_of(n);
        let frame = self.bounds(n).map(|v| v as f32);
        let lines = text::lines(&f.story.text, &f.story.spans, frame, &tf, f.start);
        Ok((f.story.clone(), lines))
    }

    /// The story of the thread starting at `head`, its spans, and each frame with the
    /// byte where its text starts and where the text left over for the next begins.
    #[allow(clippy::type_complexity)]
    fn flow(&self, head: TreeID) -> Res<(String, Vec<Span>, Vec<(TreeID, usize, Option<usize>)>)> {
        let t = self.own_text(head)?.to_string();
        let v = serde_json::to_value(self.meta(head).get_deep_value()).map_err(err)?;
        let palette = self.palette();
        let modes = self.active_modes(head);
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let spans = self.spans(head, &v, &s);
        let mut out = Vec::new();
        let mut from = Some(0);
        for f in self.thread(head) {
            let start = from.unwrap_or(t.len());
            let next = from.and_then(|b| {
                let tf = self.text_frame_of(f);
                let frame = self.bounds(f).map(|v| v as f32);
                text::overflow(&t, &spans, frame, &tf, b)
            });
            out.push((f, start, next));
            from = next;
        }
        Ok((t, spans, out))
    }

    /// The frame of the thread of `n` that holds the byte `at`: the last one that
    /// gets text and starts at or before it.
    fn holding(&self, n: TreeID, at: usize) -> Res<TreeID> {
        let flows = self.flows();
        let mut f = flows.get(&n).ok_or("not a text node")?.head;
        let mut holder = f;
        while let Some(flow) = flows.get(&f) {
            let Some(next) = flow.next.filter(|_| flow.rest.is_some()) else {
                break;
            };
            if flows[&next].start <= at {
                holder = next;
            }
            f = next;
        }
        Ok(holder)
    }

    /// The frame of the thread of the text `id` that holds the UTF-16 `index`.
    pub fn text_frame(&self, id: &str, index: usize) -> Res<String> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        Ok(self.holding(n, byte_of(&t, index)?)?.to_string())
    }

    /// [x, top, bottom] in pt of a caret before the UTF-16 `index` of the text `id`,
    /// in the frame of its thread that holds it.
    pub fn caret(&self, id: &str, index: usize) -> Res<[f64; 3]> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        let at = byte_of(&t, index)?;
        let (story, lines) = self.frame_lines(self.holding(n, at)?)?;
        Ok(text::caret(&story.text, &lines, at).map(f64::from))
    }

    /// The UTF-16 index of the character boundary in the frame `id` nearest (x, y).
    pub fn text_index(&self, id: &str, x: f64, y: f64) -> Res<usize> {
        let n = self.node(id)?;
        let (story, lines) = self.frame_lines(n)?;
        let at = match lines.is_empty() {
            true => self.flows()[&n].start,
            false => text::index_at(&lines, x as f32, y as f32),
        };
        Ok(utf16_of(&story.text, at))
    }

    /// UTF-16 start and end of the line of the text `id` that holds `index`.
    pub fn text_line(&self, id: &str, index: usize) -> Res<[usize; 2]> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        let at = byte_of(&t, index)?;
        let (story, lines) = self.frame_lines(self.holding(n, at)?)?;
        Ok(match lines.get(text::line_of(&lines, at)) {
            Some(l) => [l.start, l.end].map(|b| utf16_of(&story.text, b)),
            None => [index; 2],
        })
    }

    /// The selection from `anchor` to `focus` in the text `id` as highlighted boxes in
    /// the frames of its thread on `page`, or a caret `caret_width` pt wide where they
    /// meet.
    pub fn text_overlay(
        &self,
        id: &str,
        anchor: usize,
        focus: usize,
        caret_width: f32,
        page: &str,
    ) -> Res<Vec<Op>> {
        let n = self.node(id)?;
        let page = self.sheet(page)?;
        let t = self.text(n)?.to_string();
        let [a, b] = [anchor.min(focus), anchor.max(focus)].map(|i| byte_of(&t, i));
        let (a, b) = (a?, b?);
        let solid = |color| Paint::Solid {
            color,
            ink: Ink::Rgb,
        };
        if a == b {
            let f = self.holding(n, a)?;
            if self.root(f) != page {
                return Ok(Vec::new());
            }
            let (story, lines) = self.frame_lines(f)?;
            let [x, top, bottom] = text::caret(&story.text, &lines, a);
            return Ok(vec![Op::FillPath {
                paint: solid([0.0, 0.0, 0.0, 1.0]),
                path: rect(x - caret_width / 2.0, top, caret_width, bottom - top),
            }]);
        }
        let mut ops = Vec::new();
        for f in self.thread(self.story(n)) {
            if self.root(f) != page {
                continue;
            }
            let (story, lines) = self.frame_lines(f)?;
            let t = &story.text;
            ops.extend(
                lines
                    .iter()
                    .filter(|l| a <= l.end && b > l.start || a == l.start && b >= l.start)
                    .map(|l| {
                        let x0 = text::x_at(t, l, a.max(l.start));
                        let mut x1 = text::x_at(t, l, b.min(l.end));
                        if b > l.end {
                            x1 = x1.max(x0 + (l.bottom - l.top) / 4.0);
                        }
                        Op::FillPath {
                            paint: solid(SELECTION),
                            path: rect(x0, l.top, x1 - x0, l.bottom - l.top),
                        }
                    }),
            );
        }
        Ok(ops)
    }

    /// The layer's own text, which is its story's unless it follows another frame.
    fn own_text(&self, id: TreeID) -> Res<LoroText> {
        container(&self.meta(id), "text")
            .and_then(|c| c.into_text().ok())
            .ok_or_else(|| "not a text node".into())
    }

    /// The text layer that follows `id` in its thread.
    fn next_of(&self, id: TreeID) -> Option<TreeID> {
        let v = value(&self.meta(id), "next")?.into_string().ok()?;
        let n = self.node(&v).ok()?;
        (self.kind(n) == "text").then_some(n)
    }

    /// The text layer that `id` follows in its thread.
    fn prev_of(&self, id: TreeID) -> Option<TreeID> {
        if let Some(flows) = &*self.flows.borrow() {
            return flows.get(&id).and_then(|f| f.prev);
        }
        let mut all = Vec::new();
        for r in self.pages().into_iter().chain(self.masters()) {
            self.walk(r, &mut all);
        }
        all.into_iter()
            .find(|&n| n != id && self.kind(n) == "text" && self.next_of(n) == Some(id))
    }

    /// The first frame of the thread of `id`, which holds the story.
    fn story(&self, id: TreeID) -> TreeID {
        let mut head = id;
        let mut seen = vec![id];
        while let Some(p) = self.prev_of(head).filter(|p| !seen.contains(p)) {
            seen.push(p);
            head = p;
        }
        head
    }

    /// The frames of the thread from `head` on.
    fn thread(&self, head: TreeID) -> Vec<TreeID> {
        let mut out = vec![head];
        while let Some(n) = self
            .next_of(*out.last().unwrap())
            .filter(|n| !out.contains(n))
        {
            out.push(n);
        }
        out
    }

    /// Takes the text layers in `id`'s subtree out of their threads, which close up;
    /// a story whose first frame goes moves on to the next frame.
    fn unlink_all(&self, id: TreeID) -> Res<()> {
        let mut all = Vec::new();
        self.walk(id, &mut all);
        for n in all.into_iter().filter(|&n| self.kind(n) == "text") {
            let next = self.next_of(n);
            match (self.prev_of(n), next) {
                (Some(p), Some(x)) => self.meta(p).insert("next", x.to_string()).map_err(err)?,
                (Some(p), None) => self.meta(p).delete("next").map_err(err)?,
                (None, Some(x)) => {
                    let delta = self.own_text(n)?.to_delta();
                    let t = self
                        .meta(x)
                        .insert_container("text", LoroText::new())
                        .map_err(err)?;
                    t.apply_delta(&delta).map_err(err)?;
                    let (from, to) = (self.meta(n), self.meta(x));
                    for k in STORY {
                        match value(&from, k) {
                            Some(v) => to.insert(k, v).map_err(err)?,
                            None => to.delete(k).map_err(err)?,
                        }
                    }
                    if let Some(size) = self.bindings(n).get("size") {
                        let mut b = self.bindings(x);
                        b.insert("size".into(), size.clone());
                        to.insert("bindings", loro(b)?).map_err(err)?;
                    }
                }
                (None, None) => {}
            }
            if next.is_some() {
                self.meta(n).delete("next").map_err(err)?;
            }
        }
        Ok(())
    }

    /// The story of the text layer `id`, held by the first frame of its thread.
    fn text(&self, id: TreeID) -> Res<LoroText> {
        self.own_text(self.story(id))
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

    /// Fits hugging text and lays out the auto layout frames in `id`'s subtree,
    /// innermost first.
    fn lay_out(&self, id: TreeID) -> Res<()> {
        let kids = self.children(id);
        for &c in &kids {
            self.lay_out(c)?;
        }
        if self.kind(id) == "text" {
            return self.fit(id);
        }
        let l = self.layout(id);
        let Some((horizontal, pad)) = l.axes().filter(|_| self.kind(id) == "frame") else {
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

    /// Sets the hugging sides of a text layer to what its text needs, from where its
    /// thread's story reaches it.
    fn fit(&self, id: TreeID) -> Res<()> {
        let Sizing {
            horizontal,
            vertical,
        } = self.layout(id).sizing;
        if vertical != Size::Hug || self.next_of(id).is_some() {
            return Ok(());
        }
        let (t, spans, flows) = self.flow(self.story(id))?;
        let from = flows.iter().find(|f| f.0 == id).map_or(0, |f| f.1);
        let tf = self.text_frame_of(id);
        let old = self.bounds(id);
        let [x, y, w, _] = old;
        let auto_width = horizontal == Size::Hug;
        let [nw, nh] = text::measure(&t, &spans, &tf, (!auto_width).then_some(w as f32), from);
        let new = [x, y, if auto_width { nw.into() } else { w }, nh.into()];
        if new != old {
            self.set_frame(id, new)?;
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

    /// How the story of every thread flows through its frames.
    /// The flows through all text layers, kept since the last command.
    fn flows(&self) -> Rc<HashMap<TreeID, Flow>> {
        match &*self.flows.borrow() {
            Some(f) => f.clone(),
            None => Rc::new(self.flow_all()),
        }
    }

    fn flow_all(&self) -> HashMap<TreeID, Flow> {
        let mut all = Vec::new();
        for r in self.pages().into_iter().chain(self.masters()) {
            self.walk(r, &mut all);
        }
        let texts: Vec<TreeID> = all
            .into_iter()
            .filter(|&n| self.kind(n) == "text")
            .collect();
        let mut prev = HashMap::new();
        for &n in &texts {
            if let Some(x) = self.next_of(n) {
                prev.insert(x, n);
            }
        }
        let mut out = HashMap::new();
        for &head in texts.iter().filter(|n| !prev.contains_key(n)) {
            let Ok((text, spans, frames)) = self.flow(head) else {
                continue;
            };
            let story = Rc::new(Story { text, spans });
            for (i, &(f, start, next)) in frames.iter().enumerate() {
                out.insert(
                    f,
                    Flow {
                        head,
                        story: story.clone(),
                        start,
                        end: next.unwrap_or(story.text.len()),
                        rest: next,
                        prev: i.checked_sub(1).map(|j| frames[j].0),
                        next: frames.get(i + 1).map(|n| n.0),
                        overset: i + 1 == frames.len() && next.is_some(),
                    },
                );
            }
        }
        out
    }

    pub fn snapshot(&self) -> Snapshot {
        let palette = self.palette();
        let flows = self.flows();
        let sheet = |p: TreeID| {
            let m = self.meta(p);
            let v = serde_json::to_value(m.get_value()).unwrap_or_default();
            let modes = self.modes(p);
            Page {
                id: p.to_string(),
                name: v["name"].as_str().unwrap_or_default().into(),
                width: num(&m, "width"),
                height: num(&m, "height"),
                bleed: num(&m, "bleed"),
                side: None,
                x: 0.0,
                master: self.master_of(p).map(|m| m.to_string()),
                detached: serde_json::from_value(v["detached"].clone()).unwrap_or_default(),
                children: self
                    .children(p)
                    .into_iter()
                    .map(|c| self.snap(c, &modes, &palette, &flows))
                    .collect(),
                modes,
            }
        };
        let facing_pages = self.facing_pages();
        let mut pages: Vec<Page> = self.pages().into_iter().map(sheet).collect();
        let places = self.places();
        for (p, place) in pages.iter_mut().zip(&places) {
            (p.side, p.x) = (place.side, place.x);
        }
        let spreads = spreads(pages.len(), facing_pages);
        let stories = flows
            .values()
            .map(|f| (f.head.to_string(), f.story.clone()))
            .collect();
        Snapshot {
            stories,
            spreads: spreads
                .iter()
                .map(|s| s.iter().map(|&i| pages[i].id.clone()).collect())
                .collect(),
            pages,
            masters: self.masters().into_iter().map(sheet).collect(),
            facing_pages,
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

    /// Where each page sits: its spread, its side and the x of its left edge on the
    /// spread, whose spine is at 0.
    fn places(&self) -> Vec<Place> {
        let pages = self.pages();
        let facing = self.facing_pages();
        let mut out: Vec<Place> = pages
            .iter()
            .map(|&id| Place {
                id,
                spread: 0,
                side: None,
                x: 0.0,
            })
            .collect();
        for (k, s) in spreads(pages.len(), facing).iter().enumerate() {
            for (j, &i) in s.iter().enumerate() {
                let side = facing.then_some(match (s.len(), i, j) {
                    (1, 0, _) | (2, _, 1) => Side::Right,
                    _ => Side::Left,
                });
                out[i].spread = k;
                out[i].side = side;
                if side == Some(Side::Left) {
                    out[i].x = -num(&self.meta(pages[i]), "width");
                }
            }
        }
        out
    }

    /// Makes each master a spread with a copy of its layers on its left page, which
    /// the left pages that override a layer override instead.
    fn split_masters(&self) -> Res<()> {
        let lefts = self.left_pages();
        for m in self.masters() {
            let w = num(&self.meta(m), "width");
            let mut clips = Vec::new();
            for c in self.children(m) {
                let clip = self.clip(c);
                let copy = self.paste(&clip, m.into(), self.index(c) + 1)?;
                clips.push((clip, copy));
                let [x, y, cw, ch] = self.bounds(copy);
                self.set_frame(copy, [x - w, y, cw, ch])?;
                self.meta(copy)
                    .insert("leftOf", c.to_string())
                    .map_err(err)?;
                self.relink(&lefts, m, c, copy)?;
            }
            self.rethread(&clips)?;
        }
        Ok(())
    }

    /// Makes each master one page again without the layers of its left page; left
    /// pages that override one of its copies override the original again.
    fn join_masters(&self) -> Res<()> {
        let lefts = self.left_pages();
        for m in self.masters() {
            for c in self.children(m) {
                let [x, _, w, _] = self.bounds(c);
                if x + w > 0.0 {
                    continue;
                }
                let of = value(&self.meta(c), "leftOf").and_then(|v| v.into_string().ok());
                if let Some(o) = of.and_then(|o| self.node(&o).ok()) {
                    self.relink(&lefts, m, c, o)?;
                }
                self.unlink_all(c)?;
                self.remove(c)?;
            }
        }
        Ok(())
    }

    fn left_pages(&self) -> Vec<TreeID> {
        let places = self.places().into_iter();
        places
            .filter(|p| p.side == Some(Side::Left))
            .map(|p| p.id)
            .collect()
    }

    /// Makes the pages among `pages` that use the master `m` and override its layer
    /// `from` override `to` instead.
    fn relink(&self, pages: &[TreeID], m: TreeID, from: TreeID, to: TreeID) -> Res<()> {
        let (from, to) = (from.to_string(), to.to_string());
        for &p in pages.iter().filter(|&&p| self.master_of(p) == Some(m)) {
            let mut detached = self.detached(p);
            if !detached.contains(&from) {
                continue;
            }
            for d in detached.iter_mut().filter(|d| **d == from) {
                *d = to.clone();
            }
            self.meta(p)
                .insert("detached", loro(detached)?)
                .map_err(err)?;
            for c in self.children(p) {
                if value(&self.meta(c), "overrideOf").and_then(|v| v.into_string().ok())
                    == Some(from.clone().into())
                {
                    self.meta(c).insert("overrideOf", to.clone()).map_err(err)?;
                }
            }
        }
        Ok(())
    }

    fn facing_pages(&self) -> bool {
        value(&self.doc.get_map("document"), "facingPages") == Some(LoroValue::Bool(true))
    }

    fn color_mode(&self) -> ColorMode {
        value(&self.doc.get_map("document"), "colorMode")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    /// The display list of the page `id`, empty when there is none.
    pub fn render(&self, id: &str) -> Vec<Op> {
        let snap = self.snapshot();
        let sheets = || snap.pages.iter().chain(&snap.masters);
        let Some(p) = sheets().find(|p| p.id == id) else {
            return Vec::new();
        };
        let mut ops = vec![page_op(p)];
        self.draw_sheet(&snap, p, None, &mut ops);
        ops
    }

    /// The document as a PDF, every page from one snapshot.
    pub fn pdf(&self) -> Vec<u8> {
        let snap = self.snapshot();
        let pages: Vec<_> = snap.pages.iter().map(|p| self.print(&snap, p)).collect();
        crate::pdf::pdf(&pages, snap.raster_ppi as f32, snap.color_mode)
    }

    /// The display list of the page `p` as it prints: with the layers of the other
    /// page of its spread, so that one across the spine prints on both.
    fn print(&self, snap: &Snapshot, p: &Page) -> Vec<Op> {
        let mut ops = vec![page_op(p)];
        let spread = snap.spreads.iter().find(|s| s.contains(&p.id));
        for q in spread.into_iter().flatten() {
            let q = snap.pages.iter().find(|o| o.id == *q).unwrap();
            let dx = q.x - p.x;
            let b = p.bleed;
            let window =
                (q.id != p.id).then_some([-b - dx, -b, p.width + 2.0 * b, p.height + 2.0 * b]);
            let mut own = Vec::new();
            self.draw_sheet(snap, q, window, &mut own);
            ops.extend(shift(own, dx as f32, 0.0));
        }
        ops
    }

    /// The master layers a page shows and its own layers, those that reach into
    /// `window` when there is one. A page of a spread shows its side of the master
    /// spread up to the spine.
    fn draw_sheet(&self, snap: &Snapshot, p: &Page, window: Option<[f64; 4]>, ops: &mut Vec<Op>) {
        let into = |n: &Node, dx: f64, r: [f64; 4]| {
            let [x, y, w, h] = reach(n);
            x + dx < r[0] + r[2] && x + dx + w > r[0] && y < r[1] + r[3] && y + h > r[1]
        };
        if let Some(m) = p
            .master
            .as_ref()
            .and_then(|m| snap.masters.iter().find(|s| s.id == *m))
        {
            let (w, h, b) = (p.width, p.height, p.bleed);
            let half = match p.side {
                Some(Side::Left) => [-b, -b, w + b, h + 2.0 * b],
                Some(Side::Right) => [0.0, -b, w + b, h + 2.0 * b],
                None => [-b, -b, w + 2.0 * b, h + 2.0 * b],
            };
            let dx = master_dx(m, p);
            let mut layers = self.master_layers(m, p);
            if !layers.iter().any(|n| n.style.mask) {
                layers.retain(|n| into(n, dx, half) && window.is_none_or(|r| into(n, dx, r)));
            }
            let mut shown = Vec::new();
            draw_all(&layers, &mut shown, &snap.palette);
            let clip = p.side.is_some() && !shown.is_empty();
            if clip {
                let [x, y, w, h] = half.map(|v| v as f32);
                ops.push(Op::PushClip {
                    path: rect(x, y, w, h),
                    invert: false,
                });
            }
            ops.extend(shift(shown, dx as f32, 0.0));
            if clip {
                ops.push(Op::PopClip);
            }
        }
        // A mask masks the layers above it, so layers go only all together then.
        match window {
            Some(r) if !p.children.iter().any(|n| n.style.mask) => {
                for n in p.children.iter().filter(|n| into(n, 0.0, r)) {
                    draw(n, ops, &snap.palette);
                }
            }
            _ => draw_all(&p.children, ops, &snap.palette),
        }
    }

    /// The layers of the master `m` that the page `p` shows, in its modes.
    fn master_layers(&self, m: &Page, p: &Page) -> Vec<Node> {
        let palette = self.palette();
        let mut modes = m.modes.clone();
        modes.extend(p.modes.clone());
        let Ok(id) = self.sheet(&m.id) else {
            return Vec::new();
        };
        let flows = self.flows();
        let mut layers: Vec<Node> = self
            .children(id)
            .into_iter()
            .filter(|c| !p.detached.contains(&c.to_string()))
            .map(|c| self.snap(c, &modes, &palette, &flows))
            .collect();
        if let Ok(page) = self.page(&p.id) {
            renumber(&mut layers, &self.number(page));
        }
        layers
    }

    /// How the text layer `n` sets its text, with the number of the page it is on.
    fn text_frame_of(&self, n: TreeID) -> TextFrame {
        let v = serde_json::to_value(self.meta(n).get_deep_value()).unwrap_or_default();
        TextFrame {
            number: self.number(self.root(n)),
            ..serde_json::from_value(v).unwrap_or_default()
        }
    }

    /// What a page number on the page or master `root` shows: the page's number, or
    /// the master's prefix.
    fn number(&self, root: TreeID) -> String {
        match self.pages().iter().position(|&p| p == root) {
            Some(i) => (i + 1).to_string(),
            None => prefix(&self.name(root)),
        }
    }

    /// The layer of the master of the page `page` at (x, y) that the page shows.
    pub fn master_hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Option<String> {
        let snap = self.snapshot();
        let p = snap.pages.iter().find(|p| p.id == page)?;
        let m = snap
            .masters
            .iter()
            .find(|m| Some(&m.id) == p.master.as_ref())?;
        let within = match p.side {
            Some(Side::Left) => x <= p.width,
            Some(Side::Right) => x >= 0.0,
            None => true,
        };
        let mut path = Vec::new();
        if within {
            let x = x - master_dx(m, p);
            hit(&self.master_layers(m, p), x, y, tolerance, &mut path);
        }
        path.into_iter().next()
    }

    pub fn hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        let mut path = Vec::new();
        let snap = self.snapshot();
        if let Some(p) = snap
            .pages
            .iter()
            .chain(&snap.masters)
            .find(|p| p.id == page)
        {
            hit(&p.children, x, y, tolerance, &mut path);
        }
        path
    }

    fn snap(
        &self,
        id: TreeID,
        inherited: &Modes,
        palette: &Palette,
        flows: &HashMap<TreeID, Flow>,
    ) -> Node {
        let m = self.meta(id);
        let v = serde_json::to_value(m.get_deep_value()).unwrap_or_default();
        let modes = self.modes(id);
        let mut active_modes = inherited.clone();
        active_modes.extend(modes.clone());
        let children = || {
            self.children(id)
                .into_iter()
                .map(|c| self.snap(c, &active_modes, palette, flows))
                .collect()
        };
        let kind = match self.kind(id).as_str() {
            "text" => match flows.get(&id) {
                Some(f) => {
                    let text = &f.story.text;
                    Kind::Text {
                        content: f.story.clone(),
                        frame: self.text_frame_of(id),
                        story: f.head.to_string(),
                        start: utf16_of(text, f.start),
                        end: utf16_of(text, f.end),
                        prev: f.prev.map(|p| p.to_string()),
                        next: f.next.map(|n| n.to_string()),
                        overset: f.overset,
                        from: f.start,
                    }
                }
                None => Kind::Text {
                    content: Rc::new(Story {
                        text: v["text"].as_str().unwrap_or_default().into(),
                        spans: self.spans(
                            id,
                            &v,
                            &Scope {
                                palette,
                                modes: &active_modes,
                            },
                        ),
                    }),
                    frame: self.text_frame_of(id),
                    story: id.to_string(),
                    start: 0,
                    end: 0,
                    prev: None,
                    next: None,
                    overset: false,
                    from: 0,
                },
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
                Kind::Text { content, from, .. } if content.text[*from..].is_empty() => {
                    "Text".into()
                }
                Kind::Text { content, from, .. } => content.text[*from..]
                    .chars()
                    .take(40)
                    .map(|c| if c == text::PAGE_NUMBER { '#' } else { c })
                    .collect(),
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
            override_of: v["overrideOf"].as_str().map(String::from),
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
                    hyphenate: p.hyphenate,
                    lang: p.lang,
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
        let [least_w, least_h] = self.least_size(id);
        let (w, h) = (w.max(least_w), h.max(least_h));
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

    /// The least [w, h] of `id`: 0.01 mm for a box, which would vanish at 0, but 0 for a
    /// path, whose bounds follow its points, and for a side of a text that hugs it.
    fn least_size(&self, id: TreeID) -> [f64; 2] {
        let least = 0.01 * MM;
        match self.kind(id).as_str() {
            "shape" if value(&self.meta(id), "shape") == Some("path".into()) => [0.0; 2],
            "shape" | "frame" => [least; 2],
            "text" => {
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
        let mut meta = self.meta(id).get_deep_value();
        let head = self.story(id);
        if let LoroValue::Map(m) = &mut meta
            && head != id
        {
            let m = m.make_mut();
            for k in STORY {
                match value(&self.meta(head), k) {
                    Some(v) => m.insert(k.into(), v),
                    None => m.remove(k),
                };
            }
        }
        Clip {
            id,
            meta,
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
                ("next", _) => {}
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

    /// Threads the copies of `pasted`, clips with the layer pasted from each, as their
    /// originals are threaded among them. A copy that follows another copy leaves the
    /// story, which each clip holds whole, to the first copy of its thread.
    fn rethread<C: std::borrow::Borrow<Clip>>(&self, pasted: &[(C, TreeID)]) -> Res<()> {
        let mut pairs = Vec::new();
        for (clip, copy) in pasted {
            self.pair_up(clip.borrow(), *copy, &mut pairs);
        }
        let copies: HashMap<String, TreeID> = pairs
            .iter()
            .map(|(old, new, _)| (old.to_string(), *new))
            .collect();
        for (_, new, next) in pairs {
            if let Some(&to) = next.and_then(|n| copies.get(&n)) {
                self.meta(new).insert("next", to.to_string()).map_err(err)?;
                let own = self.own_text(to)?;
                own.delete_utf16(0, own.len_utf16()).map_err(err)?;
            }
        }
        Ok(())
    }

    /// Each layer copied into `clip` with its copy under `copy` and the layer it threads to.
    fn pair_up(&self, clip: &Clip, copy: TreeID, out: &mut Vec<(TreeID, TreeID, Option<String>)>) {
        let next = clip
            .meta
            .as_map()
            .and_then(|m| m.get("next")?.as_string().cloned());
        out.push((clip.id, copy, next.map(|n| n.to_string())));
        for (c, n) in clip.children.iter().zip(self.children(copy)) {
            self.pair_up(c, n, out);
        }
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

    /// The pages in order.
    fn pages(&self) -> Vec<TreeID> {
        let roots = self.tree.roots().into_iter();
        roots.filter(|&r| self.kind(r) == "page").collect()
    }

    fn masters(&self) -> Vec<TreeID> {
        let roots = self.tree.roots().into_iter();
        roots.filter(|&r| self.kind(r) == "master").collect()
    }

    fn page(&self, id: &str) -> Res<TreeID> {
        self.root_of_kind(id, "page")
    }

    fn master(&self, id: &str) -> Res<TreeID> {
        self.root_of_kind(id, "master")
    }

    /// A page or a master.
    fn sheet(&self, id: &str) -> Res<TreeID> {
        self.page(id).or_else(|_| self.master(id))
    }

    fn root_of_kind(&self, id: &str, kind: &str) -> Res<TreeID> {
        let t = TreeID::try_from(id).map_err(err)?;
        match self.tree.parent(t) {
            Some(TreeParentId::Root) if self.kind(t) == kind => Ok(t),
            _ => Err(format!("no {kind} {id}")),
        }
    }

    /// The master the page `p` uses, if it is still there.
    fn master_of(&self, p: TreeID) -> Option<TreeID> {
        let v = value(&self.meta(p), "master")?;
        self.master(&v.into_string().ok()?).ok()
    }

    fn detached(&self, p: TreeID) -> Vec<String> {
        value(&self.meta(p), "detached")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
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
                _ if matches!(self.kind(root).as_str(), "page" | "master") => return Ok(t),
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

    fn name(&self, id: TreeID) -> String {
        value(&self.meta(id), "name")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default()
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
/// How far the master spread `m` moves to show its side on the page `p`: a left
/// page shows the master's left page.
fn master_dx(m: &Page, p: &Page) -> f64 {
    match p.side {
        Some(Side::Left) => m.width,
        _ => 0.0,
    }
}

/// The box of a layer with its strokes and effects: what it can draw into.
fn reach(n: &Node) -> [f64; 4] {
    let s = &n.style;
    let mut m = 3.0 * f64::from(s.stroke_weight);
    for e in &s.effects {
        m = m.max(f64::from(e.x.abs().max(e.y.abs()) + 3.0 * e.radius));
    }
    [n.x - m, n.y - m, n.w + 2.0 * m, n.h + 2.0 * m]
}

/// Sets the page number of the text layers among `nodes` to `number` and fits the
/// hugging sides of each to it, as `Doc::fit` does.
fn renumber(nodes: &mut [Node], number: &str) {
    for n in nodes {
        let Sizing {
            horizontal,
            vertical,
        } = n.layout.sizing;
        match &mut n.kind {
            Kind::Text {
                content,
                frame,
                next,
                from,
                ..
            } => {
                frame.number = number.into();
                if vertical == Size::Hug && next.is_none() {
                    let auto_width = horizontal == Size::Hug;
                    let w = (!auto_width).then_some(n.w as f32);
                    let [w, h] = text::measure(&content.text, &content.spans, frame, w, *from);
                    if auto_width {
                        n.w = w.into();
                    }
                    n.h = h.into();
                }
            }
            Kind::Group { children } | Kind::Frame { children, .. } => renumber(children, number),
            Kind::Shape(_) => {}
        }
    }
}

/// The letters a master's page numbers show: the prefix of "A-Master", or the first
/// letter, as the pages panel shows it.
pub fn prefix(name: &str) -> String {
    let head: String = name
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let short = (1..=3).contains(&head.chars().count()) && name[head.len()..].starts_with('-');
    let p = if short {
        head
    } else {
        name.chars().take(1).collect()
    };
    p.to_uppercase()
}

fn page_op(p: &Page) -> Op {
    Op::Page {
        width: p.width as f32,
        height: p.height as f32,
        bleed: p.bleed as f32,
    }
}

/// The indices of the pages of each spread among `n` pages, as `Snapshot::spreads`.
fn spreads(n: usize, facing: bool) -> Vec<Vec<usize>> {
    match facing {
        false => (0..n).map(|i| vec![i]).collect(),
        true => (0..n.min(1))
            .map(|i| vec![i])
            .chain((1..n).step_by(2).map(|i| (i..(i + 2).min(n)).collect()))
            .collect(),
    }
}

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
        Kind::Text {
            content,
            frame: tf,
            from,
            ..
        } => item(
            ops,
            text::draw(
                &content.text,
                &content.spans,
                &n.style.fills,
                frame,
                tf,
                s,
                *from,
            ),
        ),
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

/// The byte offset in `s` of the UTF-16 `index`.
fn byte_of(s: &str, index: usize) -> Res<usize> {
    let mut units = 0;
    for (b, c) in s.char_indices() {
        if units >= index {
            return Ok(b);
        }
        units += c.len_utf16();
    }
    if units >= index {
        Ok(s.len())
    } else {
        Err("index outside the text".into())
    }
}

/// The UTF-16 index of the byte offset `byte` in `s`.
fn utf16_of(s: &str, byte: usize) -> usize {
    s[..byte.min(s.len())].encode_utf16().count()
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
    use crate::display_list::{CUBIC, Paint};

    fn page(d: &Doc) -> Page {
        d.snapshot().pages.remove(0)
    }

    fn page_ops(d: &Doc) -> Vec<Op> {
        d.render(&page(d).id)
    }

    fn hits(d: &Doc, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        d.hit(&page(d).id, x, y, tolerance)
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
        let text = |n: &Node| match &n.kind {
            Kind::Text { content, frame, .. } => (content.clone(), frame.clone()),
            k => panic!("not text: {k:?}"),
        };
        assert_eq!(text(&children(dup)[0]), text(&children(orig)[0]));
        assert_eq!(frame(&children(dup)[0]), frame(&children(orig)[0]));
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
                page: None,
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
        let again = d
            .apply(Command::Paste {
                above: vec![],
                page: None,
            })
            .unwrap();
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
        assert!(
            matches!(&page(&d).children[1].kind, Kind::Text { content, .. } if content.text == "Hallo")
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
        assert_eq!(hits(&d, 7.0, 7.0, 0.0), [b]);
        assert_eq!(hits(&d, 2.0, 2.0, 0.0), [a]);
        assert!(hits(&d, 20.0, 2.0, 0.0).is_empty());
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
        assert_eq!(hits(&d, 5.0, 5.0, 0.0), [f.clone(), g.clone(), a]);
        assert_eq!(hits(&d, 30.0, 30.0, 0.0), [f]);
    }

    #[test]
    fn clipped_content_is_only_hit_inside_its_frame() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let a = create(&mut d, &f, NewKind::Rect, [5.0, 5.0, 20.0, 20.0]);
        assert!(hits(&d, 15.0, 15.0, 0.0).is_empty());
        assert_eq!(hits(&d, 8.0, 8.0, 0.0), [f.clone(), a.clone()]);
        d.apply(Command::Set {
            id: f.clone(),
            props: Props {
                clip: Some(false),
                ..Props::default()
            },
        })
        .unwrap();
        assert_eq!(hits(&d, 15.0, 15.0, 0.0), [f, a]);
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
        let ops = page_ops(&Doc::new());
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
        assert!(Doc::new().render("9@9").is_empty());
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
        assert_eq!(hits(&d, 5.0, 5.0, 0.0), [e]);
        assert!(hits(&d, 0.5, 0.5, 0.0).is_empty());

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
        assert_eq!(hits(&d, 15.0, 6.0, 0.5), [l]);
        assert!(hits(&d, 12.0, 8.0, 0.5).is_empty());
        assert!(hits(&d, 15.0, 6.0, 0.0).is_empty());
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
        let ops = page_ops(&d);
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
        assert!(matches!(page_ops(&d)[2], Op::PushClip { invert: true, .. }));
    }

    #[test]
    fn arrows_add_a_head_to_the_stroke() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Arrow, [0.0, 0.0, 10.0, 0.0]);
        assert_eq!(page(&d).children[0].name, "Arrow");
        let Op::StrokePath { path, .. } = &page_ops(&d)[2] else {
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
            !page_ops(&d)
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
        let ops = page_ops(&d);
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
        let ops = page_ops(&d);
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
        assert_eq!(hits(&d, 5.0, 5.0, 0.0), [above]);
        assert_eq!(hits(&d, 0.5, 0.5, 0.0), [below]);
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
        } = &page_ops(&d)[2]
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
            facing_pages: None,
        }
    }

    fn cmyk(d: &mut Doc) {
        d.apply(Command::SetDocument {
            raster_ppi: None,
            color_mode: Some(ColorMode::Cmyk),
            facing_pages: None,
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
        let colors: Vec<[f32; 4]> = page_ops(&d)
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

    #[test]
    fn a_range_bound_to_a_colour_variable_takes_the_mode_of_the_page_it_shows_on() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let (c, _) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let ink = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        set_value(&mut d, &ink, &dark, Value::Color(Color::Rgb(0x0000ffff))).unwrap();
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p2, [0.0, 0.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        let red = TextProps {
            fill: Some(var(&ink)),
            ..TextProps::default()
        };
        format(&mut d, &a, Some([0, 5]), red).unwrap();
        let m = add_master(&mut d);
        let t = create(&mut d, &m, NewKind::Text, [0.0, 200.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Yo");
        let red = TextProps {
            fill: Some(var(&ink)),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 2]), red).unwrap();
        use_master(&mut d, &p2, Some(&m)).unwrap();
        use_mode(&mut d, &p2, &c, Some(&dark));
        let glyphs = |d: &Doc, p: &str| {
            d.render(p)
                .into_iter()
                .filter_map(|o| match o {
                    Op::GlyphRun {
                        paint: Paint::Solid { color, .. },
                        ..
                    } => Some(color),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(glyphs(&d, &p1), [[1.0, 0.0, 0.0, 1.0]]);
        assert_eq!(glyphs(&d, &p2), [[0.0, 0.0, 1.0, 1.0]; 2]);
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
            Kind::Text { content, .. } => content.spans.clone(),
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
            hyphenate: Some(true),
            lang: Some(Lang::De),
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
        assert_eq!((s[1].attrs.hyphenate, s[1].attrs.lang), (true, Lang::De));
        assert_eq!((s[2].attrs.hyphenate, s[2].attrs.lang), (false, Lang::En));
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
        let colors: Vec<_> = page_ops(&d)
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

    fn first_glyph(d: &Doc) -> [f32; 2] {
        page_ops(d)
            .into_iter()
            .find_map(|op| match op {
                Op::GlyphRun { positions, .. } => Some([positions[0], positions[1]]),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn a_text_layer_sets_its_text_inside_its_insets_and_rejects_bad_frames() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi");
        let [x0, y0] = first_glyph(&d);
        set(
            &mut d,
            &t,
            Props {
                inset_left: Some(4.0),
                inset_top: Some(6.0),
                columns: Some(3),
                vertical_align: Some(VerticalAlign::Top),
                ..Props::default()
            },
        );
        let [x1, y1] = first_glyph(&d);
        assert!((x1 - x0 - 4.0).abs() < 1e-4 && (y1 - y0 - 6.0).abs() < 1e-4);
        let Kind::Text { frame, .. } = page(&d).children.pop().unwrap().kind else {
            panic!("not text");
        };
        assert_eq!(frame.columns, 3);
        for bad in [
            Props {
                columns: Some(0),
                ..Props::default()
            },
            Props {
                inset_right: Some(-1.0),
                ..Props::default()
            },
            Props {
                baseline_grid: Some(-2.0),
                ..Props::default()
            },
        ] {
            assert!(
                d.apply(Command::Set {
                    id: t.clone(),
                    props: bad
                })
                .is_err()
            );
        }
    }

    /// Auto line height of the bundled font at 12 pt.
    const LEADING: f64 = 16.45;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.05
    }

    fn set_frame(d: &mut Doc, id: &str, [x, y, w, h]: [f64; 4]) {
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

    fn set_text(d: &mut Doc, id: &str, text: &str) {
        d.apply(Command::SetText {
            id: id.into(),
            text: text.into(),
        })
        .unwrap();
    }

    fn node_sizing(d: &Doc, id: &str) -> Sizing {
        fn find(nodes: &[Node], id: &str) -> Option<Sizing> {
            nodes.iter().find_map(|n| {
                (n.id == id)
                    .then_some(n.layout.sizing)
                    .or_else(|| find(children(n), id))
            })
        }
        find(&page(d).children, id).unwrap()
    }

    #[test]
    fn a_new_text_layer_is_auto_width_and_grows_with_its_text_and_size() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [10.0, 20.0, 0.0, 0.0]);
        assert_eq!(node_sizing(&d, &t), sizing(Size::Hug, Size::Hug).unwrap());
        assert!(
            matches!(&page(&d).children[0].kind, Kind::Text { content, .. } if content.text.is_empty())
        );
        assert_eq!(page(&d).children[0].name, "Text");
        assert_eq!(frames(&d, &t)[0][..3], [10.0, 20.0, 0.0]);
        assert!(close(frames(&d, &t)[0][3], LEADING));
        set_text(&mut d, &t, "Text");
        let w1 = frames(&d, &t)[0][2];
        set_text(&mut d, &t, "Text\nText text");
        let [_, _, w2, h2] = frames(&d, &t)[0];
        assert!(w2 > w1 * 1.5 && close(h2, 2.0 * LEADING), "{w2} {h2}");
        let (c, m) = collection(&mut d, "Type");
        let v = variable(&mut d, &c, "Size", Value::Number(24.0)).unwrap();
        bind(&mut d, &t, "size", Some(&v)).unwrap();
        let [_, _, w3, h3] = frames(&d, &t)[0];
        assert!(close(w3, 2.0 * w2) && close(h3, 2.0 * h2), "{w3} {h3}");
        set_value(&mut d, &v, &m, Value::Number(6.0)).unwrap();
        let [_, _, w4, h4] = frames(&d, &t)[0];
        assert!(close(w4, w2 / 2.0) && close(h4, h2 / 2.0), "{w4} {h4}");
    }

    #[test]
    fn resizing_the_width_makes_text_auto_height_and_the_height_makes_it_fixed() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Hi there Hi there");
        let auto_width = frames(&d, &t)[0];
        set_frame(&mut d, &t, [0.0, 0.0, auto_width[2] / 2.0, auto_width[3]]);
        assert_eq!(node_sizing(&d, &t), sizing(Size::Fixed, Size::Hug).unwrap());
        let [.., w, h] = frames(&d, &t)[0];
        assert!(
            close(w, auto_width[2] / 2.0) && close(h, 2.0 * LEADING),
            "{w} {h}"
        );
        set_frame(&mut d, &t, [0.0, 0.0, w, 100.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(frames(&d, &t)[0], [0.0, 0.0, w, 100.0]);
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Hug, Size::Fixed),
                ..Props::default()
            },
        );
        assert_eq!(node_sizing(&d, &t), sizing(Size::Hug, Size::Hug).unwrap());
        assert_eq!(frames(&d, &t)[0], auto_width);
        set_frame(&mut d, &t, [0.0, 0.0, 50.0, 50.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        set_frame(&mut d, &t, [0.0, 0.0, auto_width[2], 50.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
    }

    #[test]
    fn text_filling_an_auto_layout_frame_rewraps_and_the_frame_hugs_it() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 10.0]);
        let t = create(&mut d, &f, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set(
            &mut d,
            &f,
            Props {
                direction: Some(Direction::Vertical),
                padding_top: Some(5.0),
                padding_right: Some(5.0),
                padding_bottom: Some(5.0),
                padding_left: Some(5.0),
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Fill, Size::Hug),
                ..Props::default()
            },
        );
        set_text(&mut d, &t, "Hi there Hi there Hi there Hi there Hi there");
        let [outer, text] = frames(&d, &f)[..] else {
            panic!("one child");
        };
        assert_eq!(text[2], 90.0);
        assert!(text[3] > 2.5 * LEADING, "{text:?}");
        assert!(close(outer[3], text[3] + 10.0), "{outer:?} {text:?}");
        set_frame(&mut d, &f, [0.0, 0.0, 400.0, outer[3]]);
        let [outer, text] = frames(&d, &f)[..] else {
            panic!("one child");
        };
        assert_eq!(text[2], 390.0);
        assert!(close(text[3], LEADING), "{text:?}");
        assert!(close(outer[3], LEADING + 10.0), "{outer:?}");
    }

    fn edit(d: &mut Doc, id: &str, range: [usize; 2], text: &str) -> Res<Vec<String>> {
        d.apply(Command::EditText {
            id: id.into(),
            range,
            text: text.into(),
        })
    }

    #[test]
    fn edit_text_replaces_a_range_and_typed_text_takes_the_attributes_before_it() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        format(&mut d, &t, Some([0, 5]), sized(20.0)).unwrap();
        edit(&mut d, &t, [5, 5], "!").unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        edit(&mut d, &t, [0, 1], "J").unwrap();
        edit(&mut d, &t, [6, 12], "").unwrap();
        assert!(
            matches!(&page(&d).children[0].kind, Kind::Text { content, .. } if content.text == "Jello!")
        );
        assert!(edit(&mut d, &t, [3, 9], "x").is_err());
        assert!(edit(&mut d, &t, [4, 3], "x").is_err());
    }

    #[test]
    fn the_engine_finds_and_draws_the_caret_and_the_selection_of_a_text() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [10.0, 20.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Hi\nHi");
        let [x0, top, bottom] = d.caret(&t, 0).unwrap();
        assert!(close(x0, 10.0) && close(top, 20.0) && close(bottom, 20.0 + LEADING));
        let [x2, ..] = d.caret(&t, 2).unwrap();
        assert!(x2 > x0);
        assert_eq!(d.text_index(&t, x2 + 0.5, 25.0).unwrap(), 2);
        assert_eq!(d.text_index(&t, 11.0, 20.0 + LEADING + 3.0).unwrap(), 3);
        assert_eq!(d.text_line(&t, 4).unwrap(), [3, 5]);
        let fills = |ops: Vec<Op>| -> Vec<Vec<f32>> {
            ops.into_iter()
                .map(|op| match op {
                    Op::FillPath { path, .. } => path,
                    op => panic!("unexpected {op:?}"),
                })
                .collect()
        };
        let caret = fills(d.text_overlay(&t, 2, 2, 0.5, &p).unwrap());
        assert_eq!(caret.len(), 1);
        assert!((caret[0][1] - x2 as f32 + 0.25).abs() < 0.01, "{caret:?}");
        assert_eq!(fills(d.text_overlay(&t, 1, 4, 0.5, &p).unwrap()).len(), 2);
        assert!(d.caret(&t, 9).is_err());
    }

    #[test]
    fn a_hugging_frame_whose_last_child_is_deleted_keeps_its_size_as_fixed() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 1.0, 1.0]);
        let r = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 30.0, 20.0]);
        auto(
            &mut d,
            &f,
            Props {
                padding_left: Some(5.0),
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        assert_eq!(frames(&d, &f)[0], [0.0, 0.0, 35.0, 20.0]);
        d.apply(Command::Delete { ids: vec![r] }).unwrap();
        assert_eq!(frames(&d, &f), [[0.0, 0.0, 35.0, 20.0]]);
        assert_eq!(
            node_sizing(&d, &f),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        d.apply(Command::Undo).unwrap();
        assert_eq!(node_sizing(&d, &f), sizing(Size::Hug, Size::Hug).unwrap());
    }

    fn add_page(d: &mut Doc, after: Option<&str>) -> String {
        d.apply(Command::AddPage {
            after: after.map(String::from),
        })
        .unwrap()
        .remove(0)
    }

    fn page_ids(d: &Doc) -> Vec<String> {
        d.snapshot().pages.into_iter().map(|p| p.id).collect()
    }

    #[test]
    fn pages_are_added_after_a_page_with_its_size_and_moved_and_deleted_with_undo() {
        let (mut d, p1) = empty();
        d.apply(Command::SetPage {
            id: p1.clone(),
            width: Some(100.0),
            height: Some(200.0),
            bleed: Some(5.0),
        })
        .unwrap();
        let p3 = add_page(&mut d, None);
        let p2 = add_page(&mut d, Some(&p1));
        assert_eq!(page_ids(&d), [&p1, &p2, &p3].map(String::clone));
        let s = d.snapshot();
        assert_eq!(
            [s.pages[1].width, s.pages[1].height, s.pages[1].bleed],
            [100.0, 200.0, 5.0]
        );
        assert!(s.pages[1].children.is_empty());
        let r = create(&mut d, &p3, NewKind::Rect, [1.0, 2.0, 3.0, 4.0]);
        d.apply(Command::MovePage {
            id: p3.clone(),
            index: 0,
        })
        .unwrap();
        assert_eq!(page_ids(&d), [&p3, &p1, &p2].map(String::clone));
        d.apply(Command::MovePage {
            id: p3.clone(),
            index: 9,
        })
        .unwrap();
        assert_eq!(page_ids(&d), [&p1, &p2, &p3].map(String::clone));
        d.apply(Command::DeletePage { id: p3.clone() }).unwrap();
        assert_eq!(page_ids(&d), [&p1, &p2].map(String::clone));
        assert!(
            d.apply(Command::Delete {
                ids: vec![r.clone()]
            })
            .is_err()
        );
        d.apply(Command::Undo).unwrap();
        assert_eq!(page_ids(&d), [&p1, &p2, &p3].map(String::clone));
        assert_eq!(
            ids(&d.snapshot().pages[2].children),
            std::slice::from_ref(&r)
        );
        d.apply(Command::DeletePage { id: p1.clone() }).unwrap();
        d.apply(Command::DeletePage { id: p2.clone() }).unwrap();
        assert!(d.apply(Command::DeletePage { id: p3.clone() }).is_err());
        assert!(d.apply(Command::MovePage { id: r, index: 0 }).is_err());
        assert!(
            d.apply(Command::SetPage {
                id: p3,
                width: Some(-1.0),
                height: None,
                bleed: None,
            })
            .is_err()
        );
    }

    fn facing(d: &mut Doc, on: bool) {
        d.apply(Command::SetDocument {
            raster_ppi: None,
            color_mode: None,
            facing_pages: Some(on),
        })
        .unwrap();
    }

    fn sides(d: &Doc) -> Vec<Option<Side>> {
        d.snapshot().pages.iter().map(|p| p.side).collect()
    }

    #[test]
    fn facing_pages_pair_the_pages_into_spreads_after_the_first_right_page() {
        let (mut d, p1) = empty();
        assert!(d.snapshot().facing_pages);
        let p2 = add_page(&mut d, None);
        assert_eq!(d.snapshot().spreads, [vec![p1.clone()], vec![p2.clone()]]);
        assert_eq!(sides(&d), [Some(Side::Right), Some(Side::Left)]);
        let p3 = add_page(&mut d, None);
        let p4 = add_page(&mut d, None);
        let s = d.snapshot();
        assert_eq!(
            s.spreads,
            [
                vec![p1.clone()],
                vec![p2.clone(), p3.clone()],
                vec![p4.clone()]
            ]
        );
        use Side::{Left, Right};
        assert_eq!(sides(&d), [Right, Left, Right, Left].map(Some));
        facing(&mut d, false);
        let s = d.snapshot();
        assert!(!s.facing_pages);
        assert_eq!(s.spreads, [p1, p2, p3, p4].map(|p| vec![p]));
        assert_eq!(sides(&d), [None; 4]);
        d.apply(Command::Undo).unwrap();
        assert!(d.snapshot().facing_pages);
    }

    fn set_width(d: &mut Doc, id: &str, width: f64) {
        d.apply(Command::SetPage {
            id: id.into(),
            width: Some(width),
            height: None,
            bleed: None,
        })
        .unwrap();
    }

    /// The least and greatest x of each path filled in `ops`.
    fn filled_x(ops: &[Op]) -> Vec<[f32; 2]> {
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

    #[test]
    fn the_pages_of_a_spread_sit_left_and_right_of_the_spine() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let p3 = add_page(&mut d, None);
        set_width(&mut d, &p2, 100.0);
        set_width(&mut d, &p3, 120.0);
        let xs = |d: &Doc| d.snapshot().pages.iter().map(|p| p.x).collect::<Vec<_>>();
        assert_eq!(xs(&d), [0.0, -100.0, 0.0]);
        facing(&mut d, false);
        assert_eq!(xs(&d), [0.0; 3]);
        let _ = p1;
    }

    #[test]
    fn a_layer_across_the_spine_prints_on_both_pages_of_its_spread() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let p3 = add_page(&mut d, None);
        set_width(&mut d, &p2, 100.0);
        create(&mut d, &p2, NewKind::Rect, [90.0, 0.0, 20.0, 10.0]);
        create(&mut d, &p3, NewKind::Rect, [5.0, 0.0, 10.0, 10.0]);
        create(&mut d, &p3, NewKind::Rect, [50.0, 0.0, 10.0, 10.0]);
        create(&mut d, &p1, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(filled_x(&d.render(&p3)), [[5.0, 15.0], [50.0, 60.0]]);
        let snap = d.snapshot();
        let print = |i: usize| filled_x(&d.print(&snap, &snap.pages[i]));
        assert_eq!(print(1), [[90.0, 110.0], [105.0, 115.0]]);
        assert_eq!(print(2), [[-10.0, 10.0], [5.0, 15.0], [50.0, 60.0]]);
        assert_eq!(print(0), [[0.0, 10.0]]);
    }

    #[test]
    fn a_layer_moved_onto_the_other_page_of_its_spread_keeps_its_place_on_the_spread() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let p3 = add_page(&mut d, None);
        set_width(&mut d, &p2, 100.0);
        let g = create(&mut d, &p2, NewKind::Frame, [95.0, 5.0, 20.0, 10.0]);
        let r = create(&mut d, &g, NewKind::Rect, [96.0, 6.0, 2.0, 2.0]);
        let mv = |d: &mut Doc, to: &str| {
            d.apply(Command::Move {
                ids: vec![g.clone()],
                parent: to.into(),
                index: 0,
            })
            .unwrap()
        };
        mv(&mut d, &p3);
        let s = d.snapshot();
        assert_eq!(frame(&s.pages[2].children[0]), [-5.0, 5.0, 20.0, 10.0]);
        assert_eq!(
            frame(&children(&s.pages[2].children[0])[0]),
            [-4.0, 6.0, 2.0, 2.0]
        );
        mv(&mut d, &p1);
        assert_eq!(
            frame(&d.snapshot().pages[0].children[0]),
            [-5.0, 5.0, 20.0, 10.0]
        );
        let _ = r;
    }

    #[test]
    fn a_duplicated_page_follows_its_original_with_copies_of_its_layers() {
        let (mut d, p1) = empty();
        let r = create(&mut d, &p1, NewKind::Rect, [1.0, 2.0, 3.0, 4.0]);
        let p2 = d
            .apply(Command::DuplicatePage { id: p1.clone() })
            .unwrap()
            .remove(0);
        let s = d.snapshot();
        assert_eq!(page_ids(&d), [p1, p2]);
        assert_eq!(frame(&s.pages[1].children[0]), [1.0, 2.0, 3.0, 4.0]);
        assert_ne!(s.pages[1].children[0].id, r);
    }

    #[test]
    fn each_page_renders_and_hits_its_own_layers() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let a = create(&mut d, &p1, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &p2, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(d.hit(&p1, 5.0, 5.0, 0.0), [a]);
        assert_eq!(d.hit(&p2, 5.0, 5.0, 0.0), [b]);
        let items = |p: &str| d.render(p).iter().filter(|o| **o == Op::EndItem).count();
        assert_eq!((items(&p1), items(&p2)), (1, 1));
        assert_ne!(d.render(&p1), d.render(&p2));
    }

    #[test]
    fn paste_without_a_target_goes_into_the_source_parent_only_on_the_same_page() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let f = create(&mut d, &p1, NewKind::Frame, [0.0, 0.0, 9.0, 9.0]);
        let r = create(&mut d, &f, NewKind::Rect, [1.0, 1.0, 2.0, 2.0]);
        d.apply(Command::Copy { ids: vec![r] }).unwrap();
        let paste = |d: &mut Doc, page: &str| {
            d.apply(Command::Paste {
                above: vec![],
                page: Some(page.into()),
            })
            .unwrap()
            .remove(0)
        };
        let on2 = paste(&mut d, &p2);
        assert_eq!(ids(&d.snapshot().pages[1].children), [on2]);
        let on1 = paste(&mut d, &p1);
        assert_eq!(children(&d.snapshot().pages[0].children[0])[1].id, on1);
    }

    fn add_master(d: &mut Doc) -> String {
        d.apply(Command::AddMaster { like: None })
            .unwrap()
            .remove(0)
    }

    fn use_master(d: &mut Doc, page: &str, master: Option<&str>) -> Res<Vec<String>> {
        d.apply(Command::UseMaster {
            page: page.into(),
            master: master.map(String::from),
        })
    }

    fn items(ops: &[Op]) -> Vec<u32> {
        ops.iter()
            .filter_map(|o| match o {
                Op::BeginItem { item } => Some(*item),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_master_draws_under_the_layers_of_the_pages_that_use_it_and_is_not_hit_there() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let m = add_master(&mut d);
        let s = d.snapshot();
        assert_eq!(s.masters.len(), 1);
        assert_eq!(s.masters[0].name, "A-Master");
        assert_eq!(s.masters[0].width, s.pages[0].width);
        let under = create(&mut d, &m, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        let over = create(&mut d, &p1, NewKind::Ellipse, [5.0, 5.0, 5.0, 5.0]);
        use_master(&mut d, &p1, Some(&m)).unwrap();
        assert_eq!(d.snapshot().pages[0].master.as_deref(), Some(m.as_str()));
        let on = page_ops(&d);
        assert_eq!(items(&on).len(), 2);
        assert_eq!(items(&d.render(&m)), items(&on)[..1]);
        assert!(items(&d.render(&p2)).is_empty());
        assert!(hits(&d, 15.0, 15.0, 0.0).is_empty());
        assert_eq!(hits(&d, 7.0, 7.0, 0.0), [over]);
        assert_eq!(d.hit(&m, 15.0, 15.0, 0.0), std::slice::from_ref(&under));
        assert_eq!(d.master_hit(&p1, 15.0, 15.0, 0.0), Some(under.clone()));
        assert_eq!(d.master_hit(&p2, 15.0, 15.0, 0.0), None);
        assert!(use_master(&mut d, &p1, Some(&p2)).is_err());
        assert_eq!(d.apply(Command::AddMaster { like: None }).unwrap().len(), 1);
        assert_eq!(d.snapshot().masters[1].name, "B-Master");
        d.apply(Command::SetMaster {
            id: m.clone(),
            name: "Body".into(),
        })
        .unwrap();
        assert_eq!(d.snapshot().masters[0].name, "Body");
        assert!(
            d.apply(Command::SetMaster {
                id: m.clone(),
                name: "B-Master".into(),
            })
            .is_err()
        );
    }

    #[test]
    fn new_pages_take_the_master_of_the_page_before_and_a_deleted_master_leaves_its_pages() {
        let (mut d, p1) = empty();
        let m = add_master(&mut d);
        use_master(&mut d, &p1, Some(&m)).unwrap();
        add_page(&mut d, Some(&p1));
        let dup = d
            .apply(Command::DuplicatePage { id: p1.clone() })
            .unwrap()
            .remove(0);
        let masters = |d: &Doc| -> Vec<Option<String>> {
            d.snapshot().pages.into_iter().map(|p| p.master).collect()
        };
        assert_eq!(
            masters(&d),
            [Some(m.clone()), Some(m.clone()), Some(m.clone())]
        );
        assert_eq!(page_ids(&d)[1], dup);
        d.apply(Command::DeleteMaster { id: m.clone() }).unwrap();
        assert!(d.snapshot().masters.is_empty());
        assert_eq!(masters(&d), [None, None, None]);
        d.apply(Command::Undo).unwrap();
        assert_eq!(masters(&d)[0], Some(m));
    }

    #[test]
    fn an_overridden_master_layer_becomes_a_page_layer_until_reset_to_the_master() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let m = add_master(&mut d);
        let r = create(&mut d, &m, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        let top = create(&mut d, &p1, NewKind::Ellipse, [50.0, 50.0, 5.0, 5.0]);
        for p in [&p1, &p2] {
            use_master(&mut d, p, Some(&m)).unwrap();
        }
        let copy = d
            .apply(Command::Override {
                page: p1.clone(),
                id: r.clone(),
            })
            .unwrap()
            .remove(0);
        let pg = page(&d);
        assert_eq!(ids(&pg.children), [copy.clone(), top.clone()]);
        assert_eq!(pg.children[0].override_of.as_deref(), Some(r.as_str()));
        assert_eq!(frame(&pg.children[0]), [0.0, 0.0, 20.0, 20.0]);
        assert_eq!(pg.detached, std::slice::from_ref(&r));
        assert_eq!(items(&page_ops(&d)).len(), 2);
        assert_eq!(hits(&d, 15.0, 15.0, 0.0), std::slice::from_ref(&copy));
        assert_eq!(d.master_hit(&p1, 15.0, 15.0, 0.0), None);
        assert_eq!(items(&d.render(&p2)).len(), 1);
        assert!(
            d.apply(Command::Override {
                page: p1.clone(),
                id: top.clone(),
            })
            .is_err()
        );
        d.apply(Command::Delete {
            ids: vec![copy.clone()],
        })
        .unwrap();
        assert_eq!(items(&page_ops(&d)).len(), 1);
        d.apply(Command::ResetToMaster {
            ids: vec![p1.clone()],
        })
        .unwrap();
        assert!(page(&d).detached.is_empty());
        assert_eq!(items(&page_ops(&d)).len(), 2);
        let copy = d
            .apply(Command::Override {
                page: p1.clone(),
                id: r.clone(),
            })
            .unwrap()
            .remove(0);
        d.apply(Command::ResetToMaster { ids: vec![copy] }).unwrap();
        let pg = page(&d);
        assert_eq!((ids(&pg.children), pg.detached.len()), (vec![top], 0));
        assert_eq!(d.master_hit(&p1, 15.0, 15.0, 0.0), Some(r));
    }

    /// The x range of each clip pushed in `ops`.
    fn clips_x(ops: &[Op]) -> Vec<[f32; 2]> {
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
    fn master_spread() -> (Doc, [String; 5]) {
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

    #[test]
    fn a_page_shows_the_side_of_its_master_spread_that_it_is_on() {
        let (d, [p1, p2, _, left, right]) = master_spread();
        assert_eq!(d.master_hit(&p1, 15.0, 5.0, 0.0), Some(right.clone()));
        assert_eq!(d.master_hit(&p2, 15.0, 5.0, 0.0), Some(left));
        assert_eq!(d.master_hit(&p2, 115.0, 5.0, 0.0), None);
        let [w, b] = [page(&d).width, page(&d).bleed].map(|v| v as f32);
        assert_eq!(filled_x(&d.render(&p1)), [[10.0, 20.0]]);
        assert_eq!(clips_x(&d.render(&p1)), [[0.0, w + b]]);
        assert_eq!(filled_x(&d.render(&p2)), [[10.0, 20.0]]);
        assert_eq!(clips_x(&d.render(&p2)), [[-b, w]]);
        let _ = right;
    }

    #[test]
    fn a_master_layer_overridden_on_a_left_page_stays_where_the_page_shows_it() {
        let (mut d, [_, p2, _, left, _]) = master_spread();
        d.apply(Command::Override {
            page: p2.clone(),
            id: left.clone(),
        })
        .unwrap();
        let s = d.snapshot();
        assert_eq!(frame(&s.pages[1].children[0]), [10.0, 0.0, 10.0, 10.0]);
        assert_eq!(d.master_hit(&p2, 15.0, 5.0, 0.0), None);
    }

    #[test]
    fn with_facing_pages_a_master_gets_its_layers_on_both_pages_and_without_them_one_page() {
        let (mut d, p1) = empty();
        facing(&mut d, false);
        let p2 = add_page(&mut d, None);
        let m = add_master(&mut d);
        set_width(&mut d, &m, 100.0);
        let r = create(&mut d, &m, NewKind::Rect, [10.0, 0.0, 10.0, 10.0]);
        for p in [&p1, &p2] {
            use_master(&mut d, p, Some(&m)).unwrap();
        }
        let copies: Vec<String> = [&p1, &p2]
            .map(|p| {
                d.apply(Command::Override {
                    page: p.clone(),
                    id: r.clone(),
                })
                .unwrap()
                .remove(0)
            })
            .into();
        let master = |d: &Doc| {
            d.snapshot().masters[0]
                .children
                .iter()
                .map(frame)
                .collect::<Vec<_>>()
        };
        let detached = |d: &Doc| {
            d.snapshot()
                .pages
                .iter()
                .map(|p| p.detached.clone())
                .collect::<Vec<_>>()
        };
        let of = |d: &Doc| {
            d.snapshot()
                .pages
                .iter()
                .map(|p| p.children[0].override_of.clone().unwrap())
                .collect::<Vec<_>>()
        };
        facing(&mut d, true);
        assert_eq!(
            master(&d),
            [[10.0, 0.0, 10.0, 10.0], [-90.0, 0.0, 10.0, 10.0]]
        );
        let l = d.snapshot().masters[0].children[1].id.clone();
        assert_eq!(detached(&d), [vec![r.clone()], vec![l.clone()]]);
        assert_eq!(of(&d), [r.clone(), l]);
        assert_eq!(ids(&d.snapshot().pages[1].children), [copies[1].clone()]);
        facing(&mut d, false);
        assert_eq!(master(&d), [[10.0, 0.0, 10.0, 10.0]]);
        assert_eq!(detached(&d), [vec![r.clone()], vec![r.clone()]]);
        assert_eq!(of(&d), [r.clone(), r]);
    }

    /// The text of the glyph runs of `ops`.
    fn run_texts(ops: &[Op]) -> Vec<String> {
        ops.iter()
            .filter_map(|o| match o {
                Op::GlyphRun { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_page_number_shows_the_number_of_its_page_and_on_a_master_its_prefix() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let p3 = add_page(&mut d, None);
        let m = add_master(&mut d);
        let marked = |d: &mut Doc, parent: &str, text: &str| {
            let t = create(d, parent, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
            d.apply(Command::SetText {
                id: t.clone(),
                text: text.replace('#', &text::PAGE_NUMBER.to_string()),
            })
            .unwrap();
            t
        };
        marked(&mut d, &p1, "#");
        marked(&mut d, &m, "Page #");
        for p in [&p2, &p3] {
            use_master(&mut d, p, Some(&m)).unwrap();
        }
        assert_eq!(run_texts(&d.render(&p1)), ["1"]);
        assert_eq!(run_texts(&d.render(&m)), ["PageA"]);
        assert_eq!(run_texts(&d.render(&p2)), ["Page2"]);
        assert_eq!(run_texts(&d.render(&p3)), ["Page3"]);
        assert_eq!(page(&d).children[0].name, "#");
        assert!(page(&d).children[0].w > 0.0);
        d.apply(Command::MovePage {
            id: p1.clone(),
            index: 2,
        })
        .unwrap();
        assert_eq!(run_texts(&d.render(&p1)), ["3"]);
    }

    #[test]
    fn a_master_page_number_wider_than_the_prefix_refits_its_text_on_the_page() {
        let (mut d, _) = empty();
        let mut last = String::new();
        for _ in 0..11 {
            last = add_page(&mut d, None);
        }
        let m = add_master(&mut d);
        let t = create(&mut d, &m, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        d.apply(Command::SetText {
            id: t,
            text: format!("Page {}", text::PAGE_NUMBER),
        })
        .unwrap();
        use_master(&mut d, &last, Some(&m)).unwrap();
        assert_eq!(run_texts(&d.render(&last)), ["Page12"]);
    }

    /// A fixed text frame on `page` at `frame`.
    fn fixed_text(d: &mut Doc, page: &str, frame: [f64; 4]) -> String {
        let t = create(d, page, NewKind::Text, [frame[0], frame[1], 0.0, 0.0]);
        set_frame(d, &t, frame);
        t
    }

    fn thread(d: &mut Doc, from: &str, to: &str) -> Res<Vec<String>> {
        d.apply(Command::Thread {
            from: from.into(),
            to: to.into(),
        })
    }

    /// The story, start, end, previous and next frame, and overset of a text layer.
    fn flow(d: &Doc, id: &str) -> (String, usize, usize, Option<String>, Option<String>, bool) {
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
    fn two_pages() -> (Doc, String, String, String, String) {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 2.0 * LEADING + 1.0]);
        let b = fixed_text(&mut d, &p2, [10.0, 10.0, 100.0, 300.0]);
        set_text(&mut d, &a, "Hi\nHi\nHi\nHi");
        thread(&mut d, &a, &b).unwrap();
        (d, p1, p2, a, b)
    }

    #[test]
    fn a_frame_with_more_text_than_fits_is_overset() {
        let (mut d, p1) = empty();
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 2.0 * LEADING + 1.0]);
        set_text(&mut d, &a, "Hi\nHi\nHi\nHi");
        assert!(flow(&d, &a).5);
    }

    #[test]
    fn threaded_frames_share_one_story_and_each_sets_its_part() {
        let (d, _, _, a, b) = two_pages();
        let story = "Hi\nHi\nHi\nHi".to_string();
        assert_eq!(
            flow(&d, &a),
            (story.clone(), 0, 6, None, Some(b.clone()), false)
        );
        assert_eq!(flow(&d, &b), (story, 6, 11, Some(a), None, false));
    }

    #[test]
    fn each_page_draws_the_lines_of_its_frames_of_a_thread() {
        let (d, p1, p2, _, _) = two_pages();
        let runs = |p: &str| {
            d.render(p)
                .into_iter()
                .filter_map(|o| match o {
                    Op::GlyphRun { positions, .. } => Some(positions[1]),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(runs(&p1).len(), 2);
        let on2 = runs(&p2);
        assert_eq!(on2.len(), 2);
        assert!(on2[0] > 10.0 && on2[0] < 10.0 + LEADING as f32, "{on2:?}");
    }

    #[test]
    fn text_typed_into_a_later_frame_goes_into_the_story_and_the_caret_finds_its_frame() {
        let (mut d, _, _, a, b) = two_pages();
        edit(&mut d, &b, [7, 7], "X").unwrap();
        assert_eq!(flow(&d, &a).0, "Hi\nHi\nHXi\nHi");
        assert_eq!(d.text_frame(&a, 8).unwrap(), b);
        assert_eq!(d.text_frame(&b, 2).unwrap(), a);
        assert_eq!(d.text_frame(&a, 6).unwrap(), b);
        let [_, top, _] = d.caret(&a, 8).unwrap();
        assert!(close(top, 10.0), "{top}");
        assert_eq!(d.text_index(&b, 11.0, 11.0).unwrap(), 6);
        assert_eq!(d.text_line(&a, 7).unwrap(), [6, 9]);
    }

    #[test]
    fn a_selection_across_threaded_frames_shows_on_each_page() {
        let (d, p1, p2, a, _) = two_pages();
        let overlay = |page: &str| d.text_overlay(&a, 1, 8, 0.5, page).unwrap().len();
        assert_eq!((overlay(&p1), overlay(&p2)), (2, 1));
    }

    #[test]
    fn the_last_frame_of_a_thread_is_overset_when_the_story_does_not_fit() {
        let (mut d, _, _, a, b) = two_pages();
        set_frame(&mut d, &b, [10.0, 10.0, 100.0, LEADING + 1.0]);
        assert!(flow(&d, &b).5);
        assert!(!flow(&d, &a).5);
    }

    #[test]
    fn unthreading_takes_the_story_back_into_the_first_frame_and_undo_threads_again() {
        let (mut d, _, _, a, b) = two_pages();
        d.apply(Command::Unthread { id: a.clone() }).unwrap();
        assert_eq!(flow(&d, &a).4, None);
        assert!(flow(&d, &a).5);
        assert_eq!(flow(&d, &b).0, "");
        assert_eq!(flow(&d, &b).3, None);
        d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&d, &b).3, Some(a));
    }

    #[test]
    fn only_fixed_frames_thread_and_the_last_may_be_auto_height() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set_text(&mut d, &a, "Hi Hi Hi Hi");
        set_frame(&mut d, &a, [0.0, 0.0, 30.0, LEADING]);
        set(
            &mut d,
            &a,
            Props {
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        let b = create(&mut d, &p, NewKind::Text, [0.0, 100.0, 0.0, 0.0]);
        set_text(&mut d, &b, "Yo");
        let c = fixed_text(&mut d, &p, [0.0, 200.0, 30.0, 30.0]);
        set(
            &mut d,
            &c,
            Props {
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        thread(&mut d, &a, &b).unwrap();
        assert_eq!(
            node_sizing(&d, &a),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(
            node_sizing(&d, &b),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(flow(&d, &a).0, "Hi Hi Hi Hi\nYo");
        assert!(close(frames(&d, &a)[0][3], 2.0 * LEADING));
        thread(&mut d, &b, &c).unwrap();
        assert_eq!(flow(&d, &c).1, 14);
        assert!(close(frames(&d, &c)[0][3], LEADING));
        assert_eq!(node_sizing(&d, &c), sizing(Size::Fixed, Size::Hug).unwrap());
        set_frame(&mut d, &b, [0.0, 100.0, 30.0, 1.0]);
        assert_eq!(flow(&d, &c).1, 12);
        assert!(close(frames(&d, &c)[0][3], LEADING));
        for (id, s) in [
            (&a, sizing(Size::Fixed, Size::Hug)),
            (&c, sizing(Size::Hug, Size::Hug)),
        ] {
            let bad = Command::Set {
                id: id.clone(),
                props: Props {
                    sizing: s,
                    ..Props::default()
                },
            };
            assert!(d.apply(bad).is_err());
        }
        assert!(thread(&mut d, &a, &a).is_err());
        assert!(thread(&mut d, &c, &a).is_err());
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        assert!(thread(&mut d, &c, &r).is_err());
    }

    #[test]
    fn a_frame_threaded_after_one_with_a_next_goes_in_between() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 50.0, 20.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 50.0, 20.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 50.0, 20.0]);
        thread(&mut d, &a, &c).unwrap();
        thread(&mut d, &a, &b).unwrap();
        assert_eq!(flow(&d, &a).4, Some(b.clone()));
        assert_eq!(flow(&d, &b).4, Some(c.clone()));
        assert_eq!(flow(&d, &c).3, Some(b));
    }

    #[test]
    fn deleting_a_threaded_frame_keeps_its_story_in_the_rest_of_the_thread() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, LEADING + 1.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo\nHu");
        thread(&mut d, &a, &b).unwrap();
        thread(&mut d, &b, &c).unwrap();
        format(&mut d, &a, None, sized(10.0)).unwrap();
        d.apply(Command::Delete {
            ids: vec![b.clone()],
        })
        .unwrap();
        assert_eq!(flow(&d, &a).4, Some(c.clone()));
        assert_eq!(flow(&d, &c).1, 3);
        d.apply(Command::Delete {
            ids: vec![a.clone()],
        })
        .unwrap();
        let (text, start, _, prev, ..) = flow(&d, &c);
        assert_eq!((text.as_str(), start, prev), ("Hi\nHo\nHu", 0, None));
        assert!(spans(&d).iter().all(|s| s.attrs.size == 10.0));
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&d, &b).3, Some(a));
    }

    #[test]
    fn a_copy_of_a_threaded_frame_holds_its_whole_story_and_no_thread() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        let copy = d
            .apply(Command::Duplicate {
                ids: vec![b.clone()],
            })
            .unwrap()
            .remove(0);
        let (text, start, _, prev, next, _) = flow(&d, &copy);
        assert_eq!(
            (text.as_str(), start, prev, next),
            ("Hi\nHo", 0, None, None)
        );
        assert_eq!(flow(&d, &a).4, Some(b));
    }

    #[test]
    fn boxes_keep_a_least_size_and_paths_may_be_flat() {
        let (mut d, p) = empty();
        let least = 0.01 * MM;
        let size = |d: &Doc, id: &str| {
            let [.., w, h] = d.bounds(d.node(id).unwrap());
            [w, h]
        };
        for kind in [NewKind::Rect, NewKind::Ellipse, NewKind::Frame] {
            let r = create(&mut d, &p, kind, [0.0, 0.0, 40.0, 0.0]);
            assert_eq!(size(&d, &r), [40.0, least]);
            set_frame(&mut d, &r, [0.0, 0.0, 0.0, 20.0]);
            assert_eq!(size(&d, &r), [least, 20.0]);
        }
        let t = fixed_text(&mut d, &p, [0.0, 0.0, 40.0, 0.0]);
        assert_eq!(size(&d, &t), [40.0, least]);
        let hug = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(size(&d, &hug)[0], 0.0);
        for kind in [NewKind::Line, NewKind::Arrow] {
            let l = create(&mut d, &p, kind, [0.0, 0.0, 40.0, 0.0]);
            assert_eq!(size(&d, &l), [40.0, 0.0]);
        }
    }

    #[test]
    fn frames_thread_within_the_pages_or_within_one_master() {
        let (mut d, p) = empty();
        let (m1, m2) = (add_master(&mut d), add_master(&mut d));
        let on_page = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, 100.0]);
        let [a, b, c] = [&m1, &m1, &m2].map(|m| fixed_text(&mut d, m, [0.0, 0.0, 100.0, 100.0]));
        assert!(thread(&mut d, &a, &on_page).is_err());
        assert!(thread(&mut d, &on_page, &a).is_err());
        assert!(thread(&mut d, &a, &c).is_err());
        thread(&mut d, &a, &b).unwrap();
    }

    #[test]
    fn threaded_master_frames_thread_on_the_left_page_when_facing_pages_go_on() {
        let (mut d, _) = empty();
        facing(&mut d, false);
        let m = add_master(&mut d);
        let a = fixed_text(&mut d, &m, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &m, [0.0, 50.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        facing(&mut d, true);
        let copies: Vec<String> = d.snapshot().masters[0]
            .children
            .iter()
            .filter(|n| n.x < 0.0)
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(flow(&d, &copies[0]).4, Some(copies[1].clone()));
        assert_eq!(flow(&d, &copies[1]).1, 3);
    }

    #[test]
    fn a_thread_from_the_left_master_page_keeps_its_story_when_facing_pages_go_off() {
        let (mut d, _) = empty();
        let m2 = add_master(&mut d);
        let left = fixed_text(&mut d, &m2, [-100.0, 0.0, 50.0, LEADING + 1.0]);
        let right = fixed_text(&mut d, &m2, [10.0, 0.0, 100.0, 100.0]);
        set_text(&mut d, &left, "Hi\nHo");
        thread(&mut d, &left, &right).unwrap();
        facing(&mut d, false);
        assert_eq!(flow(&d, &right).0, "Hi\nHo");
    }

    #[test]
    fn grouping_layers_across_the_spine_keeps_them_in_place() {
        let (mut d, _) = empty();
        let left = add_page(&mut d, None);
        let right = add_page(&mut d, None);
        let a = create(&mut d, &left, NewKind::Rect, [10.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &right, NewKind::Rect, [10.0, 0.0, 10.0, 10.0]);
        let shift = d.snapshot().pages[1].x;
        for frame in [false, true] {
            let g = d
                .apply(Command::Group {
                    ids: vec![a.clone(), b.clone()],
                    frame,
                })
                .unwrap()
                .remove(0);
            let [x, ..] = d.bounds(d.node(&a).unwrap());
            assert_eq!(x, 10.0 + shift, "{frame}");
            assert_eq!(d.bounds(d.node(&g).unwrap())[0], 10.0 + shift);
            d.apply(Command::Undo).unwrap();
        }
    }

    #[test]
    fn copies_of_threaded_frames_thread_among_themselves() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, LEADING + 1.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo\nHu");
        thread(&mut d, &a, &b).unwrap();
        thread(&mut d, &b, &c).unwrap();

        let page = d
            .apply(Command::DuplicatePage { id: p.clone() })
            .unwrap()
            .remove(0);
        let [a2, b2, c2] = d.snapshot().pages[1]
            .children
            .iter()
            .map(|n| n.id.clone())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        assert_eq!(flow(&d, &a2).4, Some(b2.clone()));
        assert_eq!(flow(&d, &b2).4, Some(c2.clone()));
        assert_eq!(flow(&d, &c2).1, 6);
        assert_eq!(flow(&d, &a).4, Some(b.clone()));
        d.apply(Command::DeletePage { id: page }).unwrap();

        let copies = d
            .apply(Command::Duplicate {
                ids: vec![b.clone(), c.clone()],
            })
            .unwrap();
        let (text, start, _, prev, next, _) = flow(&d, &copies[0]);
        assert_eq!(
            (text.as_str(), start, prev, next),
            ("Hi\nHo\nHu", 0, None, Some(copies[1].clone()))
        );
        assert_eq!(flow(&d, &copies[1]).1, 3);
        assert_eq!(flow(&d, &c).3, Some(b.clone()));

        d.apply(Command::Copy {
            ids: vec![a.clone(), b.clone()],
        })
        .unwrap();
        let pasted = d
            .apply(Command::Paste {
                above: vec![],
                page: None,
            })
            .unwrap();
        assert_eq!(flow(&d, &pasted[0]).4, Some(pasted[1].clone()));
        assert_eq!(flow(&d, &pasted[1]).1, 3);
    }

    /// The M1 booklet: 8 facing A5 pages on a master spread with a page number at the
    /// outer foot of each side, one story of 41 k characters threaded through a frame
    /// on every page, coloured by a variable that each page shows in a mode of its own.
    struct Booklet {
        d: Doc,
        pages: Vec<String>,
        master: String,
        frames: Vec<String>,
        story: String,
        /// The story's colour on each page.
        inks: Vec<[f32; 4]>,
    }

    fn booklet() -> Booklet {
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

    fn scope_none() -> Scope<'static> {
        static PALETTE: std::sync::LazyLock<Palette> = std::sync::LazyLock::new(Palette::default);
        static MODES: std::sync::LazyLock<Modes> = std::sync::LazyLock::new(Modes::new);
        Scope {
            palette: &PALETTE,
            modes: &MODES,
        }
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
        std::fs::write(&path, b.d.pdf()).unwrap();
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
}
