use super::*;

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
    /// The highest total of inks in % that preflight allows.
    pub ink_limit: f64,
    /// How the PDF exports; an RGB document exports PDF/X-4 in place of PDF/X-1a.
    pub preset: Preset,
    pub crop_marks: bool,
    pub include_bleed: bool,
    #[serde(flatten)]
    pub palette: Palette,
    /// The fonts text can be set in, the bundled one first.
    pub fonts: Vec<Typeface>,
    pub missing_fonts: Vec<MissingFont>,
    /// The colours of each image by its hash.
    pub images: BTreeMap<String, image::Space>,
    pub preflight: Vec<Issue>,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// A font that text is set in but that is not there, and the stories by their first
/// frame that use it.
#[derive(Debug, PartialEq, Serialize)]
pub struct MissingFont {
    pub font: Typeface,
    pub stories: Vec<String>,
}

/// Something about the layer `layer` on the page or master `page` that may print wrong.
#[derive(Debug, PartialEq, Serialize)]
pub struct Issue {
    pub page: String,
    pub layer: String,
    pub name: String,
    #[serde(flatten)]
    pub problem: Problem,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "problem", rename_all = "camelCase")]
pub enum Problem {
    Overset,
    MissingFont {
        font: String,
    },
    /// Reaches the trim edge but not the edge of the bleed.
    ShortOfBleed,
    /// Is coloured in RGB in a CMYK document.
    Rgb,
    /// Is filled with an image at fewer pixels per inch than print needs.
    LowPpi {
        ppi: f64,
    },
    /// Has an RGB colour that prints visibly different in a CMYK document.
    Gamut,
    /// Has a colour whose inks add up to `ink` %, above the document's limit.
    Ink {
        ink: f64,
    },
    /// Ends `distance` pt inside the trim, closer than `SAFE`.
    NearTrim {
        distance: f64,
    },
}

/// How far inside the trim a layer should stay, 3 mm.
const SAFE: f64 = 3.0 * 72.0 / 25.4;

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
    pub grids: Vec<Grid>,
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
    /// The upright box on the page that the layer and what it does not clip cover,
    /// turned by its rotation and its ancestors'.
    pub bounds: [f64; 4],
    /// Modes chosen on this layer.
    pub modes: Modes,
    /// Modes this layer's variables resolve in, its own and inherited.
    pub active_modes: Modes,
    /// Number variables by property.
    pub bindings: BTreeMap<String, String>,
    /// The master layer this page layer overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_of: Option<String>,
    /// Effective pixels per inch of the coarsest visible image fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ppi: Option<f64>,
    /// Not drawn, exported, hit or preflighted, with its children.
    pub hidden: bool,
    /// Not hit on the canvas, with its children.
    pub locked: bool,
    /// Degrees counterclockwise around the centre, with its children.
    pub rotation: f64,
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

pub(super) fn master_dx(m: &Page, p: &Page) -> f64 {
    match p.side {
        Some(Side::Left) => m.width,
        _ => 0.0,
    }
}

/// The box of a layer with its strokes and effects: what it can draw into.
pub(super) fn reach(n: &Node) -> [f64; 4] {
    let s = &n.style;
    let mut m = 3.0 * f64::from(s.stroke_weight);
    for e in &s.effects {
        m = m.max(f64::from(e.x.abs().max(e.y.abs()) + 3.0 * e.radius));
    }
    [n.x - m, n.y - m, n.w + 2.0 * m, n.h + 2.0 * m]
}

/// The issues of every layer, page by page and then master by master.
pub(super) fn preflight(snap: &Snapshot) -> Vec<Issue> {
    let mut out = Vec::new();
    let page = |id: &String| snap.pages.iter().find(|q| q.id == *id).unwrap();
    for p in &snap.pages {
        let spread = snap.spreads.iter().find(|s| s.contains(&p.id)).unwrap();
        let (l, r) = (page(&spread[0]), page(spread.last().unwrap()));
        let x0 = match l.side {
            Some(Side::Right) => f64::NEG_INFINITY,
            _ => l.x - p.x,
        };
        let x1 = match r.side {
            Some(Side::Left) => f64::INFINITY,
            _ => r.x + r.width - p.x,
        };
        for n in &p.children {
            visit(n, p, Some([x0, x1]), snap, &mut out);
        }
    }
    for m in &snap.masters {
        let x0 = if snap.facing_pages { -m.width } else { 0.0 };
        for n in &m.children {
            visit(n, m, Some([x0, m.width]), snap, &mut out);
        }
    }
    out
}

