//! The example poster and booklet in `examples/`. `SATZ_WRITE_EXAMPLES=1 cargo test
//! examples` writes them again after the format or the examples changed.

use crate::color::{Color, ColorMode, to_cmyk};
use crate::display_list::Op;
use crate::doc::{Command, Doc, NewKind, Problem, Props, TextProps};
use crate::layout::{Direction, MainAlign, Size, Sizing};
use crate::style::{Effect, EffectKind, Fill, FillKind, FillStop};
use crate::text::{PAGE_NUMBER, TextAlign};
use crate::variable::Value;

const MM: f64 = 72.0 / 25.4;
const PHOTO: &[u8] = include_bytes!("../../examples/earthrise.jpg");
const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples");

/// The headings of the booklet and the pages they are on.
const CONTENTS: [(&str, usize); 5] = [
    ("The flight", 3),
    ("The photograph", 3),
    ("Christmas Eve", 4),
    ("Setting this booklet", 4),
    (FIGURES, 6),
];
const FIGURES: &str = "Apollo 8 in figures";

const STORY: &str = "\
The flight
Apollo 8 left Cape Kennedy on the morning of 21 December 1968. It was the first time a Saturn V rocket carried a crew, and the first time people travelled beyond the orbit of the Earth. Frank Borman, James Lovell and William Anders were on their way to the Moon, a journey of almost three days.
On 24 December the spacecraft passed behind the Moon and fired its engine to enter lunar orbit. For the first time, no one on Earth could hear from a crew. When the signal returned, the three men had seen the far side of the Moon with their own eyes. They would circle it ten times in twenty hours.
Borman was the commander, Lovell the command module pilot and Anders the lunar module pilot, although Apollo 8 carried no lunar module. Its task was to prove that a crew could fly to the Moon, find their way around it and come home, so that later flights could land. Between their duties the three men looked down on a surface no one had seen this close: grey, cratered and without colour, lit at a low angle near the terminator so that every rim and ridge threw a long shadow.
The photograph
The crew had a long list of tasks for each orbit: photographing possible landing sites, navigating by the stars, describing what they saw. On the fourth orbit, as the spacecraft turned, Anders looked out and saw the Earth come up over the lunar horizon. Nobody had planned for the moment. He took a black and white picture first, then asked his crewmates for a magazine of colour film. He used a Hasselblad camera with a 250 mm lens, and there was little time before the view turned away.
The colour frame he took next is catalogued as AS08-14-2383. The world came to know it as Earthrise. It shows a blue and white planet, half in shadow, above a grey and empty landscape. Everything people had ever known lies in that small bright shape, while the ground in front of it had never been touched.
Time magazine named the three astronauts its Men of the Year, and in 1969 the photograph appeared on a United States postage stamp. It is often called one of the most influential photographs ever taken. Many credit it with showing people how small and fragile their world is, a year and a half before the first Earth Day.
Christmas Eve
That evening the crew broadcast pictures of the Moon to Earth and took turns to read the first verses of the Book of Genesis. It was one of the most watched broadcasts of its time. Two days later Apollo 8 left lunar orbit, and on 27 December it splashed down in the Pacific Ocean.
Setting this booklet
This booklet was made with Satz, a desktop publishing program that runs in the browser. Its pages sit on a master spread that carries the running head, the rule below it and the page numbers, so every page shows its own number at its outer edge. The text you are reading is one story that flows through a chain of threaded frames, from page 3 to page 5, and moves on by itself when a frame grows or shrinks.
Satz shapes the text with harfrust and breaks it into lines with the Knuth-Plass algorithm, which weighs every line of a paragraph against the others before it settles on any of them, and hyphenates words where that improves the spacing. The same engine lays out the canvas and writes the PDF, so what the screen shows is what the printer gets.
The document is set up for offset printing in CMYK. The accent colour is a spot colour, an ink of its own on a plate of its own, while the photograph is separated through the FOGRA51 print condition on export. Each page has 3 mm of bleed, and the PDF carries trim and bleed boxes and crop marks. The cover and the back use a dark mode of the colour variables that the inner pages use in light mode.
Before export, preflight checks the document: text that does not fit its frames, fonts that are missing, images below 300 ppi, objects that reach the trim but stop short of the bleed, and RGB colour in a CMYK document. The photograph is an RGB file, so preflight lists it, and the PDF separates it into the four process inks.";

