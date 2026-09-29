use super::*;

pub(super) struct Clip {
    /// The copied layer, so that copies of threaded frames thread among themselves.
    pub(super) id: TreeID,
    pub(super) meta: LoroValue,
    pub(super) text: Vec<TextDelta>,
    pub(super) children: Vec<Clip>,
}

impl Doc {
    pub(super) fn clip(&self, id: TreeID) -> Clip {
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

    pub(super) fn paste(&self, clip: &Clip, parent: TreeParentId, index: usize) -> Res<TreeID> {
        let new = self.tree.create_at(parent, index).map_err(err)?;
        let to = self.meta(new);
        for (k, v) in clip.meta.clone().into_map().unwrap().iter() {
            match (k.as_str(), v) {
                (NEXT, _) => {}
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
    pub(super) fn rethread<C: std::borrow::Borrow<Clip>>(&self, pasted: &[(C, TreeID)]) -> Res<()> {
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
                self.set_next(new, Some(to))?;
                let own = self.own_text(to)?;
                own.delete_utf16(0, own.len_utf16()).map_err(err)?;
            }
        }
        Ok(())
    }

    /// Each layer copied into `clip` with its copy under `copy` and the layer it threads to.
    pub(super) fn pair_up(
        &self,
        clip: &Clip,
        copy: TreeID,
        out: &mut Vec<(TreeID, TreeID, Option<String>)>,
    ) {
        let next = clip
            .meta
            .as_map()
            .and_then(|m| m.get(NEXT)?.as_string().cloned());
        out.push((clip.id, copy, next.map(|n| n.to_string())));
        for (c, n) in clip.children.iter().zip(self.children(copy)) {
            self.pair_up(c, n, out);
        }
    }

    pub(super) fn duplicate(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let mut out = Vec::new();
        let mut clips = Vec::new();
        for id in self.sorted(&ids)?.into_iter().rev() {
            let parent = self.tree.parent(id).ok_or("no parent")?;
            let clip = self.clip(id);
            let copy = self.paste(&clip, parent, self.index(id) + 1)?;
            self.meta(copy).delete(OVERRIDE_OF).map_err(err)?;
            out.push(copy.to_string());
            clips.push((clip, copy));
        }
        self.rethread(&clips)?;
        out.reverse();
        Ok(out)
    }

    pub(super) fn copy(&mut self, ids: Vec<String>) -> Res<Vec<String>> {
        self.clipboard = self
            .sorted(&ids)?
            .into_iter()
            .map(|id| (self.clip(id), self.tree.parent(id).and_then(parent_node)))
            .collect();
        Ok(vec![])
    }

    pub(super) fn paste_clipboard(
        &self,
        above: Vec<String>,
        page: Option<String>,
    ) -> Res<Vec<String>> {
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
            self.meta(copy).delete(OVERRIDE_OF).map_err(err)?;
            out.push(copy.to_string());
            pasted.push((clip, copy));
        }
        self.rethread(&pasted)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

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
}
