use super::*;

#[test]
fn fixture_unicode_names() {
    let names: String = (0..=0x10FFFF)
        .filter_map(char::from_u32)
        .filter_map(|c| {
            let name = unicode_names2::name(c)?.to_string();
            let hex = format!("{:X}", c as u32);
            (!name.ends_with(&format!("-{hex}"))).then(|| format!("{hex} {name}\n"))
        })
        .collect();
    crate::fixture("unicode-names.txt", names.as_bytes());
}

fn drawn(
    text: &str,
    spans: &[Span],
    style: &Style,
    frame: [f32; 4],
    tf: &TextFrame,
    s: &Scope,
) -> Vec<Op> {
    draw(
        text,
        spans,
        style,
        frame,
        tf,
        s,
        &set(text, spans, frame, tf, 0).0,
    )
}
use crate::style::Fill;
use crate::variable::{Modes, Palette};

fn black() -> Style {
    Style {
        fills: vec![Fill::solid(0x000000ffu32)],
        ..Style::default()
    }
}

#[test]
fn a_font_without_the_tables_text_is_set_with_is_rejected() {
    for table in [b"hhea", b"head", b"cmap", b"hmtx"] {
        let mut bytes = FONT.to_vec();
        let at = bytes.windows(4).position(|w| w == table).unwrap();
        bytes[at] = b'x';
        assert!(
            add_font(&bytes).is_err(),
            "{}",
            str::from_utf8(table).unwrap()
        );
    }
}

#[test]
fn a_font_is_named_by_its_family_and_style() {
    let f = typeface(FONT).unwrap();
    assert_eq!(
        (f.family.as_str(), f.style.as_str()),
        ("Source Serif 4", "Regular")
    );
    let styles: Vec<_> = fonts().into_iter().map(|f| f.style).collect();
    assert_eq!(styles, ["Regular", "Bold", "Italic", "Bold Italic"]);
}

const H: u16 = 9;
const I: u16 = 36;
const HYPHEN: u16 = 502;
const ASCENT: f32 = 10.36;
const AUTO: f32 = 13.71;

fn attrs(size: f64) -> Attrs {
    Attrs {
        size,
        ..Attrs::default()
    }
}

fn one(text: &str, a: Attrs) -> Vec<Span> {
    vec![Span {
        len: text.encode_utf16().count(),
        attrs: a,
    }]
}

fn runs(text: &str, spans: &[Span], frame: [f32; 4]) -> Vec<(f32, Vec<u16>, Vec<f32>)> {
    framed(text, spans, frame, &TextFrame::default())
}

fn framed(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
) -> Vec<(f32, Vec<u16>, Vec<f32>)> {
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    drawn(text, spans, &black(), frame, tf, &s)
        .into_iter()
        .map(|op| match op {
            Op::GlyphRun {
                size,
                glyphs,
                positions,
                ..
            } => (size, glyphs, positions),
            _ => panic!("unexpected {op:?}"),
        })
        .collect()
}

fn plain(text: &str, a: Attrs, frame: [f32; 4]) -> Vec<(Vec<u16>, Vec<f32>)> {
    runs(text, &one(text, a), frame)
        .into_iter()
        .map(|(_, g, p)| (g, p))
        .collect()
}

fn assert_close(a: &[f32], b: &[f32]) {
    assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 0.01, "{a:?} vs {b:?}");
    }
}

#[test]
fn a_stroke_goes_around_the_glyphs_outside_of_them() {
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    let style = Style {
        strokes: vec![Fill::solid(0xff0000ffu32)],
        stroke_align: crate::style::Align::Outside,
        ..Style::default()
    };
    let frame = [0.0, 0.0, 100.0, 50.0];
    let ops = drawn(
        "Hi",
        &one("Hi", attrs(10.0)),
        &style,
        frame,
        &TextFrame::default(),
        &s,
    );
    let [
        Op::PushClip { path, invert: true },
        Op::StrokePath { width, .. },
        Op::PopClip,
    ] = &ops[..]
    else {
        panic!("{ops:?}");
    };
    assert_eq!(*width, 2.0);
    let [x, y, w, h] = crate::geom::bounds(path);
    assert!(
        x > 0.0 && x < 2.0 && w > 10.0 && y > 0.0 && y + h < ASCENT + 0.5,
        "{x} {y} {w} {h}"
    );
    assert!(
        drawn(
            "Hi",
            &one("Hi", attrs(10.0)),
            &black(),
            frame,
            &TextFrame::default(),
            &s,
        )
        .iter()
        .all(|op| matches!(op, Op::GlyphRun { .. }))
    );
}