/// Adds the issues of the layer `n` on `p` and its children to `out`; a top layer
/// is checked against the bleed of the trim from `x0` to `x1`, which leaves out a spine.
pub(super) fn visit(
    n: &Node,
    p: &Page,
    trim: Option<[f64; 2]>,
    snap: &Snapshot,
    out: &mut Vec<Issue>,
) {
    if n.hidden {
        return;
    }
    let mut problems = Vec::new();
    let s = &n.style;
    let paints = || s.fills.iter().chain(&s.strokes).filter(|f| f.visible);
    let mut colors: Vec<&Color> = paints()
        .flat_map(|f| match f.kind {
            FillKind::Solid => vec![&f.color],
            FillKind::Linear | FillKind::Radial => f.stops.iter().map(|s| &s.color).collect(),
            FillKind::Image => vec![],
        })
        .chain(
            s.effects
                .iter()
                .filter(|e| e.visible && e.kind == EffectKind::DropShadow)
                .map(|e| &e.color),
        )
        .collect();
    let mut children: &[Node] = &[];
    match &n.kind {
        Kind::Text {
            content,
            story,
            overset,
            ..
        } => {
            if *overset {
                problems.push(Problem::Overset);
            }
            if *story == n.id {
                colors.extend(content.spans.iter().filter_map(|s| s.attrs.fill.as_ref()));
                problems.extend(
                    snap.missing_fonts
                        .iter()
                        .filter(|m| m.stories.contains(story))
                        .map(|m| Problem::MissingFont {
                            font: m.font.name.clone(),
                        }),
                );
            }
        }
        Kind::Group { children: c } | Kind::Frame { children: c, .. } => children = c,
        Kind::Shape(_) => {}
    }
    let [x, y, w, h] = n.bounds;
    if let Some([x0, x1]) = trim
        && !matches!(n.kind, Kind::Text { .. })
        && x < x1
        && x + w > x0
        && y < p.height
        && y + h > 0.0
    {
        let insets = [x - x0, x1 - x - w, y, p.height - y - h];
        let near = insets
            .iter()
            .copied()
            .filter(|d| (0.01..SAFE - 0.01).contains(d));
        if insets.iter().any(|&d| d < 0.01 && d > 0.01 - p.bleed) {
            problems.push(Problem::ShortOfBleed);
        } else if let Some(distance) = near.reduce(f64::min) {
            problems.push(Problem::NearTrim { distance });
        }
    }
    let scope = Scope {
        palette: &snap.palette,
        modes: &n.active_modes,
    };
    let image = paints().any(|f| f.kind == FillKind::Image);
    if snap.color_mode == ColorMode::Cmyk
        && (image || colors.iter().any(|c| c.ink(&scope) == Ink::Rgb))
    {
        problems.push(Problem::Rgb);
    }
    if snap.color_mode == ColorMode::Cmyk {
        let inks: Vec<Ink> = colors.iter().map(|c| c.ink(&scope)).collect();
        let rgb = colors.iter().zip(&inks).filter(|(_, i)| **i == Ink::Rgb);
        if rgb.clone().any(|(c, _)| {
            let [r, g, b, _] = c.rgba(&scope);
            color::out_of_gamut([r, g, b])
        }) {
            problems.push(Problem::Gamut);
        }
        let ink = colors
            .iter()
            .zip(&inks)
            .map(|(c, i)| match i {
                Ink::Spot { tint, .. } => f64::from(*tint) * 100.0,
                _ => f64::from(i.cmyk(&c.rgba(&scope)).iter().sum::<f32>()) * 100.0,
            })
            .fold(0.0, f64::max);
        if ink > snap.ink_limit + 0.5 {
            problems.push(Problem::Ink { ink });
        }
    }
    if let Some(ppi) = n.ppi.filter(|&ppi| ppi < PRINT_PPI - 0.5) {
        problems.push(Problem::LowPpi { ppi });
    }
    out.extend(problems.into_iter().map(|problem| Issue {
        page: p.id.clone(),
        layer: n.id.clone(),
        name: n.name.clone(),
        problem,
    }));
    for c in children {
        visit(c, p, None, snap, out);
    }
}

