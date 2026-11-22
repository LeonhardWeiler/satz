use super::*;

pub(super) use crate::display_list::Paint;

pub(super) fn page(d: &Doc) -> Rc<Page> {
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

pub(super) fn pdf(d: &Doc) -> Vec<u8> {
    let all: Vec<u32> = (0..d.snapshot().pages.len() as u32).collect();
    d.pdf("Satz", "2026-09-28T12:00:00Z", &all).unwrap()
}

pub(super) fn empty() -> (Doc, String) {
    let mut d = Doc::sample();
    let p = page(&d);
    let ids = p.children.iter().map(|n| n.id.clone()).collect();
    d.apply(Command::Delete { ids }).unwrap();
    (d, p.id.clone())
}

pub(super) fn frame(n: &Node) -> [f64; 4] {
    [n.x, n.y, n.w, n.h]
}

pub(super) fn ids(nodes: &[Rc<Node>]) -> Vec<String> {
    nodes.iter().map(|n| n.id.clone()).collect()
}

pub(super) fn children(n: &Node) -> &[Rc<Node>] {
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

pub(super) fn rgb_fill() -> Props {
    Props {
        fills: Some(vec![Fill::solid(0xd9d9d9ffu32)]),
        ..Props::default()
    }
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

pub(super) fn bind(d: &mut Doc, id: &str, prop: &str, variable: Option<&str>) -> Res<Vec<String>> {
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
    fn find<'a>(nodes: &'a [Rc<Node>], id: &str) -> Option<&'a Node> {
        nodes.iter().find_map(|n| {
            (n.id == id)
                .then_some(&**n)
                .or_else(|| find(children(n), id))
        })
    }
    let pg = page(d);
    let f = find(&pg.children, id).unwrap();
    [frame(f)]
        .into_iter()
        .chain(children(f).iter().map(|n| frame(n)))
        .collect()
}

pub(super) fn sizing(horizontal: Size, vertical: Size) -> Option<Sizing> {
    Some(Sizing {
        horizontal,
        vertical,
    })
}

pub(super) fn text(d: &mut Doc, content: &str) -> String {
    let p = page(d).id.clone();
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
    match &page(d).children.last().unwrap().kind {
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
        props: TextProps {
            size: Some(size),
            line_height: Some(14.0),
            letter_spacing: Some(0.0),
            paragraph_spacing: Some(4.0),
            ..TextProps::default()
        },
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
        crop: false,
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
    fn find(nodes: &[Rc<Node>], id: &str) -> Option<Sizing> {
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
    d.build_snapshot()
        .pages
        .iter()
        .map(|p| p.id.clone())
        .collect()
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

pub(super) fn filled_x(ops: &[Op]) -> Vec<[f32; 2]> {
    ops.iter()
        .filter_map(|o| match o {
            Op::FillPath { path, .. } => {
                let xs: Vec<f32> = crate::geom::segments(path)
                    .flat_map(|(_, p)| p.iter().step_by(2).copied())
                    .collect();
                let min = xs.iter().copied().fold(f32::MAX, f32::min);
                Some([min, xs.iter().copied().fold(f32::MIN, f32::max)])
            }
            _ => None,
        })
        .collect()
}

pub(super) fn add_master(d: &mut Doc) -> String {
    d.apply(Command::AddMaster {
        like: None,
        layers: false,
    })
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

pub(super) fn clips_x(ops: &[Op]) -> Vec<[f32; 2]> {
    let clips = ops.iter().filter_map(|o| match o {
        Op::PushClip { path, .. } => Some(Op::FillPath {
            paint: Paint::Solid {
                color: [0.0; 4],
                ink: Ink::Rgb,
                overprint: false,
            },
            path: path.clone(),
        }),
        _ => None,
    });
    filled_x(&clips.collect::<Vec<_>>())
}

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

pub(super) fn run_texts(ops: &[Op]) -> Vec<String> {
    ops.iter()
        .filter_map(|o| match o {
            Op::GlyphRun { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

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

pub(super) fn flow(
    d: &Doc,
    id: &str,
) -> (String, usize, usize, Option<String>, Option<String>, bool) {
    let s = d.snapshot();
    let all: Vec<&Rc<Node>> = s
        .pages
        .iter()
        .chain(&s.masters)
        .flat_map(|p| &p.children)
        .collect();
    let n = all.into_iter().find(|n| n.id == id).unwrap();
    match &n.kind {
        Kind::Text {
            story,
            prev,
            next,
            overset,
            ..
        } => {
            assert!(story == id || prev.is_some(), "{story} heads {id}");
            let [start, end] = d.frame_range(id).unwrap();
            (
                s.stories[&TreeID::try_from(story.as_str()).unwrap()]
                    .text
                    .clone(),
                start,
                end,
                prev.clone(),
                next.clone(),
                overset.is_some(),
            )
        }
        k => panic!("not text: {k:?}"),
    }
}

pub(super) fn two_pages() -> (Doc, String, String, String, String) {
    let (mut d, p1) = empty();
    let p2 = add_page(&mut d, None);
    let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 2.0 * LEADING + 1.0]);
    let b = fixed_text(&mut d, &p2, [10.0, 10.0, 100.0, 300.0]);
    set_text(&mut d, &a, "Hi\nHi\nHi\nHi");
    thread(&mut d, &a, &b).unwrap();
    (d, p1, p2, a, b)
}

pub(super) struct Booklet {
    d: Doc,
    pages: Vec<String>,
    master: String,
    frames: Vec<String>,
    story: String,
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
    let hash = d.add_image(&image::tests::png(w, h), None).unwrap().hash;
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

fn texts(d: &Doc) -> Vec<(String, usize)> {
    fn walk(nodes: &[Rc<Node>], out: &mut Vec<(String, usize)>) {
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
    let obj = create(&mut d, &p1, NewKind::Rect, [0.0, 0.0, 12.0, 30.0]);
    let table = create(&mut d, &p1, NewKind::Table, [300.0, 300.0, 200.0, 60.0]);
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
        let cmd = match rand(20) {
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
                crop: false,
            },
            5 if rand(2) == 0 => Command::Thread {
                from: id,
                to: t[rand(t.len())].0.clone(),
            },
            5 => Command::ThreadBefore {
                id,
                to: t[rand(t.len())].0.clone(),
            },
            6 => Command::Unthread { id },
            7 => Command::SetTextStyle {
                id: body.clone(),
                name: None,
                grep: None,
                props: TextProps {
                    size: Some(6.0 + rand(20) as f64),
                    ..TextProps::default()
                },
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
            13 => Command::Anchor {
                id: obj.clone(),
                text: id,
                at: x,
            },
            14 if rand(2) == 0 => Command::Unanchor { id: obj.clone() },
            14 => Command::Set {
                id: obj.clone(),
                props: Props {
                    anchored: Some(Anchoring::Custom),
                    ..Props::default()
                },
            },
            12 => Command::Undo,
            15 => Command::TableInsert {
                id: [table.clone(), id][rand(2)].clone(),
                column: rand(2) == 0,
                at: rand(4),
            },
            16 => Command::TableDelete {
                id: table.clone(),
                column: rand(2) == 0,
                at: rand(3),
            },
            17 if rand(2) == 0 => Command::SplitCell { id },
            17 => Command::MergeCells {
                ids: vec![id, t[rand(t.len())].0.clone()],
            },
            18 => Command::SizeTable {
                id: table.clone(),
                columns: vec![frame[2]; 2],
                rows: vec![frame[3] / 4.0; 2],
            },
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
    let s = Doc::sample().snapshot();
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
        ids: vec!["1@1".into()],
    };
    assert_eq!(
        d.apply(bad).err(),
        Some(Error::NotFound {
            what: "node",
            id: "1@1".into()
        })
    );
    d.apply(Command::Delete {
        ids: vec![a.clone()],
    })
    .unwrap();
    assert_eq!(
        d.apply(Command::Delete {
            ids: vec![a.clone()]
        })
        .err(),
        Some(Error::NotFound {
            what: "node",
            id: a
        })
    );
}

#[test]
fn grouping_a_group_with_its_child_groups_the_group() {
    let (mut d, p) = empty();
    let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
    let group = |d: &mut Doc, ids: Vec<String>| {
        d.apply(Command::Group { ids, frame: false })
            .unwrap()
            .remove(0)
    };
    let g = group(&mut d, vec![a.clone()]);
    let outer = group(&mut d, vec![g.clone(), a]);
    let pg = page(&d);
    assert_eq!(ids(&pg.children), [outer]);
    assert_eq!(ids(children(&pg.children[0])), [g]);
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

fn placed(d: &Doc, id: &str) -> [f64; 3] {
    let n = d.node(id).unwrap();
    let [x, y, w, h] = d.bounds(n);
    let [cx, cy] = geom::apply(d.turn(n), [x + w / 2.0, y + h / 2.0]);
    [cx, cy, d.angle(n)].map(|v| (v * 1e6).round() / 1e6)
}

#[test]
fn layers_taken_out_of_a_turned_group_or_frame_stay_where_they_show() {
    for frame in [false, true] {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 4.0]);
        let b = create(&mut d, &p, NewKind::Rect, [0.0, 16.0, 20.0, 4.0]);
        let g = d
            .apply(Command::Group {
                ids: vec![a.clone(), b.clone()],
                frame,
            })
            .unwrap()
            .remove(0);
        for id in [&g, &a] {
            let turned = Props {
                rotation: Some(90.0),
                ..Props::default()
            };
            set(&mut d, id, turned);
        }
        let before = [placed(&d, &a), placed(&d, &b)];
        d.apply(Command::Move {
            ids: vec![a.clone()],
            parent: p.clone(),
            index: 0,
        })
        .unwrap();
        assert_eq!(placed(&d, &a), before[0], "{frame}");
        assert_eq!(placed(&d, &a)[2], 180.0);
        d.apply(Command::Ungroup { ids: vec![g] }).unwrap();
        assert_eq!(placed(&d, &b), before[1], "{frame}");
    }
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
        crop: false,
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
        crop: false,
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
    assert_eq!(d.apply(into_self).err(), Some(Error::IntoItself));
    let into_leaf = Command::Move {
        ids: vec![b],
        parent: a,
        index: 0,
    };
    assert_eq!(d.apply(into_leaf).err(), Some(Error::NotAContainer));
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
    assert_eq!(d.apply(cyclic).err(), Some(Error::IntoItself));
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
    let mut d = Doc::sample();
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
    let mut d = Doc::sample();
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
            crop: false,
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
    let mut d = Doc::sample();
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
fn redo_brings_created_nodes_back_with_their_ids() {
    let (mut d, p) = empty();
    let m = add_master(&mut d);
    use_master(&mut d, &p, Some(&m)).unwrap();
    let a = fixed_text(&mut d, &p, [0.0, 0.0, 50.0, 50.0]);
    let b = fixed_text(&mut d, &p, [60.0, 0.0, 50.0, 50.0]);
    thread(&mut d, &a, &b).unwrap();
    d.apply(Command::Group {
        ids: vec![a, b],
        frame: false,
    })
    .unwrap();
    add_page(&mut d, None);
    let before = d.snapshot();
    let steps = 9;
    for _ in 0..steps {
        d.apply(Command::Undo).unwrap();
    }
    for _ in 0..steps {
        d.apply(Command::Redo).unwrap();
    }
    let after = d.snapshot();
    assert_eq!(after.pages, before.pages);
    assert_eq!(after.masters, before.masters);
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
        (
            Props {
                stroke_weight: Some(-0.1),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "stroke weight",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            Props {
                radius: Some(-1.0),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "radius",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            Props {
                count: Some(2),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "count",
                lo: 3.0,
                hi: 60.0,
            },
        ),
        (
            Props {
                count: Some(61),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "count",
                lo: 3.0,
                hi: 60.0,
            },
        ),
        (
            Props {
                ratio: Some(0.0),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "ratio",
                lo: 0.01,
                hi: 1.0,
            },
        ),
        (
            Props {
                ratio: Some(1.01),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "ratio",
                lo: 0.01,
                hi: 1.0,
            },
        ),
        (
            Props {
                opacity: Some(1.5),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "opacity",
                lo: 0.0,
                hi: 1.0,
            },
        ),
        (
            Props {
                opacity: Some(f32::NAN),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "opacity",
                lo: 0.0,
                hi: 1.0,
            },
        ),
        (
            Props {
                effects: Some(vec![Effect {
                    radius: -1.0,
                    ..Effect::default()
                }]),
                ..Props::default()
            },
            Error::OutOfRange {
                what: "blur",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
    ];
    for (props, e) in bad {
        let what = format!("{props:?}");
        assert_eq!(set(&mut d, &r, props).err(), Some(e), "{what}");
    }
    assert_eq!(
        format(&mut d, &t, None, sized(0.09)).err(),
        Some(Error::OutOfRange {
            what: "text size",
            lo: 0.1,
            hi: MAX_TEXT
        })
    );
    format(&mut d, &t, None, sized(0.1)).unwrap();
    let set_frame = |d: &mut Doc, w, h| {
        d.apply(Command::SetFrame {
            id: r.clone(),
            x: -5.0,
            y: 0.0,
            w,
            h,
            crop: false,
        })
    };
    assert_eq!(
        set_frame(&mut d, -1.0, 1.0).err(),
        Some(Error::OutOfRange {
            what: "width",
            lo: 0.0,
            hi: MAX_LENGTH
        })
    );
    assert_eq!(
        set_frame(&mut d, 1.0, -1.0).err(),
        Some(Error::OutOfRange {
            what: "height",
            lo: 0.0,
            hi: MAX_LENGTH
        })
    );
    set_frame(&mut d, 10.0, 0.0).unwrap();
    assert_eq!(frame(&page(&d).children[0]), [-5.0, 0.0, 10.0, 0.01 * MM]);
}

#[test]
fn text_wraps_around_a_layer_above_it_with_its_offset() {
    let (mut d, p) = empty();
    let t = text(&mut d, "Hi");
    set_frame(&mut d, &t, [0.0, 0.0, 400.0, 400.0]);
    let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 200.0, 50.0]);
    let wrap = |wrap| Props {
        wrap: Some(wrap),
        wrap_offset_x: Some(10.0),
        wrap_offset_y: Some(20.0),
        ..Props::default()
    };
    set(&mut d, &r, wrap(Wrap::Around));
    assert!(close(first_glyph(&d)[0].into(), 210.0));
    set(&mut d, &r, wrap(Wrap::Jump));
    assert!(close(first_glyph(&d)[0].into(), 0.0));
    assert!(first_glyph(&d)[1] > 70.0);
    d.apply(Command::Order {
        ids: vec![t],
        to: Order::Front,
    })
    .unwrap();
    assert!(first_glyph(&d)[1] < 20.0);
}

#[test]
fn a_change_on_one_page_keeps_the_others_of_the_snapshot() {
    let mut d = Doc::sample();
    let first = page(&d);
    d.apply(Command::DuplicatePage {
        id: first.id.clone(),
    })
    .unwrap();
    let before = d.snapshot();
    let rect = &before.pages[0].children[0];
    d.apply(Command::SetFrame {
        id: rect.id.clone(),
        x: rect.x + 10.0,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        crop: false,
    })
    .unwrap();
    let after = d.snapshot();
    assert!(!Rc::ptr_eq(&before.pages[0], &after.pages[0]));
    assert!(Rc::ptr_eq(&before.pages[1], &after.pages[1]));
    set_width(&mut d, &first.id, 300.0);
    let wide = d.snapshot();
    assert!(!Rc::ptr_eq(&after.pages[0], &wide.pages[0]));
    assert!(Rc::ptr_eq(&after.pages[1], &wide.pages[1]));
    let r = create(&mut d, &first.id, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
    assert!(Rc::ptr_eq(&wide.pages[1], &d.snapshot().pages[1]));
    d.apply(Command::Delete { ids: vec![r] }).unwrap();
    let kept = d.snapshot();
    let r = create(&mut d, &first.id, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
    d.apply(Command::Delete { ids: vec![r] }).unwrap();
    assert!(Rc::ptr_eq(&kept.pages[1], &d.snapshot().pages[1]));
}

#[test]
fn a_file_with_a_value_no_command_takes_is_not_opened() {
    let (mut d, p) = empty();
    let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
    let t = create(&mut d, &p, NewKind::Text, [0.0; 4]);
    assert!(Doc::load(&d.save()).is_ok());
    for ((id, k, v), e) in [
        (
            (&r, "strokeWeight", -1.0),
            Error::OutOfRange {
                what: "stroke weight",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            (&r, "w", 1e300),
            Error::OutOfRange {
                what: "width",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            (&p, "height", 1e308),
            Error::OutOfRange {
                what: "page height",
                lo: 1.0,
                hi: MAX_PAGE,
            },
        ),
    ] {
        let d = Doc::load(&d.save()).unwrap();
        let n = TreeID::try_from(id.as_str()).unwrap();
        d.meta(n).insert(k, v).unwrap();
        assert_eq!(Doc::load(&d.save()).err(), Some(e), "{k}");
    }
    let n = TreeID::try_from(t.as_str()).unwrap();
    let text = d.own_text(n).unwrap();
    text.insert(0, "a").unwrap();
    text.mark(0..1, "size", 1e40).unwrap();
    assert_eq!(
        Doc::load(&d.save()).err(),
        Some(Error::OutOfRange {
            what: "text size",
            lo: 0.1,
            hi: MAX_TEXT
        })
    );
}

#[test]
fn a_group_in_an_auto_layout_lays_out_the_same_again() {
    let (mut d, p) = empty();
    let mm = |v: f64| (v + 1.0 / 7.0) * MM;
    let a = create(
        &mut d,
        &p,
        NewKind::Rect,
        [mm(10.0), mm(58.0), mm(16.0), mm(16.0)],
    );
    let b = create(
        &mut d,
        &p,
        NewKind::Rect,
        [mm(147.0), mm(175.0), mm(15.0), mm(10.0)],
    );
    let c = create(
        &mut d,
        &p,
        NewKind::Rect,
        [mm(165.0), mm(175.0), mm(15.0), mm(10.0)],
    );
    let f = d
        .apply(Command::AutoLayout { ids: vec![c] })
        .unwrap()
        .remove(0);
    let g = d
        .apply(Command::Group {
            ids: vec![a, b],
            frame: false,
        })
        .unwrap();
    d.apply(Command::Move {
        ids: g,
        parent: f,
        index: 0,
    })
    .unwrap();
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
        crop: false,
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
        (
            Command::SetDocument(Settings {
                raster_ppi: Some(150.0),
                color_mode: Some(ColorMode::Cmyk),
                ink_limit: Some(500.0),
                ..Settings::default()
            }),
            Error::InkLimit,
        ),
        (
            set(
                &r,
                Props {
                    opacity: Some(2.0),
                    ..Props::default()
                },
            ),
            Error::OutOfRange {
                what: "opacity",
                lo: 0.0,
                hi: 1.0,
            },
        ),
        (
            set(
                &f,
                Props {
                    gap: Some(f64::NAN),
                    ..Props::default()
                },
            ),
            Error::OutOfRange {
                what: "gap",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (
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
            Error::OutOfRange {
                what: "effect offset",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (
            fill(Fill {
                transform: [f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0],
                ..Fill::default()
            }),
            Error::PaintTransform,
        ),
        (
            fill(Fill {
                stops: vec![FillStop {
                    at: 2.0,
                    color: Color::Rgb(BLACK),
                }],
                ..Fill::default()
            }),
            Error::OutOfRange {
                what: "stop",
                lo: 0.0,
                hi: 1.0,
            },
        ),
        (
            new(&p, [0.0, 0.0, -1.0, 1.0]),
            Error::OutOfRange {
                what: "width",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            new(&p, [f64::NAN, 0.0, 1.0, 1.0]),
            Error::OutOfRange {
                what: "x",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (new(&r, [0.0, 0.0, 1.0, 1.0]), Error::NotAContainer),
        (
            frame(&r, [f64::NAN, 0.0, 10.0, 10.0]),
            Error::OutOfRange {
                what: "x",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (
            frame(&r, [0.0, f64::INFINITY, 10.0, 10.0]),
            Error::OutOfRange {
                what: "y",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (
            frame(&p, [0.0, 0.0, 10.0, 10.0]),
            Error::NotALayer { id: p.clone() },
        ),
        (path(vec![LINE, 1.0, 1.0]), Error::NotAPath),
        (path(vec![MOVE, f32::NAN, 0.0]), Error::NotAPath),
        (path(vec![MOVE, 0.0]), Error::NotAPath),
        (
            Command::SetDocument(Settings {
                raster_ppi: Some(5000.0),
                facing_pages: Some(true),
                ..Settings::default()
            }),
            Error::RasterPpi,
        ),
        (
            Command::SetDocument(Settings {
                image_ppi: Some(10.0),
                ..Settings::default()
            }),
            Error::ImagePpi,
        ),
        (
            Command::SetDocument(Settings {
                jpeg_quality: Some(101),
                ..Settings::default()
            }),
            Error::JpegQuality,
        ),
        (
            Command::SetPage {
                id: p.clone(),
                width: Some(100.0),
                height: Some(-1.0),
                bleed: None,
                scale: false,
            },
            Error::OutOfRange {
                what: "page height",
                lo: 1.0,
                hi: MAX_PAGE,
            },
        ),
        (
            Command::SetPage {
                id: p.clone(),
                width: Some(1e6),
                height: None,
                bleed: Some(1e5),
                scale: false,
            },
            Error::OutOfRange {
                what: "page width",
                lo: 1.0,
                hi: MAX_PAGE,
            },
        ),
        (
            frame(&r, [0.0, 0.0, 1e9, 10.0]),
            Error::OutOfRange {
                what: "width",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            new(&p, [-1e9, 0.0, 1.0, 1.0]),
            Error::OutOfRange {
                what: "x",
                lo: -MAX_LENGTH,
                hi: MAX_LENGTH,
            },
        ),
        (
            Command::SetSection {
                id: p.clone(),
                section: Some(Section {
                    start: 10000,
                    style: Numbering::Arabic,
                    prefix: String::new(),
                }),
            },
            Error::SectionStart { max: 9999 },
        ),
        (
            Command::SetGrids {
                id: p.clone(),
                grids: vec![Grid {
                    kind: GridKind::Columns,
                    count: 1000,
                    gutter: 0.0,
                    margin: 0.0,
                    size: 0.0,
                }],
            },
            Error::OutOfRange {
                what: "grid count",
                lo: 1.0,
                hi: 100.0,
            },
        ),
        (
            Command::ScaleText {
                id: t.clone(),
                by: 0.0,
            },
            Error::ScaleNotPositive,
        ),
        (
            Command::SetGrids {
                id: p.clone(),
                grids: vec![Grid {
                    kind: GridKind::Grid,
                    count: 1,
                    gutter: 0.0,
                    margin: 0.0,
                    size: 0.0,
                }],
            },
            Error::OutOfRange {
                what: "cell size",
                lo: 1.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            set(
                &r,
                Props {
                    corners: Some(vec![1.0, 2.0]),
                    ..Props::default()
                },
            ),
            Error::Corners,
        ),
        (
            set(
                &r,
                Props {
                    sweep: Some(2.0),
                    ..Props::default()
                },
            ),
            Error::OutOfRange {
                what: "sweep",
                lo: 0.0,
                hi: 1.0,
            },
        ),
        (
            set(
                &r,
                Props {
                    rotation: Some(f64::NAN),
                    ..Props::default()
                },
            ),
            Error::OutOfRange {
                what: "rotation",
                lo: -360.0,
                hi: 360.0,
            },
        ),
        (
            set(
                &r,
                Props {
                    corners: Some(vec![1.0, 2.0, f32::NAN, 0.0]),
                    ..Props::default()
                },
            ),
            Error::OutOfRange {
                what: "radius",
                lo: 0.0,
                hi: MAX_LENGTH,
            },
        ),
        (
            Command::ScaleText {
                id: r.clone(),
                by: 2.0,
            },
            Error::NotText,
        ),
        (
            Command::ResetToMaster {
                ids: vec![copy, r.clone()],
            },
            Error::NotAnOverride { id: r.clone() },
        ),
        (
            Command::Delete {
                ids: vec![r.clone(), p.clone()],
            },
            Error::NotALayer { id: p.clone() },
        ),
        (
            Command::FillArea {
                id: r.clone(),
                x: 1e6,
                y: 1e6,
            },
            Error::AreaNotPath,
        ),
        (
            Command::Flip {
                id: p.clone(),
                vertical: false,
            },
            Error::NotALayer { id: p.clone() },
        ),
        (
            Command::Format {
                id: t.clone(),
                range: None,
                props: sized(f64::NAN),
            },
            Error::OutOfRange {
                what: "text size",
                lo: 0.1,
                hi: MAX_TEXT,
            },
        ),
        (
            Command::AddTextStyle {
                name: "Body".into(),
                props: TextProps {
                    size: Some(f64::INFINITY),
                    line_height: Some(0.0),
                    letter_spacing: Some(0.0),
                    paragraph_spacing: Some(0.0),
                    ..TextProps::default()
                },
            },
            Error::OutOfRange {
                what: "text size",
                lo: 0.1,
                hi: MAX_TEXT,
            },
        ),
        (
            Command::DeleteMode {
                collection: c.clone(),
                id: "none".into(),
            },
            Error::NotFound {
                what: "mode",
                id: "none".into(),
            },
        ),
        (
            Command::SetVariable {
                id: v.clone(),
                name: None,
                mode: None,
                value: Some(Value::Number(f64::NAN)),
            },
            Error::NotFinite,
        ),
    ];
    let before = d.snapshot();
    for (cmd, e) in bad {
        let what = format!("{cmd:?}");
        assert_eq!(d.apply(cmd).err(), Some(e), "{what}");
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
        crop: false,
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
        crop: false,
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
        crop: false,
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
    assert!(matches!(n.kind, Kind::Frame { clip: false, .. }));
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
        crop: true,
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
fn each_booklet_page_colors_the_story_in_its_own_mode() {
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
fn a_pdf_holds_the_pages_asked_for_in_their_order() {
    let b = booklet();
    let path = std::env::temp_dir().join(format!("satz-range-{}.pdf", std::process::id()));
    std::fs::write(
        &path,
        b.d.pdf("Satz", "2026-09-28T12:00:00Z", &[5, 2]).unwrap(),
    )
    .unwrap();
    let out = std::process::Command::new("mutool")
        .args(["draw", "-q", "-F", "text", "-o", "-"])
        .arg(&path)
        .output()
        .expect("mutool runs");
    std::fs::remove_file(&path).unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let numbers: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.len() == 1)
        .collect();
    assert_eq!(numbers, ["6", "3"]);
}

#[test]
fn an_uploaded_cmyk_profile_is_kept_undone_and_the_pdf_output_intent() {
    let mut d = Doc::blank(100.0, 100.0, 1, false, ColorMode::Cmyk);
    assert_eq!(
        d.set_profile(Some(b"not icc".to_vec())).err(),
        Some(Error::NotIcc)
    );
    let mut icc = include_bytes!("../../icc/FOGRA51.icc").to_vec();
    icc[24] ^= 1;
    let page = d.snapshot().pages[0].id.clone();
    let printed = d.printed(&page);
    d.set_profile(Some(icc)).unwrap();
    assert_ne!(d.printed(&page), printed);
    let name = d.snapshot().profile.clone().unwrap();
    assert!(!name.is_empty());
    let pdf =
        String::from_utf8_lossy(&d.pdf("T", "2026-09-28T12:00:00Z", &[0]).unwrap()).into_owned();
    assert_eq!(name, "FOGRA51 (Satz)");
    assert!(pdf.contains("/OutputConditionIdentifier(Custom)/Info(FOGRA51 \\(Satz\\))"));
    assert_eq!(Doc::load(&d.save()).unwrap().snapshot().profile, Some(name));
    d.apply(Command::Undo).unwrap();
    assert_eq!(d.snapshot().profile, None);
    assert_eq!(d.printed(&page), printed);
    let pdf =
        String::from_utf8_lossy(&d.pdf("T", "2026-09-28T12:00:00Z", &[0]).unwrap()).into_owned();
    assert!(pdf.contains("/OutputConditionIdentifier(FOGRA51)"));
}

#[test]
fn a_package_holds_the_document_its_pdf_fonts_and_used_images() {
    let mut b = booklet();
    let (_, hash) = place(&mut b.d, &b.pages[0], 2, 2);
    b.d.add_image(&image::tests::png(3, 3), None).unwrap();
    let zip = b.d.package("Satz", "2026-09-28T12:00:00Z", &[0]).unwrap();
    let files = crate::zip::unzip(&zip);
    let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(
        names,
        [
            "Satz/Satz.satz",
            "Satz/Satz.pdf",
            &format!("Satz/Fonts/{}.ttf", text::fonts()[0].name),
            &format!("Satz/Images/{hash}.png"),
        ]
    );
    assert!(Doc::load(&files[0].1).is_ok());
    assert!(files[1].1.starts_with(b"%PDF"));
    let at = zip
        .windows(names[3].len())
        .position(|w| w == names[3].as_bytes())
        .unwrap();
    assert_eq!(zip[at - 30 + 8], 0, "the image is stored");
}

#[test]
fn a_package_gives_same_named_images_and_fonts_their_own_files() {
    let mut b = booklet();
    let twin = [MONO, &[0; 4]].concat();
    for (i, font) in [MONO, &twin].into_iter().enumerate() {
        let face = b.d.add_font(font).unwrap();
        format(&mut b.d, &b.frames[0], Some([i, i + 1]), in_font(face)).unwrap();
    }
    for (w, name) in [
        (2, "IMG_0001.JPG"),
        (3, "IMG_0001.JPG"),
        (4, "img_0001.jpg"),
    ] {
        let png = image::tests::png(w, w);
        b.d.add_image(&png, Some(name.into())).unwrap();
    }
    let zip = b.d.package("Satz", "2026-09-28T12:00:00Z", &[0]).unwrap();
    let names: std::collections::BTreeSet<String> = crate::zip::unzip(&zip)
        .into_iter()
        .map(|f| f.0.to_lowercase())
        .filter(|n| n.contains("mono") || n.contains("images/"))
        .collect();
    let mono = text::typeface(MONO).unwrap().name.to_lowercase();
    assert_eq!(
        names,
        [
            format!("satz/fonts/{mono}-2.ttf"),
            format!("satz/fonts/{mono}.ttf"),
            "satz/images/img_0001-2.jpg".into(),
            "satz/images/img_0001-3.jpg".into(),
            "satz/images/img_0001.jpg".into(),
        ]
        .into()
    );
}

#[test]
fn the_booklet_pdf_reads_as_one_story_with_a_number_on_every_page() {
    let b = booklet();
    let path = std::env::temp_dir().join(format!("satz-booklet-{}.pdf", std::process::id()));
    std::fs::write(&path, pdf(&b.d)).unwrap();
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
fn loading_a_document_writes_nothing() {
    use crate::examples::{booklet, heavy, poster};
    for d in [poster(), booklet(), heavy(2)] {
        let loaded = Doc::load(&d.save()).unwrap();
        assert_eq!(loaded.doc.oplog_vv(), d.doc.oplog_vv());
    }
}

#[test]
fn a_synced_document_follows_with_the_changes_alone() {
    use crate::examples::heavy;
    let mut b = booklet();
    let mut other = heavy(1);
    other.sync(&b.d.updates(&[])).unwrap();
    let vv = b.d.version_vector();
    b.d.apply(Command::Delete {
        ids: vec![b.frames[7].clone()],
    })
    .unwrap();
    let changes = b.d.updates(&vv);
    assert!(changes.len() < b.d.updates(&[]).len() / 10);
    other.sync(&changes).unwrap();
    assert_eq!(other.doc.oplog_vv(), b.d.doc.oplog_vv());
    let page = &b.d.snapshot().pages[1].id;
    assert_eq!(other.render(page), b.d.render(page));
    let fresh = heavy(1);
    other.sync(&fresh.updates(&other.version_vector())).unwrap();
    assert_eq!(other.doc.oplog_vv(), fresh.doc.oplog_vv());
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
    assert_eq!(
        loaded.snapshot().stories[&TreeID::try_from(b.frames[0].as_str()).unwrap()].text,
        b.story
    );
}

#[test]
fn a_file_that_is_not_a_satz_document_does_not_load() {
    assert_eq!(Doc::load(b"not a satz file").err(), Some(Error::NotSatz));
    let other = LoroDoc::new();
    other.get_map("x").insert("a", 1).unwrap();
    assert_eq!(
        Doc::load(&other.export(ExportMode::Snapshot).unwrap()).err(),
        Some(Error::NotSatz)
    );
}

#[test]
fn cropping_a_turned_image_keeps_it_where_it_is_on_the_page() {
    let (mut d, p) = empty();
    let (id, _) = place(&mut d, &p, 600, 300);
    let props = Props {
        rotation: Some(30.0),
        ..Props::default()
    };
    d.apply(Command::Set {
        id: id.clone(),
        props,
    })
    .unwrap();
    let image = |d: &Doc| {
        page_ops(d)
            .into_iter()
            .find_map(|o| match o {
                Op::Image { transform, .. } => Some(transform),
                _ => None,
            })
            .unwrap()
    };
    let on_page = |m: [f32; 6], frame| geom::then(m.map(f64::from), geom::rotation(30.0, frame));
    let before = on_page(image(&d), [28.0, 164.0, 144.0, 72.0]);
    let crop = |d: &mut Doc, w| {
        d.apply(Command::SetFrame {
            id: id.clone(),
            x: 40.0,
            y: 170.0,
            w,
            h: 50.0,
            crop: true,
        })
    };
    crop(&mut d, 100.0).unwrap();
    assert_eq!(frame(&page(&d).children[0]), [40.0, 170.0, 100.0, 50.0]);
    let after = on_page(image(&d), [40.0, 170.0, 100.0, 50.0]);
    assert!(
        after.iter().zip(before).all(|(a, b)| (a - b).abs() < 1e-3),
        "{after:?} {before:?}"
    );
    assert_eq!(crop(&mut d, 0.0).err(), Some(Error::CropToNothing));
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
fn an_image_fills_or_fits_its_box_or_the_box_takes_its_shape() {
    let (mut d, p) = empty();
    let (id, _) = place(&mut d, &p, 600, 300);
    set_frame(&mut d, &id, [0.0, 0.0, 100.0, 100.0]);
    let fit = |d: &mut Doc, fit| {
        d.apply(Command::FitImage {
            id: id.clone(),
            fit,
        })
        .unwrap();
        let n = &page(d).children[0];
        (frame(n), n.style.fills[0].transform)
    };
    assert_eq!(fit(&mut d, Fit::Cover).1, [2.0, 0.0, 0.0, 1.0, -0.5, 0.0]);
    assert_eq!(fit(&mut d, Fit::Contain).1, [1.0, 0.0, 0.0, 0.5, 0.0, 0.25]);
    assert_eq!(
        fit(&mut d, Fit::Frame),
        ([0.0, 0.0, 100.0, 50.0], [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    );
    let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
    assert_eq!(
        d.apply(Command::FitImage {
            id: r,
            fit: Fit::Cover
        })
        .err(),
        Some(Error::NoImageFill)
    );
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
    assert_eq!(problems(&d), [(id.clone(), Problem::LowPpi { ppi: 150.0 })]);
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
#[allow(clippy::disallowed_methods)]
fn a_saved_document_leaves_out_the_images_no_layer_uses() {
    let (mut d, p) = empty();
    let (kept, a) = place(&mut d, &p, 60, 30);
    let (gone, b) = place(&mut d, &p, 40, 30);
    let before = d.doc.oplog_frontiers();
    d.apply(Command::Delete { ids: vec![gone] }).unwrap();
    let saved = d.save();
    assert!(d.image(&b).is_ok());
    let raw = LoroDoc::new();
    raw.import(&saved).unwrap();
    assert_eq!(
        raw.checkout(&before).err(),
        Some(loro::LoroError::SwitchToVersionBeforeShallowRoot)
    );
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
    assert_eq!(
        apply(&id, fills(Fill::image("0123456789abcdef"))).err(),
        Some(Error::NoImage {
            hash: "0123456789abcdef".into()
        })
    );
    let strokes = Props {
        strokes: Some(vec![Fill::image(&hash)]),
        ..Props::default()
    };
    assert_eq!(apply(&id, strokes).err(), Some(Error::ImageStroke));
    let t = text(&mut d, "Hi");
    let mut apply = |id: &str, props| {
        d.apply(Command::Set {
            id: id.into(),
            props,
        })
    };
    assert_eq!(
        apply(&t, fills(Fill::image(&hash))).err(),
        Some(Error::ImageTextFill)
    );
    assert_eq!(d.add_image(b"GIF89a", None).err(), Some(Error::NotAnImage));
    let mut huge = Vec::new();
    let mut w = png::Encoder::new(&mut huge, 10_000, 5_001)
        .write_header()
        .unwrap();
    w.write_chunk(png::chunk::IDAT, &[]).unwrap();
    drop(w);
    let e = d.add_image(&huge, None).unwrap_err();
    assert_eq!(e, Error::TooManyPixels);
    let other = Doc::default();
    other
        .doc
        .get_map("images")
        .insert("huge", LoroValue::Binary(huge.into()))
        .unwrap();
    let saved = other.doc.export(ExportMode::Snapshot).unwrap();
    assert_eq!(Doc::load(&saved).err(), Some(e));
    let unknown = Command::PlaceImage {
        parent: p,
        image: "0123456789abcdef".into(),
        name: "gone.png".into(),
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };
    assert_eq!(
        d.apply(unknown).err(),
        Some(Error::NoImage {
            hash: "0123456789abcdef".into()
        })
    );
}

#[test]
fn fixture_types() {
    use ts_rs::{Config, TS, TypeVisitor};
    struct Decls(Config, BTreeMap<String, String>);
    impl TypeVisitor for Decls {
        fn visit<T: TS + 'static + ?Sized>(&mut self) {
            if T::output_path().is_none() {
                return;
            }
            let name = T::ident(&self.0);
            if !self.1.contains_key(&name) {
                self.1.insert(name, T::decl(&self.0));
                T::visit_dependencies(self);
            }
        }
    }
    let mut d = Decls(Config::new(), BTreeMap::new());
    d.visit::<Command>();
    d.visit::<Snapshot>();
    d.visit::<Story>();
    d.visit::<crate::style::Style>();
    d.visit::<Layout>();
    d.visit::<crate::text::TextFrame>();
    d.visit::<Props>();
    d.visit::<Palette>();
    d.visit::<history::Version>();
    d.visit::<crate::Delta>();
    d.visit::<crate::Scope>();
    d.visit::<crate::image::ImageInfo>();
    d.visit::<crate::error::Error>();
    let mut ts: String = d.1.values().map(|v| format!("export {v}\n")).collect();
    let list = |l: &[&str]| serde_json::to_string(l).unwrap();
    ts += &format!(
        "export type Sheet = Omit<Page, \"children\"> & {{ children: Array<string | Node> }}\nexport type Changes = Omit<Snapshot, keyof Delta> & Delta\nexport type Process = Extract<Color, number | {{ cmyk: Array<number> }}>\nexport const MM = {MM}\nexport const SELECTION = {:?}\nexport const STYLED = {} as const\nexport type Styled = (typeof STYLED)[number]\nexport const BINDABLE = {} as const\nexport type Bindable = (typeof BINDABLE)[number] | Styled\n",
        SELECTION,
        list(&STYLED),
        list(&BINDABLE),
    );
    crate::fixture("types.ts", ts.as_bytes());
}

#[cfg(feature = "io")]
#[test]
fn damaged_files_fail_without_a_panic() {
    let damaged = |bytes: &[u8]| {
        let n = bytes.len();
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut out: Vec<Vec<u8>> = (0..64).map(|i| bytes[..n * i / 64].to_vec()).collect();
        for _ in 0..64 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut b = bytes.to_vec();
            b[seed as usize % n] ^= 1 << (seed >> 61);
            out.push(b);
        }
        out
    };
    let sample = Doc::sample();
    let saved = sample.save();
    let mut edited = Doc::load(&saved).unwrap();
    let vv = edited.version_vector();
    let ids = page(&edited)
        .children
        .iter()
        .map(|n| n.id.clone())
        .collect();
    edited.apply(Command::Copy { ids }).unwrap();
    let clip = edited.clip_out();
    let ids = page(&edited)
        .children
        .iter()
        .map(|n| n.id.clone())
        .collect();
    edited.apply(Command::Delete { ids }).unwrap();
    let update = edited.updates(&vv);
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="5" height="5" fill="#f00"/></svg>"##;
    let fig = include_bytes!("../../testdata/structure.fig");
    let pdf = pdf(&sample);
    let png = sample.png(&page(&sample).id, 4.0).unwrap();
    let jpeg = include_bytes!("../../testdata/cmyk-black.jpg");
    let ycck = include_bytes!("../../testdata/ycck-cyan.jpg");
    let woff2 = include_bytes!("../../fonts/DMMono-Regular.woff2");
    let icc = include_bytes!("../../icc/FOGRA51.icc");
    type Entry<'a> = (&'a str, &'a [u8], Box<dyn Fn(&[u8]) -> bool + 'a>);
    let entries: Vec<Entry> = vec![
        ("load", &saved, Box::new(|b| Doc::load(b).is_ok())),
        (
            "sync",
            &update,
            Box::new(|b| Doc::load(&saved).unwrap().sync(b).is_ok()),
        ),
        (
            "take_clip",
            &clip,
            Box::new(|b| Doc::default().take_clip(b).is_ok()),
        ),
        (
            "import svg",
            svg,
            Box::new(|b| crate::import::import(b, "a.svg").is_ok()),
        ),
        (
            "import pdf",
            &pdf,
            Box::new(|b| crate::import::import(b, "a.pdf").is_ok()),
        ),
        (
            "import fig",
            fig,
            Box::new(|b| crate::import::import(b, "a.fig").is_ok()),
        ),
        (
            "add_font ttf",
            MONO,
            Box::new(|b| Doc::default().add_font(b).is_ok()),
        ),
        (
            "add_font woff2",
            woff2,
            Box::new(|b| Doc::default().add_font(b).is_ok()),
        ),
        (
            "add_image png",
            &png,
            Box::new(|b| Doc::default().add_image(b, None).is_ok()),
        ),
        (
            "add_image jpeg",
            jpeg,
            Box::new(|b| Doc::default().add_image(b, None).is_ok()),
        ),
        (
            "register_image ycck",
            ycck,
            Box::new(|b| Doc::default().register_image(b).is_ok()),
        ),
        (
            "set_profile",
            icc,
            Box::new(|b| Doc::default().set_profile(Some(b.to_vec())).is_ok()),
        ),
    ];
    let mut panics = vec![];
    for (name, bytes, f) in &entries {
        assert!(f(bytes), "{name} fails on the intact file");
        for (i, b) in damaged(bytes).iter().enumerate() {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(b))).is_err() {
                panics.push(format!("{name} {i}"));
            }
        }
    }
    for (from, to) in [
        ("PASS_THROUGH", "PASS__THROUG"),
        ("PASS_THROUGH", "P_\u{e4}SS_THROU"),
    ] {
        let b = crate::fig::tests::renamed(fig, from, to);
        if std::panic::catch_unwind(|| crate::import::import(&b, "a.fig")).is_err() {
            panics.push(format!("fig blend {to}"));
        }
    }
    assert!(panics.is_empty(), "{panics:?}");
}