#[test]
fn short_text_sits_on_the_first_baseline() {
    let r = plain("Hi", attrs(10.0), [5.0, 7.0, 100.0, 50.0]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].0, vec![H, I]);
    assert_close(&r[0].1, &[5.0, 7.0 + ASCENT, 12.88, 7.0 + ASCENT]);
}

#[test]
fn a_page_number_marker_sets_the_number_of_the_page_in_its_place() {
    let frame = [0.0, 0.0, 100.0, 50.0];
    let marked = format!("p{PAGE_NUMBER}.");
    let tf = TextFrame {
        number: "12".into(),
        ..TextFrame::default()
    };
    let a = attrs(10.0);
    let runs = framed(&marked, &one(&marked, a.clone()), frame, &tf);
    assert_eq!(runs, framed("p12.", &one("p12.", a.clone()), frame, &tf));
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    let ops = drawn(&marked, &one(&marked, a.clone()), &black(), frame, &tf, &s);
    assert!(matches!(&ops[0], Op::GlyphRun { text, .. } if text == "p12."));
    let lines = set(&marked, &one(&marked, a), frame, &tf, 0).0.geometry;
    let [before, after] = [1, 1 + PAGE_NUMBER.len_utf8()].map(|at| caret(&marked, &lines, at)[0]);
    assert!(after - before > 9.9, "{before} {after}");
}

#[test]
fn shaping_forms_the_fi_ligature() {
    assert_eq!(
        plain("fi", attrs(10.0), [0.0, 0.0, 100.0, 50.0])[0].0,
        vec![417]
    );
}

#[test]
fn justified_lines_but_the_last_fill_the_frame_width() {
    let a = Attrs {
        text_align: TextAlign::Justify,
        ..attrs(10.0)
    };
    let r = plain("Hi Hi Hi", a, [5.0, 7.0, 25.0, 50.0]);
    let b1 = 7.0 + ASCENT;
    let b2 = b1 + AUTO;
    assert_eq!(r.len(), 2);
    assert_close(&r[0].1, &[5.0, b1, 12.88, b1, 19.14, b1, 27.02, b1]);
    assert_close(&r[1].1, &[5.0, b2, 12.88, b2]);
}

#[test]
fn left_aligned_lines_keep_their_natural_spacing() {
    let r = plain("Hi Hi Hi", attrs(10.0), [5.0, 7.0, 25.0, 50.0]);
    assert_close(
        &r[0].1,
        &[5.0, 17.36, 12.88, 17.36, 18.19, 17.36, 26.07, 17.36],
    );
}

#[test]
fn center_and_right_move_the_line_into_the_free_space() {
    let at = |text_align| {
        plain(
            "Hi",
            Attrs {
                text_align,
                ..attrs(10.0)
            },
            [0.0, 0.0, 100.0, 50.0],
        )[0]
        .1[0]
    };
    assert_close(
        &[at(TextAlign::Center), at(TextAlign::Right)],
        &[44.57, 89.14],
    );
}

#[test]
fn lines_below_the_frame_are_not_drawn() {
    assert_eq!(
        plain("Hi Hi Hi", attrs(10.0), [5.0, 7.0, 25.0, 15.0]).len(),
        1
    );
}

#[test]
fn a_drop_cap_spans_its_lines_and_moves_them_in() {
    let a = Attrs {
        drop_lines: 3,
        ..attrs(10.0)
    };
    let text = "Haha hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi";
    let lines = lay_out(
        text,
        &one(text, a),
        [0.0, 0.0, 60.0, 200.0],
        &TextFrame::default(),
        0,
    );
    let cap = &lines[0][0];
    assert!(cap.size > 25.0, "{cap:?}");
    assert!((cap.y - lines[2][0].y).abs() < 0.01);
    let starts: Vec<f32> = lines
        .iter()
        .map(|l| l.iter().find(|g| g.size == 10.0).unwrap().x)
        .collect();
    assert!(
        starts[0] > 15.0 && (starts[0] - starts[2]).abs() < 0.01,
        "{starts:?}"
    );
    assert_eq!(starts[3], 0.0);
}