/// Sets the page number of the text layers among `nodes` to `number` and fits the
/// hugging sides of each to it, as `Doc::fit` does.
pub(super) fn renumber(nodes: &mut [Node], number: &str) {
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

pub(super) fn page_op(p: &Page) -> Op {
    Op::Page {
        width: p.width as f32,
        height: p.height as f32,
        bleed: p.bleed as f32,
    }
}

pub(super) fn draw_all(nodes: &[Node], ops: &mut Vec<Op>, pal: &Palette) {
    for (i, n) in nodes.iter().enumerate() {
        if n.style.mask && !n.hidden {
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

pub(super) fn draw(n: &Node, ops: &mut Vec<Op>, pal: &Palette) {
    if n.hidden {
        return;
    }
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
    let turned = n.rotation != 0.0;
    if turned {
        ops.push(Op::PushTransform {
            transform: geom::rotation(n.rotation, frame.map(f64::from)).map(|v| v as f32),
        });
    }
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
    if turned {
        ops.push(Op::PopTransform);
    }
    if wrapped {
        ops.push(Op::PopLayer);
    }
}

/// Like `draw_all`, a node is only hit inside every mask below it among its siblings.
/// Locked layers are skipped unless `locks` is false, as for masks.
pub(super) fn hit(
    nodes: &[Node],
    x: f64,
    y: f64,
    tolerance: f64,
    locks: bool,
    path: &mut Vec<String>,
) -> bool {
    for (i, n) in nodes.iter().enumerate().rev() {
        if n.hidden || locks && n.locked {
            continue;
        }
        let masks = nodes[..i].iter().filter(|m| m.style.mask && !m.hidden);
        if masks.into_iter().any(|m| {
            !hit(
                std::slice::from_ref(m),
                x,
                y,
                tolerance,
                false,
                &mut Vec::new(),
            )
        }) {
            continue;
        }
        let [a, b, c, d, e, f] = geom::rotation(-n.rotation, [n.x, n.y, n.w, n.h]);
        let (x, y) = (a * x + c * y + e, b * x + d * y + f);
        let inside = x >= n.x && x <= n.x + n.w && y >= n.y && y <= n.y + n.h;
        path.push(n.id.clone());
        let found = match &n.kind {
            Kind::Group { children } => hit(children, x, y, tolerance, locks, path),
            Kind::Frame { children, clip } => {
                (inside || !clip) && hit(children, x, y, tolerance, locks, path) || inside
            }
            Kind::Shape(s) => {
                let p = outline(s, [n.x, n.y, n.w, n.h].map(|v| v as f32));
                let (x, y) = (x as f32, y as f32);
                let stroke = if n.style.strokes.iter().any(|f| f.visible) {
                    n.style.stroke_weight / 2.0
                } else {
                    0.0
                };
                closed(&p) && contains(&p, x, y) || near(&p, x, y, tolerance as f32 + stroke)
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

impl Doc {
    /// The snapshot kept since the last command.
    pub fn snapshot(&self) -> Rc<Snapshot> {
        match &*self.snapshot.borrow() {
            Some(s) => s.clone(),
            None => Rc::new(self.build_snapshot()),
        }
    }

    pub(crate) fn build_snapshot(&self) -> Snapshot {
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
                detached: serde_json::from_value(v[DETACHED].clone()).unwrap_or_default(),
                grids: serde_json::from_value(v[GRIDS].clone()).unwrap_or_default(),
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
        let stories: BTreeMap<_, _> = flows
            .values()
            .map(|f| (f.head.to_string(), f.story.clone()))
            .collect();
        let mut missing_fonts: Vec<MissingFont> = Vec::new();
        for (id, story) in &stories {
            for font in story.spans.iter().filter_map(|s| s.attrs.font.as_ref()) {
                if text::font_id(&Some(font.clone())) & text::MISSING == 0 {
                    continue;
                }
                match missing_fonts.iter_mut().find(|m| m.font.hash == font.hash) {
                    Some(m) if m.stories.last() == Some(id) => {}
                    Some(m) => m.stories.push(id.clone()),
                    None => missing_fonts.push(MissingFont {
                        font: font.clone(),
                        stories: vec![id.clone()],
                    }),
                }
            }
        }
        let document = self.doc.get_map("document");
        let mut snap = Snapshot {
            fonts: text::fonts(),
            missing_fonts,
            images: self
                .doc
                .get_map("images")
                .keys()
                .filter_map(|h| Some((h.to_string(), image::info(&h)?.space)))
                .collect(),
            preflight: Vec::new(),
            stories,
            spreads: spreads
                .iter()
                .map(|s| s.iter().map(|&i| pages[i].id.clone()).collect())
                .collect(),
            pages,
            masters: self.masters().into_iter().map(sheet).collect(),
            facing_pages,
            raster_ppi: num(&document, "rasterPpi"),
            ink_limit: Some(num(&document, "inkLimit"))
                .filter(|&l| l > 0.0)
                .unwrap_or(300.0),
            color_mode: self.color_mode(),
            preset: match self.setting("preset", Preset::X4) {
                Preset::X1a if self.color_mode() == ColorMode::Rgb => Preset::X4,
                p => p,
            },
            crop_marks: self.setting("cropMarks", true),
            include_bleed: self.setting("includeBleed", true),
            palette,
            can_undo: self.undo.can_undo(),
            can_redo: self.undo.can_redo(),
        };
        snap.preflight = preflight(&snap);
        snap
    }

    /// The display list of the page `id` for the canvas, empty when there is none.
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

    /// The display list of the page `id` as one plate of its print, C, M and Y or K
    /// in all three, inverted to RGB so that the canvas blends its inks on paper.
    pub fn plate(&self, id: &str, k: bool) -> Vec<Op> {
        let mut ops = recolor(&self.render(id), &|c, ink| {
            let [cyan, m, y, black] = ink.cmyk(c);
            let [r, g, b] = if k { [black; 3] } else { [cyan, m, y] };
            [1.0 - r, 1.0 - g, 1.0 - b, c[3]]
        });
        for op in &mut ops {
            if let Op::Image { image, .. } = op {
                *image |= if k { image::K } else { image::CMY };
            }
        }
        ops
    }

    /// The display list of the page `id` as it prints in FOGRA51: RGB colours and images
    /// show separated.
    pub fn proof(&self, id: &str) -> Vec<Op> {
        let mut ops = recolor(&self.render(id), &|c, ink| match ink {
            Ink::Rgb => {
                let [r, g, b] = color::to_rgb(color::to_cmyk([c[0], c[1], c[2]]));
                [r, g, b, c[3]]
            }
            _ => *c,
        });
        for op in &mut ops {
            if let Op::Image { image, .. } = op {
                *image |= image::PROOF;
            }
        }
        ops
    }

    /// The inks of the page `id` as it prints at `ppi`.
    pub fn inks(&self, id: &str, ppi: f32) -> Option<Inks> {
        let snap = self.snapshot();
        let p = snap.pages.iter().find(|p| p.id == id)?;
        Inks::new(&self.print(&snap, p), ppi)
    }

    /// The document as a PDF titled `title`, made at `date` in ISO 8601 UTC, every
    /// page from one snapshot.
    pub fn pdf(&self, title: &str, date: &str) -> Vec<u8> {
        let snap = self.snapshot();
        let pages: Vec<_> = snap.pages.iter().map(|p| self.print(&snap, p)).collect();
        crate::pdf::pdf(
            &pages,
            &crate::pdf::Export {
                preset: snap.preset,
                crop_marks: snap.crop_marks,
                bleed: snap.include_bleed,
                ppi: snap.raster_ppi as f32,
                mode: snap.color_mode,
                title,
                date,
            },
        )
    }

    /// The display list of the page `p` as it prints: with the layers of the other
    /// page of its spread, so that one across the spine prints on both.
    pub(super) fn print(&self, snap: &Snapshot, p: &Page) -> Vec<Op> {
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
    pub(super) fn draw_sheet(
        &self,
        snap: &Snapshot,
        p: &Page,
        window: Option<[f64; 4]>,
        ops: &mut Vec<Op>,
    ) {
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
    pub(super) fn master_layers(&self, m: &Page, p: &Page) -> Vec<Node> {
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
    pub(super) fn text_frame_of(&self, n: TreeID) -> TextFrame {
        let v = serde_json::to_value(self.meta(n).get_deep_value()).unwrap_or_default();
        TextFrame {
            number: self.number(self.root(n)),
            ..serde_json::from_value(v).unwrap_or_default()
        }
    }

    /// What a page number on the page or master `root` shows: the page's number, or
    /// the master's prefix.
    pub(super) fn number(&self, root: TreeID) -> String {
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
            hit(&self.master_layers(m, p), x, y, tolerance, true, &mut path);
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
            hit(&p.children, x, y, tolerance, true, &mut path);
        }
        path
    }

    pub(super) fn snap(
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
        let kind = match self.kind(id) {
            Some(NodeKind::Text) => match flows.get(&id) {
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
            Some(NodeKind::Group) => Kind::Group {
                children: children(),
            },
            Some(NodeKind::Frame) => Kind::Frame {
                clip: v["clip"] == true,
                children: children(),
            },
            _ => Kind::Shape(serde_json::from_value(v.clone()).unwrap_or(Shape::Rect {
                radius: 0.0,
                corners: vec![],
            })),
        };
        let style: Style = serde_json::from_value(v.clone()).unwrap_or_default();
        let name = v["name"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| match &kind {
                Kind::Shape(Shape::Rect { .. }) => "Rectangle".into(),
                Kind::Shape(Shape::Ellipse { .. }) => "Ellipse".into(),
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
                Kind::Text { content, from, .. } => {
                    let t: String = content.text[*from..]
                        .chars()
                        .take(41)
                        .map(|c| if c == text::PAGE_NUMBER { '#' } else { c })
                        .collect();
                    match t.char_indices().nth(40) {
                        Some((end, _)) => {
                            let cut = t[..end].rfind(' ').unwrap_or(end);
                            format!("{}…", t[..cut].trim_end_matches([',', '.', ';', ':']))
                        }
                        None => t,
                    }
                }
                Kind::Group { .. } => "Group".into(),
                Kind::Frame { .. } => "Frame".into(),
            });
        let [x, y, w, h] = self.bounds(id);
        let turn = self.turn(id);
        let own = union(
            [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]
                .into_iter()
                .map(|p| geom::apply(turn, p))
                .map(|[x, y]| [x, y, 0.0, 0.0]),
        );
        let bounds = match &kind {
            Kind::Group { children } => union(children.iter().map(|c| c.bounds)),
            Kind::Frame {
                clip: false,
                children,
            } => union([own].into_iter().chain(children.iter().map(|c| c.bounds))),
            _ => own,
        };
        Node {
            id: id.to_string(),
            name,
            x,
            y,
            w,
            h,
            bounds,
            modes,
            active_modes,
            bindings: self.bindings(id),
            override_of: v[OVERRIDE_OF].as_str().map(String::from),
            hidden: v["hidden"] == true,
            locked: v["locked"] == true,
            rotation: v["rotation"].as_f64().unwrap_or(0.0),
            ppi: style
                .fills
                .iter()
                .filter(|f| f.visible)
                .filter_map(|f| f.ppi(w, h))
                .reduce(f64::min),
            layout: serde_json::from_value(v.clone()).unwrap_or_default(),
            style,
            kind,
        }
    }

    /// The text of `id` in runs of equal attributes: the layer's, under those of its
    /// text style, under the marks; paragraph attributes from each paragraph's start.
    pub(super) fn spans(&self, id: TreeID, node: &serde_json::Value, s: &Scope) -> Vec<Span> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

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
    fn render_emits_items_and_clips_frame_children() {
        let ops = page_ops(&Doc::sample());
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
        assert!(Doc::sample().render("9@9").is_empty());
    }

    #[test]
    fn a_flattened_shape_is_a_path_along_its_outline() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [10.0, 20.0, 40.0, 30.0]);
        set(
            &mut d,
            &r,
            Props {
                radius: Some(5.0),
                ..Props::default()
            },
        );
        let fills = |d: &Doc| {
            page_ops(d)
                .into_iter()
                .filter_map(|o| match o {
                    Op::FillPath { path, .. } => Some(path),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let before = fills(&d);
        d.apply(Command::Flatten { id: r.clone() }).unwrap();
        assert!(matches!(
            page(&d).children[0].kind,
            Kind::Shape(Shape::Path { .. })
        ));
        let after = fills(&d);
        assert_eq!(before.len(), after.len());
        for (a, b) in before.concat().iter().zip(after.concat()) {
            assert!((a - b).abs() < 1e-3, "{a} {b}");
        }
        assert!(d.apply(Command::Flatten { id: p }).is_err());
    }

    #[test]
    fn a_rotated_layer_is_drawn_and_hit_turned_around_its_centre() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 4.0]);
        set(
            &mut d,
            &r,
            Props {
                rotation: Some(90.0),
                ..Props::default()
            },
        );
        assert_eq!(hits(&d, 10.0, 10.0, 0.0), std::slice::from_ref(&r));
        assert!(hits(&d, 18.0, 2.0, 0.0).is_empty());
        let ops = page_ops(&d);
        let Some(Op::PushTransform { transform }) =
            ops.iter().find(|o| matches!(o, Op::PushTransform { .. }))
        else {
            panic!("no transform");
        };
        let [a, b, c, dd, e, f] = *transform;
        let (x, y) = (a * 20.0 + c * 2.0 + e, b * 20.0 + dd * 2.0 + f);
        assert!((x - 10.0).abs() < 1e-4 && (y + 8.0).abs() < 1e-4, "{x} {y}");
        assert!(ops.contains(&Op::PopTransform));
        let moved = shift(ops.clone(), 5.0, 0.0);
        let Some(Op::PushTransform { transform: t }) =
            moved.iter().find(|o| matches!(o, Op::PushTransform { .. }))
        else {
            panic!("no transform");
        };
        assert!((t[0] * 25.0 + t[2] * 2.0 + t[4] - 15.0).abs() < 1e-4);

        d.apply(Command::Flatten { id: r.clone() }).unwrap();
        let n = &page(&d).children[0];
        assert_eq!(n.rotation, 0.0);
        assert!(
            frame(n)
                .iter()
                .zip([8.0, -8.0, 4.0, 20.0])
                .all(|(a, b)| (a - b).abs() < 1e-3)
        );
        assert_eq!(hits(&d, 10.0, 10.0, 0.0), [r]);
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
        let id = create(&mut d, &p, NewKind::Line, [0.0, 0.0, 10.0, 0.0]);
        set(
            &mut d,
            &id,
            Props {
                arrow_end: Some(true),
                ..Props::default()
            },
        );
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
    fn preflight_reports_overset_missing_fonts_short_bleeds_and_rgb_in_cmyk() {
        let (mut d, p) = empty();
        facing(&mut d, false);
        let Page {
            width: w,
            height: h,
            bleed: b,
            ..
        } = page(&d);
        let bleeds = create(&mut d, &p, NewKind::Rect, [-b, -b, w + 2.0 * b, 20.0]);
        let short = create(&mut d, &p, NewKind::Rect, [0.0, 50.0, 20.0, 20.0]);
        let inside = create(&mut d, &p, NewKind::Rect, [20.0, 100.0, 20.0, 20.0]);
        create(&mut d, &p, NewKind::Rect, [w + 50.0, h - 20.0, 20.0, 20.0]);
        let t = fixed_text(&mut d, &p, [0.0, 200.0, 40.0, 12.0]);
        set_text(&mut d, &t, SAMPLE);
        let gone = Typeface {
            name: "Gone Sans".into(),
            hash: "0123456789abcdef".into(),
        };
        format(&mut d, &t, None, in_font(gone)).unwrap();
        let font = Problem::MissingFont {
            font: "Gone Sans".into(),
        };
        assert_eq!(
            problems(&d),
            [
                (short.clone(), Problem::ShortOfBleed),
                (t.clone(), Problem::Overset),
                (t.clone(), font),
            ]
        );
        let inks = |kind| Props {
            fills: Some(vec![Fill {
                kind,
                stops: vec![FillStop {
                    at: 0.0,
                    color: process(0.0, 0.0, 0.0, 1.0),
                }],
                ..Fill::default()
            }]),
            effects: Some(vec![Effect {
                kind: EffectKind::Blur,
                ..Effect::default()
            }]),
            ..Props::default()
        };
        let gradient = create(&mut d, &p, NewKind::Rect, [20.0, 150.0, 20.0, 20.0]);
        set(&mut d, &gradient, inks(FillKind::Linear));
        cmyk(&mut d);
        assert!(!problems(&d).contains(&(gradient, Problem::Rgb)));
        let rgb: Vec<_> = problems(&d)
            .into_iter()
            .filter(|i| i.1 == Problem::Rgb)
            .map(|i| i.0)
            .collect();
        assert!([bleeds, short, inside, t].iter().all(|id| rgb.contains(id)));
    }

    #[test]
    fn preflight_reports_colours_out_of_gamut_ink_over_the_limit_and_layers_near_the_trim_where_they_show()
     {
        let (mut d, p) = empty();
        facing(&mut d, false);
        let solid = |color| Props {
            fills: Some(vec![Fill {
                color,
                ..Fill::default()
            }]),
            ..Props::default()
        };
        let near = create(&mut d, &p, NewKind::Rect, [5.0, 100.0, 20.0, 20.0]);
        let red = create(&mut d, &p, NewKind::Rect, [50.0, 100.0, 20.0, 20.0]);
        let dark = create(&mut d, &p, NewKind::Rect, [100.0, 100.0, 20.0, 20.0]);
        set(&mut d, &near, solid(process(0.0, 0.0, 0.0, 1.0)));
        set(&mut d, &red, solid(Color::Rgb(0xff0000ff)));
        set(&mut d, &dark, solid(process(1.0, 1.0, 1.0, 1.0)));
        assert_eq!(
            problems(&d),
            [(near.clone(), Problem::NearTrim { distance: 5.0 })]
        );
        let turned = create(&mut d, &p, NewKind::Rect, [3.0, 150.0, 20.0, 20.0]);
        set(
            &mut d,
            &turned,
            Props {
                rotation: Some(45.0),
                ..Props::default()
            },
        );
        assert!(problems(&d).contains(&(turned, Problem::ShortOfBleed)));
        cmyk(&mut d);
        let found = problems(&d);
        assert!(found.contains(&(red.clone(), Problem::Gamut)));
        assert!(found.contains(&(dark.clone(), Problem::Ink { ink: 400.0 })));
        assert_eq!(found.iter().filter(|i| i.0 == near).count(), 1);
        let limit = |ink_limit| {
            Command::SetDocument(Settings {
                ink_limit: Some(ink_limit),
                ..Settings::default()
            })
        };
        d.apply(limit(400.0)).unwrap();
        assert!(!problems(&d).iter().any(|i| i.0 == dark));
        assert!(d.apply(limit(401.0)).is_err());
        assert_eq!(d.build_snapshot().ink_limit, 400.0);
    }

    #[test]
    fn pages_rasterize_the_coverage_of_each_ink_and_the_colours_out_of_gamut() {
        let (mut d, p) = empty();
        let Page {
            width,
            height,
            bleed,
            ..
        } = page(&d);
        let solid = |color| Props {
            fills: Some(vec![Fill {
                color,
                ..Fill::default()
            }]),
            ..Props::default()
        };
        let dark = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 72.0, 72.0]);
        set(&mut d, &dark, solid(process(0.5, 0.0, 0.0, 1.0)));
        let orange = swatch(&mut d, "Orange", process(0.0, 0.5, 1.0, 0.0), true).unwrap();
        let spot = create(&mut d, &p, NewKind::Rect, [0.0, 144.0, 72.0, 72.0]);
        set(&mut d, &spot, solid(bound(&orange, 0.5)));
        let red = create(&mut d, &p, NewKind::Rect, [144.0, 0.0, 72.0, 72.0]);
        set(&mut d, &red, solid(Color::Rgb(0xff0000ff)));
        cmyk(&mut d);
        let inks = d.inks(&p, 10.0).unwrap();
        let px = |v: f64| (v * 10.0 / 72.0) as u32;
        let size = |v: f64| (v * 10.0 / 72.0).ceil() as u32;
        assert_eq!(
            (inks.width(), inks.height()),
            (size(width + 2.0 * bleed), size(height + 2.0 * bleed))
        );
        assert_eq!(inks.spots(), ["Orange"]);
        let max = inks.max();
        assert_eq!(max.len(), 5);
        assert!((max[3] - 100.0).abs() < 1.0 && (max[4] - 50.0).abs() < 1.0);
        let at =
            |x: f64, y: f64| (px(y + bleed + 36.0) * inks.width() + px(x + bleed + 36.0)) as usize;
        let coverage = inks.coverage();
        assert!((coverage[at(0.0, 0.0)] - 150.0).abs() < 1.0);
        assert!((coverage[at(0.0, 144.0)] - 50.0).abs() < 1.0);
        let image = inks.image(0b11111, 120.0, true, true);
        assert_eq!(image[at(0.0, 0.0) * 4..][..4], [204, 47, 131, 255]);
        assert_ne!(image[at(144.0, 0.0) * 4 + 3], 0);
        assert_eq!(image[at(0.0, 144.0) * 4 + 3], 0);
        let black = inks.image(0b01000, 400.0, false, false);
        let [r, g, b, _] = black[at(0.0, 0.0) * 4..][..4] else {
            unreachable!()
        };
        assert!(r < 80 && g < 80 && b < 80);
        assert!(black[at(0.0, 144.0) * 4] > 240);
    }

    #[test]
    fn the_proof_shows_rgb_colours_as_they_print() {
        let (mut d, p) = empty();
        let red = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 72.0, 72.0]);
        set(
            &mut d,
            &red,
            Props {
                fills: Some(vec![Fill {
                    color: Color::Rgb(0xff0000ff),
                    ..Fill::default()
                }]),
                ..Props::default()
            },
        );
        let solid = |ops: Vec<Op>| {
            ops.into_iter().find_map(|op| match op {
                Op::FillPath {
                    paint: Paint::Solid { color, .. },
                    ..
                } => Some(color),
                _ => None,
            })
        };
        let [r, g, b, _] = solid(d.proof(&p)).unwrap();
        let [pr, pg, pb] = color::to_rgb(color::to_cmyk([1.0, 0.0, 0.0]));
        assert!((r - pr).abs() < 1e-4 && (g - pg).abs() < 1e-4 && (b - pb).abs() < 1e-4);
        assert!(g > 0.1 || b > 0.1);
        assert_eq!(solid(d.render(&p)).unwrap(), [1.0, 0.0, 0.0, 1.0]);
        let [c, m, y, k] = color::to_cmyk([1.0, 0.0, 0.0]);
        let [r, g, b, a] = solid(d.plate(&p, false)).unwrap();
        assert!((r - (1.0 - c)).abs() < 1e-4 && (g - (1.0 - m)).abs() < 1e-4);
        assert!((b - (1.0 - y)).abs() < 1e-4 && a == 1.0);
        let [r, g, b, _] = solid(d.plate(&p, true)).unwrap();
        assert!((r - (1.0 - k)).abs() < 1e-4 && r == g && g == b);
    }

    #[test]
    fn a_hidden_layer_and_its_children_are_not_drawn_hit_or_preflighted_until_undone() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 50.0, 20.0, 20.0]);
        let b = create(&mut d, &p, NewKind::Rect, [0.0, 50.0, 10.0, 10.0]);
        let g = d
            .apply(Command::Group {
                ids: vec![b.clone()],
                frame: false,
            })
            .unwrap()
            .remove(0);
        cmyk(&mut d);
        assert!(!page(&d).children.iter().any(|n| n.hidden || n.locked));
        let before = page_ops(&d);
        let shown = problems(&d);
        assert!(shown.iter().any(|i| i.0 == b));
        let hide = |hidden| Props {
            hidden: Some(hidden),
            ..Props::default()
        };
        set(&mut d, &g, hide(true));
        assert!(page(&d).children[1].hidden);
        assert_eq!(items(&page_ops(&d)).len(), 1);
        assert_eq!(hits(&d, 5.0, 55.0, 0.0), std::slice::from_ref(&a));
        assert!(!problems(&d).iter().any(|i| i.0 == b));
        set(&mut d, &a, hide(true));
        assert!(items(&page_ops(&d)).is_empty());
        assert!(hits(&d, 5.0, 55.0, 0.0).is_empty());
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(page_ops(&d), before);
        assert_eq!(problems(&d), shown);
    }

    #[test]
    fn a_locked_layer_and_its_children_are_drawn_but_not_hit() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 5.0, 5.0]);
        assert_eq!(hits(&d, 2.0, 2.0, 0.0), [f.clone(), b.clone()]);
        let before = page_ops(&d);
        let lock = |locked| Props {
            locked: Some(locked),
            ..Props::default()
        };
        set(&mut d, &b, lock(true));
        assert_eq!(hits(&d, 2.0, 2.0, 0.0), std::slice::from_ref(&f));
        set(&mut d, &f, lock(true));
        assert_eq!(hits(&d, 2.0, 2.0, 0.0), std::slice::from_ref(&a));
        assert_eq!(page_ops(&d), before);
        set(&mut d, &a, lock(true));
        assert!(hits(&d, 2.0, 2.0, 0.0).is_empty());
    }

    #[test]
    fn a_locked_mask_still_masks_what_is_hit_and_a_hidden_one_masks_nothing() {
        let (mut d, p) = empty();
        let m = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        d.apply(Command::Mask {
            ids: vec![m.clone()],
        })
        .unwrap();
        let props = |hidden, locked| Props {
            hidden: Some(hidden),
            locked: Some(locked),
            ..Props::default()
        };
        set(&mut d, &m, props(false, true));
        assert_eq!(hits(&d, 5.0, 5.0, 0.0), std::slice::from_ref(&a));
        assert!(hits(&d, 15.0, 15.0, 0.0).is_empty());
        set(&mut d, &m, props(true, false));
        assert_eq!(hits(&d, 15.0, 15.0, 0.0), [a]);
        assert!(!page_ops(&d).contains(&Op::BeginMask));
    }

    #[test]
    fn a_layer_on_a_spread_needs_no_bleed_at_the_spine() {
        let (mut d, first) = empty();
        let left = add_page(&mut d, Some(&first));
        let Page {
            width: w,
            height: h,
            bleed: b,
            ..
        } = page(&d);
        create(&mut d, &left, NewKind::Rect, [-b, -b, w + b, h + 2.0 * b]);
        let top = create(&mut d, &left, NewKind::Rect, [w - 10.0, 0.0, 20.0, 10.0]);
        assert_eq!(problems(&d), [(top, Problem::ShortOfBleed)]);
    }
}