fn cmyk(c: f32, m: f32, y: f32, k: f32, alpha: f32) -> Color {
    Color::Cmyk {
        cmyk: [c, m, y, k],
        alpha,
    }
}

/// The black of the photograph once it is separated, so that it meets its background without an edge.
fn photo_black(alpha: f32) -> Color {
    let [c, m, y, k] = to_cmyk([0.0; 3]);
    cmyk(c, m, y, k, alpha)
}

fn solid(c: Color) -> Option<Vec<Fill>> {
    Some(vec![Fill::solid(c)])
}

fn gradient(kind: FillKind, transform: [f32; 6], stops: Vec<(f32, Color)>) -> Option<Vec<Fill>> {
    Some(vec![Fill {
        kind,
        transform,
        stops: stops
            .into_iter()
            .map(|(at, color)| FillStop { at, color })
            .collect(),
        ..Fill::default()
    }])
}

/// An auto layout frame without fills.
fn auto(direction: Direction, align_main: MainAlign, gap: f64, name: &str) -> Props {
    Props {
        fills: Some(vec![]),
        direction: Some(direction),
        align_main: Some(align_main),
        gap: Some(gap * MM),
        ..named(name)
    }
}

/// Sizing for an auto layout frame that hugs its children's height; set it once
/// they are there, since a frame with nothing to hug keeps its size.
fn hug(horizontal: Size) -> Props {
    Props {
        sizing: Some(Sizing {
            horizontal,
            vertical: Size::Hug,
        }),
        ..Props::default()
    }
}

fn named(name: &str) -> Props {
    Props {
        name: Some(name.into()),
        ..Props::default()
    }
}

struct Build(Doc);

impl Build {
    /// An empty CMYK document with one page `w` × `h` mm.
    fn new(w: f64, h: f64, facing: bool) -> (Build, String) {
        let d = Doc::blank(w * MM, h * MM, 1, facing, ColorMode::Cmyk);
        let page = d.build_snapshot().pages[0].id.clone();
        (Build(d), page)
    }

    fn run(&mut self, cmd: Command) -> Vec<String> {
        self.0.apply(cmd).unwrap()
    }

    fn one(&mut self, cmd: Command) -> String {
        self.run(cmd).remove(0)
    }

    /// Adds a layer at [x, y, w, h] in mm.
    fn add(&mut self, parent: &str, kind: NewKind, [x, y, w, h]: [f64; 4], props: Props) -> String {
        let id = self.one(Command::Create {
            parent: parent.into(),
            kind,
            x: x * MM,
            y: y * MM,
            w: w * MM,
            h: h * MM,
        });
        self.set(&id, props);
        id
    }

    fn set(&mut self, id: &str, props: Props) {
        self.run(Command::Set {
            id: id.into(),
            props,
        });
    }

    fn frame(&mut self, id: &str, [x, y, w, h]: [f64; 4]) {
        self.run(Command::SetFrame {
            id: id.into(),
            x: x * MM,
            y: y * MM,
            w: w * MM,
            h: h * MM,
            ignore_constraints: true,
        });
    }

    /// Adds a text layer [x, y, w, h] mm of a fixed size, auto height when `h` is 0,
    /// or auto width when `w` is 0 too.
    fn text(
        &mut self,
        parent: &str,
        [x, y, w, h]: [f64; 4],
        text: &str,
        format: TextProps,
    ) -> String {
        let id = self.add(parent, NewKind::Text, [x, y, 0.0, 0.0], Props::default());
        self.run(Command::SetText {
            id: id.clone(),
            text: text.into(),
        });
        self.run(Command::Format {
            id: id.clone(),
            range: None,
            props: format,
        });
        if w > 0.0 {
            self.frame(&id, [x, y, w, h.max(1.0)]);
            let vertical = if h == 0.0 { Size::Hug } else { Size::Fixed };
            self.set(
                &id,
                Props {
                    sizing: Some(Sizing {
                        horizontal: Size::Fixed,
                        vertical,
                    }),
                    ..Props::default()
                },
            );
        }
        id
    }

