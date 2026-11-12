use super::*;

/// The indent in pt of each level of a table of contents.
const LEVEL: f64 = 12.0;

impl Doc {
    /// Fills the story of `id` with the paragraphs of the pages in the text styles
    /// `styles`, one level per style, each followed by a right tab with a dot leader
    /// and the number of the page it starts on, and keeps `styles` for updating it.
    /// Empty `styles` keep the text and forget the table.
    pub(super) fn toc(&self, id: String, styles: Vec<String>) -> Res<Vec<String>> {
        let n = self.story(self.node(&id)?);
        let t = self.own_text(n)?;
        let m = self.meta(n);
        if styles.is_empty() {
            m.delete(TOC).map_err(err)?;
            return Ok(vec![]);
        }
        for s in &styles {
            self.find::<TextStyle>("textStyles", s)?;
        }
        let entries = self.entries(n, &styles);
        let tf = self.text_frame_of(n);
        let at = (self.bounds(n)[2] - tf.inset_left - tf.inset_right).max(0.0);
        let text: Vec<_> = entries
            .iter()
            .map(|(_, e, page)| format!("{e}\t{page}"))
            .collect();
        t.update(&text.join("\n"), UpdateOptions::default())
            .map_err(|e| format!("{e:?}"))?;
        m.insert(TOC, loro(&styles)?).map_err(err)?;
        let mut start = 0;
        for ((level, ..), line) in entries.iter().zip(&text) {
            let end = start + line.encode_utf16().count();
            self.format(
                n.to_string(),
                Some([start, end]),
                TextProps {
                    indent_left: Some(*level as f64 * LEVEL),
                    tabs: Some(vec![text::Tab {
                        at,
                        align: text::TabAlign::Right,
                        leader: ".".into(),
                    }]),
                    ..TextProps::default()
                },
            )?;
            start = end + 1;
        }
        Ok(vec![])
    }

    /// The paragraphs in `styles` of the stories other than `toc`, in the order of the
    /// pages and from the top of each, with their level and page number.
    fn entries(&self, toc: TreeID, styles: &[String]) -> Vec<(usize, String, String)> {
        let flows = self.flows();
        let mut out = Vec::new();
        for page in self.pages() {
            let mut frames = Vec::new();
            self.shown_texts(page, &mut frames);
            frames.retain(|f| flows.get(f).is_some_and(|f| f.head != toc));
            frames.sort_by(|a, b| {
                let ([ax, ay, ..], [bx, by, ..]) = (self.bounds(*a), self.bounds(*b));
                ay.total_cmp(&by).then(ax.total_cmp(&bx))
            });
            for f in frames {
                let f = &flows[&f];
                let s = &f.story;
                for (b, _) in std::iter::once((0, '\n'))
                    .chain(s.text.char_indices().map(|(i, c)| (i + 1, c)))
                    .filter(|&(_, c)| c == '\n')
                    .filter(|&(b, _)| (f.start..f.end).contains(&b))
                {
                    let style = &s.spans[span_at(&s.spans, utf16_of(&s.text, b))]
                        .attrs
                        .text_style;
                    let Some(level) = styles.iter().position(|x| x == style) else {
                        continue;
                    };
                    let para = s.text[b..].split('\n').next().unwrap_or_default();
                    let entry: String = para
                        .chars()
                        .filter(|&c| c != text::PAGE_NUMBER && c != text::DATE)
                        .map(|c| if c == '\t' { ' ' } else { c })
                        .collect();
                    if !entry.trim().is_empty() {
                        out.push((level, entry.trim().into(), self.number(page)));
                    }
                }
            }
        }
        out
    }

    /// The text layers under `n` that are not hidden.
    fn shown_texts(&self, n: TreeID, out: &mut Vec<TreeID>) {
        for c in self.children(n) {
            if value(&self.meta(c), "hidden") == Some(true.into()) {
                continue;
            }
            if self.kind(c) == Some(NodeKind::Text) {
                out.push(c);
            }
            self.shown_texts(c, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn a_table_of_contents_lists_the_styled_paragraphs_with_their_pages_and_updates() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let h1 = style(&mut d, "Heading", 18.0);
        let h2 = style(&mut d, "Sub", 14.0);
        let styled = |id: &str| TextProps {
            text_style: Some(id.into()),
            ..TextProps::default()
        };
        let toc = create(&mut d, &p1, NewKind::Text, [0.0, 0.0, 200.0, 100.0]);
        set_frame(&mut d, &toc, [0.0, 0.0, 200.0, 100.0]);
        let a = create(&mut d, &p1, NewKind::Text, [0.0, 200.0, 200.0, 100.0]);
        set_text(&mut d, &a, "Intro\nbody\nDetail");
        format(&mut d, &a, Some([0, 5]), styled(&h1)).unwrap();
        format(&mut d, &a, Some([11, 17]), styled(&h2)).unwrap();
        let b = create(&mut d, &p2, NewKind::Text, [0.0, 0.0, 200.0, 100.0]);
        set_text(&mut d, &b, "Second");
        format(&mut d, &b, None, styled(&h1)).unwrap();
        let run = |d: &mut Doc, styles: &[&String]| {
            d.apply(Command::Toc {
                id: toc.clone(),
                styles: styles.iter().map(|s| s.to_string()).collect(),
            })
        };
        run(&mut d, &[&h1, &h2]).unwrap();
        let toc_of = |d: &Doc| {
            let n = d.node(&toc).unwrap();
            (d.own_text(n).unwrap().to_string(), d.text_frame_of(n).toc)
        };
        assert_eq!(
            toc_of(&d),
            (
                "Intro\t1\nDetail\t1\nSecond\t2".into(),
                vec![h1.clone(), h2.clone()]
            )
        );
        let s = d.flows()[&d.node(&toc).unwrap()].story.spans.clone();
        let t = s.iter().find(|s| s.attrs.indent_left == LEVEL).unwrap();
        assert_eq!((t.len, t.attrs.tabs[0].at), (9, 200.0));

        d.apply(Command::MovePages {
            ids: vec![p2],
            index: 0,
        })
        .unwrap();
        run(&mut d, &[&h1]).unwrap();
        assert_eq!(toc_of(&d).0, "Second\t1\nIntro\t2");
        assert!(run(&mut d, &[&"x".to_string()]).is_err());
        run(&mut d, &[]).unwrap();
        assert_eq!(toc_of(&d), ("Second\t1\nIntro\t2".into(), vec![]));
    }
}
