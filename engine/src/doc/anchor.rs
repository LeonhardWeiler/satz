use super::*;

/// The mark on an object replacement character that names the layer it stands for.
pub(super) const OBJECT: &str = "object";
/// How an anchored layer follows its character, and where that was when it last moved.
pub(super) const ANCHORED: &str = "anchored";
const ANCHOR_AT: &str = "anchorAt";

/// How an anchored layer follows its character: in the line, which makes room for
/// it, or at its own place, moving as the character moves.
#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Anchoring {
    Inline,
    Custom,
}

impl Doc {
    /// Anchors the layer `id` inline before the UTF-16 index `at` of the story of the
    /// text layer `text`, and moves it just above that layer.
    pub(super) fn anchor(&self, id: String, text: String, at: usize) -> Res<Vec<String>> {
        let (o, f) = (self.layer(&id)?, self.node(&text)?);
        let t = self.text(f)?;
        if at > t.len_utf16() {
            return Err("index outside the text".into());
        }
        if value(&self.meta(o), ANCHORED).is_some() {
            return Err(format!("{id} is anchored"));
        }
        let mut under = Vec::new();
        self.walk(o, &mut under);
        let head = self.story(f);
        if under.iter().any(|&n| self.story(n) == head) {
            return Err("a layer cannot be anchored in its own text".into());
        }
        t.insert_utf16(at, &text::ANCHOR.to_string()).map_err(err)?;
        t.mark_utf16(at..at + 1, OBJECT, id).map_err(err)?;
        let m = self.meta(o);
        m.insert(ANCHORED, "inline").map_err(err)?;
        m.insert("absolute", true).map_err(err)?;
        self.tree.mov_after(o, f).map_err(err)?;
        Ok(vec![])
    }

    /// Makes the anchored layer `id` free again and removes its character.
    pub(super) fn unanchor(&self, id: String) -> Res<Vec<String>> {
        let o = self.layer(&id)?;
        let m = self.meta(o);
        if value(&m, ANCHORED).is_none() {
            return Err(format!("{id} is not anchored"));
        }
        for head in self.heads() {
            let t = self.own_text(head)?;
            for (i, _) in anchors(&t).into_iter().rev().filter(|a| a.1 == id) {
                t.delete_utf16(i, 1).map_err(err)?;
            }
        }
        m.delete(ANCHORED).map_err(err)?;
        m.delete(ANCHOR_AT).map_err(err)?;
        Ok(vec![])
    }

    /// Makes the characters in the texts of the copies `pasted` stand for the copies
    /// `copies` of their layers and drops those of layers not copied; copies that no
    /// character stands for are free.
    pub(super) fn reanchor(
        &self,
        pasted: impl Iterator<Item = TreeID> + Clone,
        copies: &HashMap<String, TreeID>,
    ) -> Res<()> {
        let mut named = HashSet::new();
        for n in pasted.clone() {
            let Ok(t) = self.own_text(n) else { continue };
            for (i, id) in anchors(&t).into_iter().rev() {
                match copies.get(&id) {
                    Some(&c) => {
                        t.mark_utf16(i..i + 1, OBJECT, c.to_string()).map_err(err)?;
                        named.insert(c);
                    }
                    None => t.delete_utf16(i, 1).map_err(err)?,
                }
            }
        }
        for n in pasted.filter(|n| !named.contains(n)) {
            let m = self.meta(n);
            if value(&m, ANCHORED).is_some() {
                m.delete(ANCHORED).map_err(err)?;
                m.delete(ANCHOR_AT).map_err(err)?;
            }
        }
        Ok(())
    }

    /// The width and height that the layer `id` takes in a line when it is anchored
    /// inline.
    pub(super) fn object_size(&self, id: &str) -> [f64; 2] {
        let Ok(o) = self.node(id) else {
            return [0.0; 2];
        };
        match field::<Anchoring>(&self.meta(o).get_deep_value(), ANCHORED) {
            Some(Anchoring::Inline) => {
                let [_, _, w, h] = self.bounds(o);
                [w, h]
            }
            _ => [0.0; 2],
        }
    }