    /// Places the photograph into [x, y, w, h] mm, showing the part of it that
    /// `transform` maps into the layer's unit box.
    fn photo(&mut self, parent: &str, frame: [f64; 4], transform: [f32; 6]) -> String {
        let hash = self.0.add_image(PHOTO).unwrap().hash;
        let id = self.one(Command::PlaceImage {
            parent: parent.into(),
            image: hash.clone(),
            name: "Earthrise".into(),
            x: frame[0] * MM,
            y: frame[1] * MM,
            w: frame[2] * MM,
            h: frame[3] * MM,
        });
        self.set(
            &id,
            Props {
                fills: Some(vec![Fill {
                    transform,
                    ..Fill::image(&hash)
                }]),
                ..Props::default()
            },
        );
        id
    }

    fn swatch(&mut self, name: &str, color: Color, spot: bool) -> Color {
        let swatch = self.one(Command::AddSwatch {
            name: name.into(),
            color,
            spot,
        });
        Color::Swatch {
            swatch,
            tint: 1.0,
            alpha: 1.0,
        }
    }
}

fn poster() -> Doc {
    let (mut b, page) = Build::new(420.0, 594.0, false);
    let p = &page;
    let orange = b.swatch("Satz Orange", cmyk(0.0, 0.55, 1.0, 0.0, 1.0), true);
    let light = cmyk(0.0, 0.0, 0.0, 0.12, 1.0);
    b.add(
        p,
        NewKind::Rect,
        [-3.0, -3.0, 426.0, 600.0],
        Props {
            fills: solid(photo_black(1.0)),
            ..named("Background")
        },
    );
    let mask = b.add(
        p,
        NewKind::Rect,
        [83.0, 30.0, 254.0, 254.0],
        Props {
            fills: gradient(
                FillKind::Radial,
                [0.5, 0.0, 0.0, 0.5, 0.5, 0.5],
                vec![
                    (0.0, photo_black(1.0)),
                    (0.75, photo_black(1.0)),
                    (1.0, photo_black(0.0)),
                ],
            ),
            mask: Some(true),
            ..named("Vignette")
        },
    );
    let photo = b.photo(
        p,
        [83.0, 30.0, 254.0, 254.0],
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    );
    let group = b.one(Command::Group {
        ids: vec![mask, photo],
        frame: false,
    });
    b.set(&group, named("Photograph"));
    b.add(
        p,
        NewKind::Ellipse,
        [70.0, 322.0, 280.0, 36.0],
        Props {
            fills: solid(orange.clone()),
            opacity: Some(0.7),
            effects: Some(vec![Effect {
                kind: EffectKind::Blur,
                radius: 16.0 * MM as f32,
                ..Effect::default()
            }]),
            ..named("Glow")
        },
    );
    let title = b.text(
        p,
        [0.0, 290.0, 420.0, 0.0],
        "Earthrise",
        TextProps {
            size: Some(210.0),
            fill: Some(orange.clone()),
            text_align: Some(TextAlign::Center),
            ..TextProps::default()
        },
    );
    b.set(
        &title,
        Props {
            effects: Some(vec![Effect {
                y: 2.0 * MM as f32,
                radius: 6.0 * MM as f32,
                color: photo_black(0.8),
                ..Effect::default()
            }]),
            ..named("Title")
        },
    );
    b.text(
        p,
        [0.0, 382.0, 420.0, 0.0],
        "Apollo 8  ·  24 December 1968",
        TextProps {
            size: Some(40.0),
            letter_spacing: Some(4.0),
            fill: Some(cmyk(0.0, 0.0, 0.0, 0.0, 1.0)),
            text_align: Some(TextAlign::Center),
            ..TextProps::default()
        },
    );
    let body = b.text(
        p,
        [60.0, 428.0, 300.0, 72.0],
        "On 24 December 1968, on their fourth orbit of the Moon, the crew of Apollo 8 saw the Earth rise over the lunar horizon. William Anders took this photograph through a window of the spacecraft. For the first time, people saw their planet whole, small and bright against the dark, from the sky of another world.\n\
This poster was set with Satz, which lays out pages in the browser and writes print-ready PDF: CMYK with a spot colour, a photograph separated through FOGRA51, 3 mm of bleed and crop marks.",
        TextProps {
            size: Some(19.0),
            line_height: Some(28.0),
            paragraph_spacing: Some(10.0),
            fill: Some(light.clone()),
            text_align: Some(TextAlign::Justify),
            hyphenate: Some(true),
            ..TextProps::default()
        },
    );
    b.set(
        &body,
        Props {
            columns: Some(2),
            gutter: Some(10.0 * MM),
            ..named("Text")
        },
    );
    b.add(
        p,
        NewKind::Line,
        [60.0, 552.0, 300.0, 0.0],
        Props {
            strokes: solid(orange),
            stroke_weight: Some(1.5),
            ..named("Rule")
        },
    );
    let footer = b.add(
        p,
        NewKind::Frame,
        [60.0, 558.0, 300.0, 10.0],
        auto(
            Direction::Horizontal,
            MainAlign::SpaceBetween,
            0.0,
            "Footer",
        ),
    );
    for line in [
        "Photograph AS08-14-2383 by William Anders, NASA",
        "Set with Satz",
    ] {
        let format = TextProps {
            size: Some(12.0),
            fill: Some(light.clone()),
            ..TextProps::default()
        };
        b.text(&footer, [60.0, 558.0, 0.0, 0.0], line, format);
    }
    b.set(&footer, hug(Size::Fixed));
    b.0
}

