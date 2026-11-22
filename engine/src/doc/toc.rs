use super::*;

const LEVEL: f64 = 12.0;

impl Doc {
    pub(super) fn toc(&self, id: String, styles: Vec<String>) -> Res<Vec<String>> {
        let n = self.story(self.node(&id)?);
        self.own_text(n)?;
        let m = self.meta(n);
        if styles.is_empty() {
            m.delete(TOC).map_err(err)?;
            return Ok(vec![]);
        }
        for s in &styles {
            self.find::<TextStyle>("textStyles", s)?;
        }
        m.insert(TOC, loro(&styles)?).map_err(err)?;
        let fit = Sizing {
            horizontal: Size::Fixed,
            vertical: Size::Hug,
        };
        if self.next_of(n).is_none() && self.layout(n).sizing != fit {
            m.insert(SIZING, loro(fit)?).map_err(err)?;
        }
        self.fill(n, &styles, true)?;
        Ok(vec![])
    }

    pub(super) fn refresh_tocs(&self) -> Res<bool> {
        let tocs: Vec<_> = (self.heads().into_iter())
            .map(|n| (n, self.text_frame_of(n).toc))
            .filter(|(_, s)| !s.is_empty())
            .collect();
        if tocs.is_empty() {
            return Ok(false);
        }
        self.flows.replace(Some(Rc::new(self.flow_all(None))));
        let mut changed = false;
        for (n, styles) in tocs {
            changed |= self.fill(n, &styles, false)?;
        }
        self.flows.take();
        Ok(changed)
    }

    fn fill(&self, n: TreeID, styles: &[String], force: bool) -> Res<bool> {
        let t = self.own_text(n)?;
        let entries = self.entries(n, styles);
        let tf = self.text_frame_of(n);
        let at = (self.bounds(n)[2] - tf.inset_left - tf.inset_right).max(0.0);
        let text: Vec<_> = entries
            .iter()
            .map(|(_, e, page)| format!("{e}\t{page}"))
            .collect();
        let joined = text.join("\n");
        let flows = self.flows();
        let kept = flows.get(&n).is_some_and(|f| {
            f.story.text == joined
                && (joined.is_empty()
                    || f.story
                        .spans
                        .iter()
                        .all(|s| s.attrs.tabs.first().is_some_and(|t| t.at == at)))
        });
        if kept && !force {
            return Ok(false);
        }
        t.update(&joined, UpdateOptions::default())
            .map_err(|e| err(format!("{e:?}")))?;
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
        Ok(true)
    }

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
                        .filter(|c| ![text::PAGE_NUMBER, text::DATE, text::ANCHOR].contains(c))
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
        let [.., w, h] = frames(&d, &toc)[0];
        assert!(w == 200.0 && close(h, 3.0 * LEADING), "{w} {h}");

        d.apply(Command::MovePages {
            ids: vec![p2],
            index: 0,
        })
        .unwrap();
        assert_eq!(toc_of(&d).0, "Second\t1\nIntro\t2\nDetail\t2");
        run(&mut d, &[&h1]).unwrap();
        assert_eq!(toc_of(&d).0, "Second\t1\nIntro\t2");
        set_text(&mut d, &b, "First");
        assert_eq!(toc_of(&d).0, "First\t1\nIntro\t2");
        d.apply(Command::Undo).unwrap();
        assert_eq!(toc_of(&d).0, "Second\t1\nIntro\t2");
        set_frame(&mut d, &toc, [0.0, 0.0, 150.0, h]);
        let s = d.flows()[&d.node(&toc).unwrap()].story.spans.clone();
        assert!(s.iter().all(|s| s.attrs.tabs[0].at == 150.0));
        assert_eq!(d.bounds(d.node(&toc).unwrap())[3], h);
        assert_eq!(
            run(&mut d, &[&"x".to_string()]).err(),
            Some(Error::NotFound {
                what: "textStyle",
                id: "x".into()
            })
        );
        run(&mut d, &[]).unwrap();
        assert_eq!(toc_of(&d), ("Second\t1\nIntro\t2".into(), vec![]));
    }
}