    /// Moves the layers anchored in the stories `heads` with their characters, onto
    /// the page of the frame that sets each: inline ones onto the baseline, others
    /// by as much as their character moved. Whether any layer moved.
    pub(super) fn place_anchored(&self, heads: &HashSet<TreeID>) -> Res<bool> {
        let mut moved = false;
        for &head in heads {
            if !self
                .own_text(head)
                .is_ok_and(|t| t.to_string().contains(text::ANCHOR))
            {
                continue;
            }
            let Ok((story, frames)) = self.flow(head) else {
                continue;
            };
            for (f, start, _, placed) in &frames {
                for g in placed.lines.iter().flatten() {
                    let b = start + g.cluster;
                    if !story.text[b..].starts_with(text::ANCHOR) {
                        continue;
                    }
                    let a = &story.spans[span_at(&story.spans, utf16_of(&story.text, b))].attrs;
                    let Ok(o) = self.node(&a.object) else {
                        continue;
                    };
                    let v = self.meta(o).get_deep_value();
                    let Some(how) = field::<Anchoring>(&v, ANCHORED) else {
                        continue;
                    };
                    if self.root(o) != self.root(*f) {
                        self.tree.mov_after(o, *f).map_err(err)?;
                        moved = true;
                    }
                    let to = [g.x, g.y].map(f64::from);
                    let was = field::<[f64; 2]>(&v, ANCHOR_AT).unwrap_or(to);
                    let [x, y, w, h] = self.bounds(o);
                    let [nx, ny] = match how {
                        Anchoring::Inline => [to[0], to[1] - h],
                        Anchoring::Custom => [x + to[0] - was[0], y + to[1] - was[1]],
                    };
                    if [nx, ny] != [x, y] {
                        self.set_frame(o, [nx, ny, w, h])?;
                        moved = true;
                    }
                    if field::<[f64; 2]>(&v, ANCHOR_AT) != Some(to) {
                        self.meta(o).insert(ANCHOR_AT, loro(to)?).map_err(err)?;
                    }
                }
            }
        }
        Ok(moved)
    }
}