#[test]
fn kept_lines_move_on_to_the_next_column_together() {
    let tf = TextFrame {
        columns: 2,
        gutter: 10.0,
        ..TextFrame::default()
    };
    let text = "Hi\nhi hi hi";
    let columns = |keep_lines| {
        let a = Attrs {
            keep_lines,
            ..attrs(10.0)
        };
        lay_out(text, &one(text, a), [0.0, 0.0, 40.0, 42.0], &tf, 0)
            .iter()
            .map(|l| l[0].x > 0.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(columns(1), [false, false, false, true]);
    assert_eq!(columns(2), [false, true, true, true]);
}

#[test]
fn a_tab_goes_to_its_stop_and_a_leader_fills_the_gap() {
    let at = |text: &str, tabs: Vec<Tab>, byte: usize| {
        let a = Attrs {
            tabs,
            ..attrs(10.0)
        };
        let lines = lay_out(
            text,
            &one(text, a),
            [0.0, 0.0, 200.0, 50.0],
            &TextFrame::default(),
            0,
        );
        let g = lines[0].iter().find(|g| g.cluster == byte).unwrap();
        (g.x, lines[0].len())
    };
    let stop = |at, align, leader: &str| Tab {
        at,
        align,
        leader: leader.into(),
    };
    assert_eq!(at("a\tb", vec![], 2), (36.0, 2));
    assert_eq!(at("a\tb", vec![stop(50.0, TabAlign::Left, "")], 2).0, 50.0);
    let (end, _) = at("a\tb", vec![stop(100.0, TabAlign::Right, "")], 2);
    let (centre, _) = at("a\tb", vec![stop(100.0, TabAlign::Center, "")], 2);
    assert!(end < centre && centre < 100.0, "{end} {centre}");
    let (point, _) = at("a\t12.5", vec![stop(80.0, TabAlign::Decimal, "")], 4);
    assert_eq!(point, 80.0);
    let (_, glyphs) = at("a\tb", vec![stop(100.0, TabAlign::Left, ".")], 2);
    assert!(glyphs > 20, "{glyphs}");
}

#[test]
fn list_markers_hang_in_front_and_numbers_count_on() {
    let text = "a\nb b b b b b b b b b\nc";
    let set = |list| {
        let a = Attrs {
            list,
            ..attrs(10.0)
        };
        lay_out(
            text,
            &one(text, a),
            [0.0, 0.0, 60.0, 200.0],
            &TextFrame::default(),
            0,
        )
    };
    let lines = set(List::Number);
    let marker = |l: &[Glyph]| l.iter().filter(|g| g.x < 15.0).count();
    assert_eq!(
        lines.iter().map(|l| marker(l)).collect::<Vec<_>>(),
        [2, 2, 0, 2]
    );
    assert_eq!(lines[1].iter().find(|g| g.x >= 15.0).unwrap().x, 15.0);
    assert_eq!(lines[2][0].x, 15.0);
    assert_eq!(set(List::Bullet)[0].len(), 2);
}

#[test]
fn max_lines_cuts_the_text_with_an_ellipsis() {
    let text = "hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi hi";
    let tf = TextFrame {
        max_lines: 2,
        ..TextFrame::default()
    };
    let lines = lay_out(
        text,
        &one(text, attrs(10.0)),
        [0.0, 0.0, 40.0, 200.0],
        &tf,
        0,
    );
    assert_eq!(lines.len(), 2);
    let dots = lines[1].last().unwrap();
    assert!(dots.cluster == text.len() && dots.x < 40.0, "{dots:?}");
    let h = measure(text, &one(text, attrs(10.0)), &tf, Some(40.0), 0)[1];
    assert_close(&[h], &[27.42]);
}

#[test]
fn trim_puts_the_cap_height_at_the_top_and_the_last_baseline_at_the_bottom() {
    let tf = TextFrame {
        trim: true,
        ..TextFrame::default()
    };
    let b = baselines("Hi\nHi", [0.0, 0.0, 100.0, 50.0], tf.clone());
    let [_, h] = measure("Hi\nHi", &one("Hi\nHi", attrs(10.0)), &tf, Some(100.0), 0);
    assert_close(&[b[0][1], h], &[6.7, b[1][1]]);
}

#[test]
fn round_baselines_puts_each_baseline_on_a_whole_point_below_the_top() {
    let tf = TextFrame {
        round_baselines: true,
        ..TextFrame::default()
    };
    let b = baselines("Hi\nHi", [0.0, 0.3, 100.0, 50.0], tf);
    assert_close(&[b[0][1] - 0.3, b[1][1] - 0.3], &[10.0, 24.0]);
}

#[test]
fn the_bundled_font_lists_its_characters() {
    let (id, chars) = characters(None);
    assert_eq!(id, 0);
    assert!(chars.contains(&0xe9));
}

#[test]
fn auto_line_height_is_the_fonts_ascent_plus_descent() {
    let r = plain("Hi\nHi", attrs(10.0), [0.0, 0.0, 100.0, 50.0]);
    assert_close(&r[1].1, &[0.0, ASCENT + AUTO, 7.88, ASCENT + AUTO]);
}

#[test]
fn a_fixed_line_height_centres_the_glyphs_in_each_line() {
    let a = Attrs {
        line_height: 20.0,
        ..attrs(10.0)
    };
    let r = plain("Hi\nHi", a, [0.0, 0.0, 100.0, 50.0]);
    assert_close(&[r[0].1[1], r[1].1[1]], &[13.5, 33.5]);
}

#[test]
fn a_line_sits_its_own_line_height_below_the_baseline_before() {
    let lh = |len, line_height| Span {
        len,
        attrs: Attrs {
            line_height,
            ..attrs(10.0)
        },
    };
    let r = runs(
        "Hi\nHi",
        &[lh(3, 30.0), lh(2, 12.0)],
        [0.0, 0.0, 100.0, 50.0],
    );
    assert_close(&[r[1].2[1] - r[0].2[1]], &[12.0]);
}

#[test]
fn paragraph_spacing_follows_each_paragraph() {
    let a = Attrs {
        paragraph_spacing: 5.0,
        ..attrs(10.0)
    };
    let r = plain("Hi\nHi", a, [0.0, 0.0, 100.0, 50.0]);
    assert_close(&[r[1].1[1]], &[ASCENT + AUTO + 5.0]);
}

#[test]
fn letter_spacing_widens_every_glyph() {
    let a = Attrs {
        letter_spacing: 10.0,
        ..attrs(10.0)
    };
    let r = plain("Hi", a, [0.0, 0.0, 100.0, 50.0]);
    assert_close(&r[0].1, &[0.0, ASCENT, 8.88, ASCENT]);
}

#[test]
fn case_features_and_indent_change_the_glyphs_and_where_they_sit() {
    let with = |a: Attrs| plain("Hi fi", a, [0.0, 0.0, 100.0, 50.0])[0].clone();
    let upper = with(Attrs {
        text_case: TextCase::Upper,
        ..attrs(10.0)
    });
    assert_eq!(upper.0[0], H);
    assert_ne!(upper.0[1], I);
    let no_liga = with(Attrs {
        features: vec!["liga=0".into()],
        ..attrs(10.0)
    });
    assert_eq!(no_liga.0.len(), 4);
    assert_eq!(with(attrs(10.0)).0.len(), 3);
    let indented = with(Attrs {
        paragraph_indent: 20.0,
        ..attrs(10.0)
    });
    assert_close(&indented.1[..1], &[20.0]);
}

#[test]
fn a_shaded_paragraph_fills_its_column_behind_its_lines_and_rules_split_the_columns() {
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    let text = "Hi\nHo";
    let spans = [
        Span {
            len: 3,
            attrs: Attrs {
                shading: Some(Color::Rgb(0xff0000ff)),
                ..attrs(10.0)
            },
        },
        Span {
            len: 2,
            attrs: attrs(10.0),
        },
    ];
    let tf = TextFrame {
        columns: 2,
        gutter: 10.0,
        column_rule: Some(Rule {
            weight: 1.0,
            color: Color::Rgb(0xff),
        }),
        ..TextFrame::default()
    };
    let ops = drawn(text, &spans, &black(), [0.0, 0.0, 110.0, 50.0], &tf, &s);
    let fills: Vec<_> = ops
        .iter()
        .filter_map(|op| match op {
            Op::FillPath { path, .. } => Some(crate::geom::bounds(path)),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0][..3], [0.0, 0.0, 50.0]);
    let rules: Vec<_> = ops
        .iter()
        .filter_map(|op| match op {
            Op::StrokePath { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(rules, [vec![MOVE, 55.0, 0.0, LINE, 55.0, 50.0]]);
}

#[test]
fn underline_and_strikethrough_draw_a_line_under_and_through_the_text() {
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    let lines = |text_decoration| {
        let a = Attrs {
            text_decoration,
            ..attrs(10.0)
        };
        drawn(
            "Hi Hi",
            &one("Hi Hi", a),
            &black(),
            [0.0, 0.0, 100.0, 50.0],
            &TextFrame::default(),
            &s,
        )
        .into_iter()
        .filter_map(|op| match op {
            Op::FillPath { path, .. } => Some([path[1], path[2], path[4]]),
            _ => None,
        })
        .collect::<Vec<_>>()
    };
    assert!(lines(TextDecoration::None).is_empty());
    let [under] = lines(TextDecoration::Underline)[..] else {
        panic!()
    };
    let [through] = lines(TextDecoration::Strikethrough)[..] else {
        panic!()
    };
    assert!(
        under[1] > ASCENT && through[1] < ASCENT,
        "{under:?} {through:?}"
    );
    assert!(under[2] > 20.0, "{under:?}");
}

#[test]
fn superscript_and_subscript_set_smaller_above_and_below_and_a_shift_raises() {
    let with = |position, baseline_shift| {
        let spans = [
            Span {
                len: 1,
                attrs: attrs(20.0),
            },
            Span {
                len: 1,
                attrs: Attrs {
                    position,
                    baseline_shift,
                    ..attrs(20.0)
                },
            },
        ];
        let glyphs: Vec<(f32, f32)> = runs("H2", &spans, [0.0, 0.0, 100.0, 50.0])
            .iter()
            .flat_map(|(size, _, at)| at.chunks(2).map(|p| (*size, p[1])))
            .collect();
        (glyphs[1].0, glyphs[1].1 - glyphs[0].1)
    };
    assert_eq!(with(Position::Normal, 0.0), (20.0, 0.0));
    let (size, dy) = with(Position::Normal, 3.0);
    assert_eq!(size, 20.0);
    assert_close(&[dy], &[-3.0]);
    let (size, dy) = with(Position::Superscript, 0.0);
    assert_close(&[size, dy], &[11.66, -6.66]);
    let (size, dy) = with(Position::Subscript, 1.0);
    assert_close(&[size, dy], &[11.66, 5.66]);
}

#[test]
fn a_bigger_span_draws_its_own_run_and_lowers_the_baseline() {
    let spans = [
        Span {
            len: 1,
            attrs: attrs(20.0),
        },
        Span {
            len: 1,
            attrs: attrs(10.0),
        },
    ];
    let r = runs("Hi", &spans, [0.0, 0.0, 100.0, 50.0]);
    assert_eq!(r.len(), 2);
    assert_eq!((r[0].0, r[1].0), (20.0, 10.0));
    assert_close(&r[0].2, &[0.0, 20.72]);
    assert_close(&r[1].2, &[15.76, 20.72]);
}

#[test]
fn hyphenation_breaks_a_long_word_at_a_syllable_and_sets_a_hyphen() {
    let de = Attrs {
        hyphenate: true,
        lang: Lang::De,
        ..attrs(10.0)
    };
    let r = plain("Silbentrennung", de, [0.0, 0.0, 45.0, 50.0]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].0.last(), Some(&HYPHEN));
    let off = plain("Silbentrennung", attrs(10.0), [0.0, 0.0, 45.0, 50.0]);
    assert_eq!(off.len(), 1);
    assert!(!off[0].0.contains(&HYPHEN));
}

#[test]
fn english_hyphenates_other_syllables_than_german() {
    let lang = |lang| Attrs {
        hyphenate: true,
        lang,
        ..attrs(10.0)
    };
    let frame = [0.0, 0.0, 38.0, 50.0];
    let en = plain("hyphenation", lang(Lang::En), frame);
    let de = plain("hyphenation", lang(Lang::De), frame);
    assert_eq!(en[0].0.last(), Some(&HYPHEN));
    assert_ne!(en[0].0.len(), de[0].0.len());
}

#[test]
fn french_italian_spanish_and_dutch_hyphenate() {
    for (word, lang) in [
        ("constitutionnellement", Lang::Fr),
        ("precipitevolissimevolmente", Lang::It),
        ("electroencefalografista", Lang::Es),
        ("ziekenhuisopname", Lang::Nl),
    ] {
        assert!(syllables(word, lang).len() > 2, "{word}");
    }
}

#[test]
fn a_word_longer_than_45_bytes_hyphenates() {
    let word = "Donaudampfschifffahrtsgesellschaftskapitänswitwenrentenkasse";
    assert!(word.len() > 45);
    assert!(syllables(word, Lang::De).len() > 10);
}

fn baselines(text: &str, frame: [f32; 4], tf: TextFrame) -> Vec<[f32; 2]> {
    framed(text, &one(text, attrs(10.0)), frame, &tf)
        .iter()
        .map(|r| [r.2[0], r.2[1]])
        .collect()
}

#[test]
fn insets_move_the_text_in_from_the_frame_edges() {
    let tf = TextFrame {
        inset_top: 3.0,
        inset_left: 5.0,
        inset_right: 70.0,
        ..TextFrame::default()
    };
    let b = baselines("Hi Hi Hi", [0.0, 0.0, 100.0, 50.0], tf);
    assert_eq!(b.len(), 2);
    assert_close(&b[0], &[5.0, 3.0 + ASCENT]);
}

#[test]
fn paragraph_indents_and_space_before_move_its_lines() {
    let a = Attrs {
        indent_left: 5.0,
        indent_right: 70.0,
        space_before: 7.0,
        ..attrs(10.0)
    };
    let text = "Hi Hi Hi\nHi";
    let b: Vec<[f32; 2]> = framed(
        text,
        &one(text, a),
        [0.0, 0.0, 100.0, 80.0],
        &TextFrame::default(),
    )
    .iter()
    .map(|r| [r.2[0], r.2[1]])
    .collect();
    assert_eq!(b.len(), 3);
    assert_close(&b[0], &[5.0, ASCENT]);
    assert_close(&b[2], &[5.0, ASCENT + 2.0 * AUTO + 7.0]);
}

#[test]
fn lines_beside_a_wrapped_area_run_in_the_room_it_leaves() {
    let tf = TextFrame {
        wrap: vec![([0.0, 0.0, 40.0, 2.0 * AUTO - 1.0], false)],
        ..TextFrame::default()
    };
    let b = baselines(&"Hi ".repeat(20), [0.0, 0.0, 100.0, 100.0], tf);
    assert_close(&[b[0][0], b[1][0], b[2][0]], &[40.0, 40.0, 0.0]);
    assert_close(&[b[0][1]], &[ASCENT]);
}

#[test]
fn lines_move_below_an_area_they_jump_or_that_leaves_too_little_room() {
    for (area, jump) in [
        ([30.0, 0.0, 60.0, 20.0], true),
        ([10.0, 0.0, 100.0, 20.0], false),
    ] {
        let tf = TextFrame {
            wrap: vec![(area, jump)],
            ..TextFrame::default()
        };
        let b = baselines("Hi Hi", [0.0, 0.0, 100.0, 100.0], tf);
        assert_close(&b[0], &[0.0, 20.0 + ASCENT]);
    }
}

#[test]
fn text_flows_into_the_next_column_when_one_is_full() {
    let tf = TextFrame {
        columns: 2,
        gutter: 10.0,
        ..TextFrame::default()
    };
    let b = baselines("Hi\nHi\nHi", [0.0, 0.0, 100.0, 30.0], tf);
    assert_eq!(b.len(), 3);
    assert_close(&b[1], &[0.0, ASCENT + AUTO]);
    assert_close(&b[2], &[55.0, ASCENT]);
}

#[test]
fn vertical_alignment_moves_the_lines_down_in_the_frame() {
    let at = |vertical_align| {
        let tf = TextFrame {
            vertical_align,
            ..TextFrame::default()
        };
        baselines("Hi", [0.0, 0.0, 100.0, 50.0], tf)[0][1]
    };
    assert_close(
        &[at(VerticalAlign::Center), at(VerticalAlign::Bottom)],
        &[28.5, 46.65],
    );
}

#[test]
fn a_baseline_grid_moves_each_baseline_down_to_the_next_grid_line() {
    let tf = TextFrame {
        inset_top: 2.0,
        baseline_grid: 15.0,
        baseline_start: 1.0,
        ..TextFrame::default()
    };
    let b = baselines("Hi\nHi", [0.0, 0.0, 100.0, 50.0], tf);
    assert_close(&[b[0][1], b[1][1]], &[18.0, 33.0]);
}

#[test]
fn measure_fits_the_frame_around_the_text_with_its_insets() {
    let tf = TextFrame {
        inset_top: 2.0,
        inset_right: 5.0,
        inset_bottom: 3.0,
        inset_left: 4.0,
        ..TextFrame::default()
    };
    let t = "Hi\nHi Hi";
    let spans = one(t, attrs(10.0));
    let [w, h] = measure(t, &spans, &tf, None, 0);
    assert_close(&[h], &[5.0 + 2.0 * AUTO]);
    assert_eq!(lay_out(t, &spans, [0.0, 0.0, w, h], &tf, 0).len(), 2);
    assert_eq!(
        lay_out(t, &spans, [0.0, 0.0, w - 1.0, 99.0], &tf, 0).len(),
        3
    );
    let [w, h] = measure(t, &spans, &tf, Some(25.0), 0);
    assert_close(&[w, h], &[25.0, 5.0 + 3.0 * AUTO]);
}

#[test]
fn measure_keeps_a_trailing_space_on_the_line() {
    let tf = TextFrame::default();
    let t = "Hello ";
    let spans = one(t, attrs(10.0));
    let [w, h] = measure(t, &spans, &tf, None, 0);
    let [bare, _] = measure("Hello", &one("Hello", attrs(10.0)), &tf, None, 0);
    assert!(w > bare, "{w} {bare}");
    assert_close(&[h], &[AUTO]);
    assert_eq!(lay_out(t, &spans, [0.0, 0.0, w, h], &tf, 0).len(), 1);
}

#[test]
fn measure_balances_the_height_over_the_columns() {
    let tf = TextFrame {
        columns: 2,
        ..TextFrame::default()
    };
    let t = "Hi\nHi\nHi\nHi";
    let [_, h] = measure(t, &one(t, attrs(10.0)), &tf, Some(100.0), 0);
    assert!((h - 2.0 * AUTO).abs() < 0.05, "{h}");
}

#[test]
fn carets_sit_where_the_characters_start_and_wrap_with_the_lines() {
    let t = "Hi Hi Hi";
    let frame = [5.0, 7.0, 25.0, 50.0];
    let ls = set(t, &one(t, attrs(10.0)), frame, &TextFrame::default(), 0)
        .0
        .geometry;
    assert_eq!(ls.len(), 2);
    assert_eq!(
        (ls[0].start, ls[0].end, ls[1].start, ls[1].end),
        (0, 5, 6, 8)
    );
    assert_close(&caret(t, &ls, 0), &[5.0, 7.0, 7.0 + AUTO]);
    assert_close(&caret(t, &ls, 1)[..1], &[12.88]);
    assert_close(&caret(t, &ls, 6), &[5.0, 7.0 + AUTO, 7.0 + 2.0 * AUTO]);
    let [end, ..] = caret(t, &ls, 8);
    assert!(end > 12.88 && end < 20.0, "{end}");
    assert_close(&caret(t, &ls, 5)[1..], &[7.0, 7.0 + AUTO]);
}

#[test]
fn the_caret_at_the_end_of_a_justified_paragraph_follows_its_last_word() {
    let t = "Hi Hi Hi";
    let a = Attrs {
        text_align: TextAlign::Justify,
        ..attrs(10.0)
    };
    let ls = set(
        t,
        &one(t, a),
        [5.0, 7.0, 25.0, 50.0],
        &TextFrame::default(),
        0,
    )
    .0
    .geometry;
    let [end, ..] = caret(t, &ls, 8);
    assert!(end > 12.88 && end < 20.0, "{end}");
}

#[test]
fn a_point_finds_the_nearest_character_boundary_on_its_line() {
    let t = "Hi Hi Hi";
    let ls = set(
        t,
        &one(t, attrs(10.0)),
        [5.0, 7.0, 25.0, 50.0],
        &TextFrame::default(),
        0,
    )
    .0
    .geometry;
    assert_eq!(index_at(&ls, 12.0, 10.0), 1);
    assert_eq!(index_at(&ls, 0.0, 0.0), 0);
    assert_eq!(index_at(&ls, 100.0, 25.0), 8);
    assert_eq!(index_at(&ls, 6.0, 100.0), 6);
}

fn texts(text: &str, a: Attrs, frame: [f32; 4]) -> Vec<String> {
    let palette = Palette::default();
    let modes = Modes::new();
    let s = Scope {
        palette: &palette,
        modes: &modes,
    };
    let spans = one(text, a);
    drawn(text, &spans, &black(), frame, &TextFrame::default(), &s)
        .into_iter()
        .map(|op| match op {
            Op::GlyphRun { text, .. } => text,
            _ => panic!("unexpected {op:?}"),
        })
        .collect()
}

#[test]
fn each_run_carries_the_text_of_its_glyphs_with_ligatures_and_hyphens() {
    assert_eq!(
        texts("Hi fine Hi", attrs(10.0), [0.0, 0.0, 30.0, 50.0]),
        ["Hifine", "Hi"]
    );
    let de = Attrs {
        hyphenate: true,
        lang: Lang::De,
        ..attrs(10.0)
    };
    assert_eq!(
        texts("Silbentrennung", de, [0.0, 0.0, 45.0, 50.0]),
        ["Silben-", "trennung"]
    );
}

#[test]
fn a_narrow_column_spreads_its_looseness_instead_of_one_very_loose_line() {
    let t = "Satz sets type in the browser. The engine shapes this paragraph with \
             harfrust, breaks it into lines with the Knuth-Plass algorithm and \
             justifies every line but the last to the width of its frame.";
    let a = Attrs {
        letter_spacing: 4.0,
        text_align: TextAlign::Justify,
        hyphenate: true,
        lang: Lang::De,
        ..attrs(13.0)
    };
    let lines = lay_out(
        t,
        &one(t, a),
        [0.0, 0.0, 150.0, 1000.0],
        &TextFrame::default(),
        0,
    );
    let counts: Vec<usize> = lines.iter().map(Vec::len).collect();
    let shortest = counts[..counts.len() - 1].iter().min().unwrap();
    assert!(*shortest >= 14, "{counts:?}");
}

#[test]
fn text_left_over_by_a_frame_is_set_from_where_it_stops_in_the_next() {
    let t = "Hi Hi Hi";
    let spans = one(t, attrs(10.0));
    let tf = TextFrame::default();
    assert_eq!(set(t, &spans, [0.0, 0.0, 25.0, 15.0], &tf, 0).1, Some(6));
    assert_eq!(set(t, &spans, [0.0, 0.0, 25.0, 50.0], &tf, 0).1, None);
    assert_eq!(set(t, &spans, [0.0, 0.0, 25.0, 50.0], &tf, 6).1, None);
    let rest = lay_out(t, &spans, [100.0, 0.0, 100.0, 50.0], &tf, 6);
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].iter().map(|g| g.id).collect::<Vec<_>>(), [H, I]);
    assert_close(&[rest[0][0].x, rest[0][0].y], &[100.0, ASCENT]);
    let ls = set(t, &spans, [100.0, 0.0, 100.0, 50.0], &tf, 6).0.geometry;
    assert_eq!((ls[0].start, ls[0].end), (6, 8));
    assert_close(&measure(t, &spans, &tf, Some(25.0), 6), &[25.0, AUTO]);
    let de = Attrs {
        hyphenate: true,
        lang: Lang::De,
        ..attrs(10.0)
    };
    let w = "Silbentrennung";
    let spans = one(w, de);
    let at = set(w, &spans, [0.0, 0.0, 45.0, 15.0], &tf, 0).1.unwrap();
    assert_eq!(&w[at..], "trennung");
    let rest = lay_out(w, &spans, [0.0, 0.0, 100.0, 50.0], &tf, at);
    assert_eq!(rest.len(), 1);
    assert!(!rest[0].iter().any(|g| g.hyphen));
    assert_eq!(rest[0].len(), "trennung".len());
}

#[test]
fn a_frame_that_starts_at_a_later_paragraph_skips_the_ones_before() {
    let t = "Hi\nHi Hi";
    let spans = one(t, attrs(10.0));
    let tf = TextFrame::default();
    assert_eq!(set(t, &spans, [0.0, 0.0, 100.0, 15.0], &tf, 0).1, Some(3));
    let rest = lay_out(t, &spans, [0.0, 0.0, 100.0, 50.0], &tf, 3);
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].len(), 4);
}