fn booklet() -> Doc {
    let (w, h) = (148.0, 210.0);
    let (mut b, first) = Build::new(w, h, true);
    let mut pages = vec![first];
    for _ in 1..8 {
        pages.push(b.one(Command::AddPage { after: None }));
    }
    let accent = b.swatch("Satz Orange", cmyk(0.0, 0.55, 1.0, 0.0, 1.0), true);
    let ids = b.run(Command::AddCollection {
        name: "Tone".into(),
    });
    let (tone, light) = (ids[0].clone(), ids[1].clone());
    b.run(Command::SetMode {
        collection: tone.clone(),
        id: light.clone(),
        name: "Light".into(),
    });
    let dark = b.one(Command::AddMode {
        collection: tone.clone(),
        name: "Dark".into(),
    });
    let mut variable = |name: &str, light: Color, dark_value: Color| {
        let id = b.one(Command::AddVariable {
            collection: tone.clone(),
            name: name.into(),
            value: Value::Color(light),
        });
        b.run(Command::SetVariable {
            id: id.clone(),
            name: None,
            mode: Some(dark.clone()),
            value: Some(Value::Color(dark_value)),
        });
        Color::Variable {
            variable: id,
            alpha: 1.0,
        }
    };
    let paper = variable("Paper", cmyk(0.0, 0.0, 0.0, 0.0, 1.0), photo_black(1.0));
    let ink = variable(
        "Ink",
        cmyk(0.0, 0.0, 0.0, 0.9, 1.0),
        cmyk(0.0, 0.0, 0.0, 0.0, 1.0),
    );
    let styles: Vec<String> = [("Body", 10.5, 15.0, 4.0), ("Heading", 16.0, 20.0, 5.0)]
        .into_iter()
        .map(|(name, size, line_height, paragraph_spacing)| {
            b.one(Command::AddTextStyle {
                name: name.into(),
                props: TextProps {
                    size: Some(size),
                    line_height: Some(line_height),
                    letter_spacing: Some(0.0),
                    paragraph_spacing: Some(paragraph_spacing),
                    ..TextProps::default()
                },
            })
        })
        .collect();
    let style = |i: usize| TextProps {
        text_style: Some(styles[i].clone()),
        fill: Some(ink.clone()),
        ..TextProps::default()
    };
    let small = |fill: &Color, align| TextProps {
        size: Some(8.0),
        letter_spacing: Some(1.0),
        fill: Some(fill.clone()),
        text_align: Some(align),
        ..TextProps::default()
    };

    let mut master = |name: &str| {
        let m = b.one(Command::AddMaster { like: None });
        b.run(Command::SetMaster {
            id: m.clone(),
            name: name.into(),
        });
        for x in [-w - 3.0, 0.0] {
            let paper = Props {
                fills: solid(paper.clone()),
                ..named("Paper")
            };
            b.add(&m, NewKind::Rect, [x, -3.0, w + 3.0, h + 6.0], paper);
        }
        m
    };
    let body = master("Body");
    let cover = master("Cover");
    for (x, align) in [(-w + 14.0, TextAlign::Left), (18.0, TextAlign::Right)] {
        b.text(
            &body,
            [x, 10.0, 116.0, 0.0],
            "EARTHRISE",
            small(&ink, align),
        );
        b.add(
            &body,
            NewKind::Line,
            [x, 16.0, 116.0, 0.0],
            Props {
                strokes: solid(accent.clone()),
                stroke_weight: Some(0.75),
                ..named("Rule")
            },
        );
        let number = PAGE_NUMBER.to_string();
        b.text(&body, [x, 196.0, 116.0, 0.0], &number, small(&ink, align));
    }
    for (i, p) in pages.iter().enumerate() {
        let inner = (1..6).contains(&i);
        b.run(Command::UseMaster {
            page: p.clone(),
            master: Some(if inner { &body } else { &cover }.clone()),
        });
        b.run(Command::UseMode {
            id: p.clone(),
            collection: tone.clone(),
            mode: Some(if inner { &light } else { &dark }.clone()),
        });
    }

    let p = &pages;
    let whole = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    b.photo(&p[0], [0.0, -3.0, w + 3.0, w + 3.0], whole);
    b.add(
        &p[0],
        NewKind::Rect,
        [0.0, 100.0, w + 3.0, 48.5],
        Props {
            fills: gradient(
                FillKind::Linear,
                [0.0, 1.0, -1.0, 0.0, 0.5, 0.0],
                vec![(0.0, photo_black(0.0)), (1.0, photo_black(1.0))],
            ),
            ..named("Fade")
        },
    );
    let title = TextProps {
        size: Some(48.0),
        fill: Some(ink.clone()),
        ..TextProps::default()
    };
    b.text(&p[0], [14.0, 160.0, 120.0, 0.0], "Earthrise", title);
    let subtitle = TextProps {
        size: Some(10.0),
        letter_spacing: Some(2.0),
        fill: Some(accent.clone()),
        ..TextProps::default()
    };
    b.text(
        &p[0],
        [14.0, 180.0, 120.0, 0.0],
        "APOLLO 8  ·  DECEMBER 1968",
        subtitle,
    );

    b.text(&p[1], [14.0, 30.0, 116.0, 0.0], "Contents", style(1));
    let list = b.add(
        &p[1],
        NewKind::Frame,
        [14.0, 45.0, 116.0, 10.0],
        auto(Direction::Vertical, MainAlign::Start, 3.0, "Contents"),
    );
    for (title, page) in CONTENTS {
        let row = b.add(
            &list,
            NewKind::Frame,
            [14.0, 45.0, 116.0, 6.0],
            auto(Direction::Horizontal, MainAlign::SpaceBetween, 0.0, title),
        );
        for text in [title.to_string(), page.to_string()] {
            b.text(&row, [14.0, 45.0, 0.0, 0.0], &text, style(0));
        }
        b.set(&row, hug(Size::Fill));
    }
    b.set(&list, hug(Size::Fixed));

    b.photo(
        &p[4],
        [0.0, -3.0, w + 3.0, 96.0],
        [1.656, 0.0, 0.0, 2.604, -0.427, -0.854],
    );
    let frames: Vec<String> = [(2, 18.0, 26.0), (3, 14.0, 26.0), (4, 18.0, 102.0)]
        .into_iter()
        .map(|(i, x, top)| {
            b.text(
                &p[i],
                [x, top, 116.0, 188.0 - top],
                "",
                TextProps::default(),
            )
        })
        .collect();
    b.run(Command::SetText {
        id: frames[0].clone(),
        text: STORY.into(),
    });
    let utf16 = |s: &str| s.encode_utf16().count();
    b.run(Command::Format {
        id: frames[0].clone(),
        range: Some([0, utf16(STORY)]),
        props: TextProps {
            text_align: Some(TextAlign::Justify),
            hyphenate: Some(true),
            ..style(0)
        },
    });
    for at in CONTENTS.map(|(title, _)| STORY.find(&format!("{title}\n"))) {
        let Some(at) = at else { continue };
        let title = &STORY[at..at + STORY[at..].find('\n').unwrap()];
        let at = utf16(&STORY[..at]);
        b.run(Command::Format {
            id: frames[0].clone(),
            range: Some([at, at + utf16(title)]),
            props: TextProps {
                text_align: Some(TextAlign::Left),
                ..style(1)
            },
        });
    }
    for f in frames.windows(2) {
        b.run(Command::Thread {
            from: f[0].clone(),
            to: f[1].clone(),
        });
    }

    b.text(&p[5], [14.0, 30.0, 116.0, 0.0], FIGURES, style(1));
    let table = b.add(
        &p[5],
        NewKind::Frame,
        [14.0, 45.0, 116.0, 10.0],
        auto(Direction::Vertical, MainAlign::Start, 3.0, "Figures"),
    );
    for (label, value) in [
        ("Launch", "21 December 1968"),
        ("Crew", "Borman, Lovell, Anders"),
        ("Orbits of the Moon", "10"),
        ("Time in lunar orbit", "20 hours"),
        ("Earthrise", "24 December 1968"),
        ("Splashdown", "27 December 1968"),
    ] {
        let row = b.add(
            &table,
            NewKind::Frame,
            [14.0, 45.0, 116.0, 8.0],
            auto(Direction::Horizontal, MainAlign::SpaceBetween, 0.0, label),
        );
        for text in [label, value] {
            b.text(&row, [14.0, 45.0, 0.0, 0.0], text, style(0));
        }
        b.set(&row, hug(Size::Fill));
    }
    b.set(&table, hug(Size::Fixed));

    b.photo(
        &p[6],
        [0.0, -3.0, w + 3.0, h + 6.0],
        [1.43, 0.0, 0.0, 1.0, -0.3, 0.0],
    );
    b.text(
        &p[6],
        [14.0, 14.0, 120.0, 0.0],
        "Earthrise, 24 December 1968. Photograph AS08-14-2383 by William Anders, NASA.",
        small(&ink, TextAlign::Left),
    );
    b.add(
        &p[7],
        NewKind::Ellipse,
        [14.0, 150.0, 10.0, 10.0],
        Props {
            fills: solid(accent),
            effects: Some(vec![Effect {
                y: 0.5 * MM as f32,
                radius: 2.0 * MM as f32,
                color: cmyk(0.0, 0.0, 0.0, 1.0, 0.6),
                ..Effect::default()
            }]),
            ..named("Mark")
        },
    );
    b.text(
        &p[7],
        [14.0, 170.0, 120.0, 0.0],
        "Set in Source Serif 4 with Satz. Printed in CMYK with one spot colour, Satz Orange.",
        style(0),
    );
    b.0
}