/// The UTF-16 index of each object replacement character in `t` that names a layer,
/// with that layer's id.
fn anchors(t: &LoroText) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    for d in t.to_delta() {
        let TextDelta::Insert { insert, attributes } = d else {
            continue;
        };
        let named = attributes.and_then(|a| a.get(OBJECT)?.as_string().map(|s| s.to_string()));
        for (i, c) in insert.encode_utf16().enumerate() {
            if let Some(id) = named.as_ref().filter(|_| c == text::ANCHOR as u16) {
                out.push((at + i, id.clone()));
            }
        }
        at += insert.encode_utf16().count();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    fn anchor(d: &mut Doc, id: &str, text: &str, at: usize) -> Res<Vec<String>> {
        d.apply(Command::Anchor {
            id: id.into(),
            text: text.into(),
            at,
        })
    }

    fn xy(d: &Doc, id: &str) -> [f64; 2] {
        let [x, y, ..] = d.bounds(d.node(id).unwrap());
        [x, y]
    }

    /// The x of each glyph of the first line of `id` and its baseline.
    fn line(d: &Doc, id: &str) -> (Vec<f32>, f32) {
        let flows = d.flows();
        let l = &flows[&d.node(id).unwrap()].placed.lines[0];
        (l.iter().map(|g| g.x).collect(), l[0].y)
    }

    #[test]
    fn an_inline_layer_sits_on_the_baseline_and_takes_its_width_in_the_line() {
        let (mut d, p) = empty();
        let t = fixed_text(&mut d, &p, [0.0, 0.0, 300.0, 300.0]);
        set_text(&mut d, &t, "ab cd");
        let (plain, base) = line(&d, &t);
        let r = create(&mut d, &p, NewKind::Rect, [200.0, 200.0, 20.0, 40.0]);
        anchor(&mut d, &r, &t, 2).unwrap();
        assert_eq!(
            d.own_text(d.node(&t).unwrap()).unwrap().to_string(),
            "ab\u{FFFC} cd"
        );
        let (xs, low) = line(&d, &t);
        assert_eq!(xs.len(), plain.len() + 1);
        assert!(close(xs[3] as f64 - plain[2] as f64, 20.0));
        assert!(low > base, "the line grows to hold the layer");
        assert_eq!(xy(&d, &r), [xs[2] as f64, low as f64 - 40.0]);
        assert_eq!(d.snapshot().pages[0].children.last().unwrap().id, r);

        edit(&mut d, &t, [0, 0], "xx").unwrap();
        assert!(xy(&d, &r)[0] > xs[2] as f64);
        let moved = xy(&d, &r);
        d.apply(Command::Undo).unwrap();
        assert_eq!(xy(&d, &r), [xs[2] as f64, low as f64 - 40.0]);
        d.apply(Command::Redo).unwrap();
        assert_eq!(xy(&d, &r), moved);
        set_frame(&mut d, &r, [0.0, 0.0, 50.0, 40.0]);
        let (xs, _) = line(&d, &t);
        assert_eq!(xy(&d, &r), [xs[4] as f64, low as f64 - 40.0]);
        assert!((50.0..55.0).contains(&(xs[5] - xs[4])), "{xs:?}");
        assert!(anchor(&mut d, &r, &t, 0).is_err());
        assert!(anchor(&mut d, &t, &t, 0).is_err());

        d.apply(Command::Undo).unwrap();
        d.apply(Command::Unanchor { id: r.clone() }).unwrap();
        assert_eq!(
            d.own_text(d.node(&t).unwrap()).unwrap().to_string(),
            "xxab cd"
        );
        assert_eq!(
            field::<Anchoring>(&d.meta(d.node(&r).unwrap()).get_deep_value(), ANCHORED),
            None
        );
        assert!(d.apply(Command::Unanchor { id: r }).is_err());
    }

    #[test]
    fn a_custom_layer_keeps_its_offset_and_follows_its_character_onto_the_next_page() {
        let (mut d, p1, p2, a, b) = two_pages();
        let r = create(&mut d, &p1, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        anchor(&mut d, &r, &a, 0).unwrap();
        set(
            &mut d,
            &r,
            Props {
                anchored: Some(Anchoring::Custom),
                ..Props::default()
            },
        );
        set_frame(&mut d, &r, [150.0, 5.0, 10.0, 10.0]);
        assert_eq!(xy(&d, &r), [150.0, 5.0]);
        edit(&mut d, &a, [0, 0], "\n\n\n\n").unwrap();
        let [x, y] = xy(&d, &r);
        let n = d.node(&r).unwrap();
        assert_eq!(d.root(n), d.node(&p2).unwrap());
        assert_eq!(d.tree.parent(n), d.tree.parent(d.node(&b).unwrap()));
        assert!(close(x, 160.0), "{x}");
        assert!(y > 10.0);
        let inline = Props {
            anchored: Some(Anchoring::Inline),
            ..Props::default()
        };
        let set = Command::Set {
            id: b,
            props: inline,
        };
        assert!(
            d.apply(set).is_err(),
            "only an anchored layer is anchored inline"
        );
    }

    #[test]
    fn copies_of_a_text_anchor_the_copies_of_its_layers_and_drop_the_others() {
        let (mut d, p) = empty();
        let t = fixed_text(&mut d, &p, [0.0, 0.0, 300.0, 300.0]);
        set_text(&mut d, &t, "ab");
        let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        anchor(&mut d, &r, &t, 1).unwrap();
        let dup = |d: &mut Doc, ids: &[&String]| {
            d.apply(Command::Duplicate {
                ids: ids.iter().map(|s| s.to_string()).collect(),
            })
            .unwrap()
        };
        let copies = dup(&mut d, &[&t, &r]);
        let text = d.own_text(d.node(&copies[0]).unwrap()).unwrap();
        assert_eq!(anchors(&text), [(1, copies[1].clone())]);
        let alone = dup(&mut d, &[&t]).remove(0);
        assert_eq!(
            d.own_text(d.node(&alone).unwrap()).unwrap().to_string(),
            "ab"
        );
        let free = dup(&mut d, &[&r]).remove(0);
        let v = d.meta(d.node(&free).unwrap()).get_deep_value();
        assert_eq!(field::<Anchoring>(&v, ANCHORED), None);
    }
}