fn issues(d: &Doc) -> Vec<(String, Problem)> {
    d.build_snapshot()
        .preflight
        .into_iter()
        .map(|i| (i.name, i.problem))
        .collect()
}

fn write(name: &str, d: &Doc) {
    if std::env::var_os("SATZ_WRITE_EXAMPLES").is_some() {
        std::fs::write(format!("{DIR}/{name}"), d.save()).unwrap();
    }
}

#[test]
fn the_poster_is_an_a2_page_without_preflight_issues() {
    let d = poster();
    assert_eq!(issues(&d), []);
    let p = &d.snapshot().pages;
    assert_eq!(p.len(), 1);
    assert!((p[0].width / MM - 420.0).abs() < 0.01 && (p[0].height / MM - 594.0).abs() < 0.01);
    write("poster.satz", &d);
}

#[test]
fn the_booklet_is_eight_a5_pages_whose_contents_list_its_headings() {
    let d = booklet();
    assert_eq!(issues(&d), []);
    let pages = d.build_snapshot().pages;
    assert_eq!(pages.len(), 8);
    for (title, page) in CONTENTS {
        let text: Vec<String> = d
            .render(&pages[page - 1].id)
            .into_iter()
            .filter_map(|o| match o {
                Op::GlyphRun { text, size, .. } if size > 12.0 => Some(text),
                _ => None,
            })
            .collect();
        assert!(
            text.concat().contains(&title.replace(" ", "")),
            "{title} is not on page {page}: {text:?}"
        );
    }
    write("booklet.satz", &d);
}

#[test]
fn the_saved_examples_load_as_they_were_built() {
    for (name, pages) in [("poster.satz", 1), ("booklet.satz", 8)] {
        let d = Doc::load(&std::fs::read(format!("{DIR}/{name}")).unwrap()).unwrap();
        assert_eq!(d.snapshot().pages.len(), pages, "{name}");
        assert_eq!(issues(&d), [], "{name}");
    }
}
