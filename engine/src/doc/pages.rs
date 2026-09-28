use super::*;

/// Where a page sits on its spread; see `Doc::places`.
pub(super) struct Place {
    pub(super) id: TreeID,
    pub(super) spread: usize,
    pub(super) side: Option<Side>,
    pub(super) x: f64,
}

/// The indices of the pages of each spread among `n` pages, as `Snapshot::spreads`.
pub(super) fn spreads(n: usize, facing: bool) -> Vec<Vec<usize>> {
    match facing {
        false => (0..n).map(|i| vec![i]).collect(),
        true => (0..n.min(1))
            .map(|i| vec![i])
            .chain((1..n).step_by(2).map(|i| (i..(i + 2).min(n)).collect()))
            .collect(),
    }
}

impl Doc {
    /// Where each page sits: its spread, its side and the x of its left edge on the
    /// spread, whose spine is at 0.
    pub(super) fn places(&self) -> Vec<Place> {
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
    pub(super) fn split_masters(&self) -> Res<()> {
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
                    .insert(LEFT_OF, c.to_string())
                    .map_err(err)?;
                self.relink(&lefts, m, c, copy)?;
            }
            self.rethread(&clips)?;
        }
        Ok(())
    }

    /// Makes each master one page again without the layers of its left page; left
    /// pages that override one of its copies override the original again.
    pub(super) fn join_masters(&self) -> Res<()> {
        let lefts = self.left_pages();
        for m in self.masters() {
            for c in self.children(m) {
                let [x, _, w, _] = self.bounds(c);
                if x + w > 0.0 {
                    continue;
                }
                let of = value(&self.meta(c), LEFT_OF).and_then(|v| v.into_string().ok());
                if let Some(o) = of.and_then(|o| self.node(&o).ok()) {
                    self.relink(&lefts, m, c, o)?;
                }
                self.unlink_all(c)?;
                self.remove(c)?;
            }
        }
        Ok(())
    }

    pub(super) fn left_pages(&self) -> Vec<TreeID> {
        let places = self.places().into_iter();
        places
            .filter(|p| p.side == Some(Side::Left))
            .map(|p| p.id)
            .collect()
    }

    /// Makes the pages among `pages` that use the master `m` and override its layer
    /// `from` override `to` instead.
    pub(super) fn relink(&self, pages: &[TreeID], m: TreeID, from: TreeID, to: TreeID) -> Res<()> {
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
                .insert(DETACHED, loro(detached)?)
                .map_err(err)?;
            for c in self.children(p) {
                if value(&self.meta(c), OVERRIDE_OF).and_then(|v| v.into_string().ok())
                    == Some(from.clone().into())
                {
                    self.meta(c).insert(OVERRIDE_OF, to.clone()).map_err(err)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn facing_pages(&self) -> bool {
        value(&self.doc.get_map("document"), "facingPages") == Some(LoroValue::Bool(true))
    }

    /// The pages in order.
    pub(super) fn pages(&self) -> Vec<TreeID> {
        let roots = self.tree.roots().into_iter();
        roots
            .filter(|&r| self.kind(r) == Some(NodeKind::Page))
            .collect()
    }

    pub(super) fn masters(&self) -> Vec<TreeID> {
        let roots = self.tree.roots().into_iter();
        roots
            .filter(|&r| self.kind(r) == Some(NodeKind::Master))
            .collect()
    }

    pub(super) fn page(&self, id: &str) -> Res<TreeID> {
        self.root_of_kind(id, NodeKind::Page)
    }

    pub(super) fn master(&self, id: &str) -> Res<TreeID> {
        self.root_of_kind(id, NodeKind::Master)
    }

    /// A page or a master.
    pub(super) fn sheet(&self, id: &str) -> Res<TreeID> {
        self.page(id).or_else(|_| self.master(id))
    }

    pub(super) fn root_of_kind(&self, id: &str, kind: NodeKind) -> Res<TreeID> {
        let t = TreeID::try_from(id).map_err(err)?;
        match self.tree.parent(t) {
            Some(TreeParentId::Root) if self.kind(t) == Some(kind) => Ok(t),
            _ => Err(format!("no {} {id}", kind.as_str())),
        }
    }

    /// The master the page `p` uses, if it is still there.
    pub(super) fn master_of(&self, p: TreeID) -> Option<TreeID> {
        let v = value(&self.meta(p), MASTER)?;
        self.master(&v.into_string().ok()?).ok()
    }

    /// Makes the layers of page `p` that override its master's plain page layers.
    pub(super) fn leave_master(&self, p: TreeID) -> Res<()> {
        for c in self.children(p) {
            if self.override_of(c).is_some() {
                self.meta(c).delete(OVERRIDE_OF).map_err(err)?;
            }
        }
        self.meta(p).delete(DETACHED).map_err(err)
    }

    /// The master layer that the page layer `id` overrides.
    pub(super) fn override_of(&self, id: TreeID) -> Option<String> {
        value(&self.meta(id), OVERRIDE_OF)
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
    }

    pub(super) fn detached(&self, p: TreeID) -> Vec<String> {
        value(&self.meta(p), DETACHED)
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default()
    }

    pub(super) fn set_document(
        &self,
        raster_ppi: Option<f64>,
        color_mode: Option<ColorMode>,
        facing_pages: Option<bool>,
        ink_limit: Option<f64>,
    ) -> Res<Vec<String>> {
        if raster_ppi.is_some_and(|ppi| !(72.0..=1200.0).contains(&ppi)) {
            return Err("raster ppi must be in 72..=1200".into());
        }
        if ink_limit.is_some_and(|l| !(200.0..=400.0).contains(&l)) {
            return Err("ink limit must be in 200..=400".into());
        }
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
            m.insert("rasterPpi", ppi).map_err(err)?;
        }
        if let Some(mode) = color_mode {
            m.insert("colorMode", loro(mode)?).map_err(err)?;
        }
        if let Some(limit) = ink_limit {
            m.insert("inkLimit", limit).map_err(err)?;
        }
        Ok(vec![])
    }

    pub(super) fn add_page(&self, after: Option<String>) -> Res<Vec<String>> {
        let pages = self.pages();
        let like = match after {
            Some(a) => self.page(&a)?,
            None => *pages.last().ok_or("no page")?,
        };
        let p = self.tree.create(None).map_err(err)?;
        self.tree.mov_after(p, like).map_err(err)?;
        let (m, from) = (self.meta(p), self.meta(like));
        m.insert(KIND, NodeKind::Page.as_str()).map_err(err)?;
        for k in ["width", "height", "bleed"] {
            m.insert(k, num(&from, k)).map_err(err)?;
        }
        if let Some(master) = value(&from, MASTER) {
            m.insert(MASTER, master).map_err(err)?;
        }
        Ok(vec![p.to_string()])
    }

    pub(super) fn duplicate_page(&self, id: String) -> Res<Vec<String>> {
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
        Ok(vec![p.to_string()])
    }

    pub(super) fn set_page(
        &self,
        id: String,
        width: Option<f64>,
        height: Option<f64>,
        bleed: Option<f64>,
    ) -> Res<Vec<String>> {
        let m = self.meta(self.sheet(&id)?);
        let sizes = [("width", width), ("height", height), ("bleed", bleed)];
        if let Some((k, _)) = sizes
            .iter()
            .find(|(_, v)| v.is_some_and(|v| !(v >= 0.0 && v.is_finite())))
        {
            return Err(format!("{k} must not be negative"));
        }
        for (k, v) in sizes {
            if let Some(v) = v {
                m.insert(k, v).map_err(err)?;
            }
        }
        Ok(vec![])
    }

    pub(super) fn delete_page(&self, id: String) -> Res<Vec<String>> {
        let p = self.page(&id)?;
        if self.pages().len() == 1 {
            return Err("a document keeps one page".into());
        }
        self.unlink_all(p)?;
        self.remove(p)?;
        Ok(vec![])
    }

    pub(super) fn move_page(&self, id: String, index: usize) -> Res<Vec<String>> {
        let p = self.page(&id)?;
        let others: Vec<_> = self.pages().into_iter().filter(|&o| o != p).collect();
        match others.get(index) {
            Some(&o) => self.tree.mov_before(p, o).map_err(err)?,
            None => self
                .tree
                .mov_after(p, *others.last().ok_or("no other page")?)
                .map_err(err)?,
        }
        Ok(vec![])
    }

    pub(super) fn add_master(&self, like: Option<String>) -> Res<Vec<String>> {
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
        m.insert(KIND, NodeKind::Master.as_str()).map_err(err)?;
        m.insert("name", name).map_err(err)?;
        for k in ["width", "height", "bleed"] {
            m.insert(k, num(&from, k)).map_err(err)?;
        }
        Ok(vec![p.to_string()])
    }

    pub(super) fn set_master(&self, id: String, name: String) -> Res<Vec<String>> {
        let m = self.master(&id)?;
        if self
            .masters()
            .into_iter()
            .any(|o| o != m && self.name(o) == name)
        {
            return Err(format!("a master named {name} exists"));
        }
        self.meta(m).insert("name", name).map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn delete_master(&self, id: String) -> Res<Vec<String>> {
        let m = self.master(&id)?;
        for p in self.pages() {
            if self.master_of(p) == Some(m) {
                self.leave_master(p)?;
                self.meta(p).delete(MASTER).map_err(err)?;
            }
        }
        self.unlink_all(m)?;
        self.remove(m)?;
        Ok(vec![])
    }

    pub(super) fn use_master(&self, page: String, master: Option<String>) -> Res<Vec<String>> {
        let p = self.page(&page)?;
        let m = master.map(|m| self.master(&m)).transpose()?;
        if m != self.master_of(p) {
            self.leave_master(p)?;
        }
        match m {
            Some(m) => self.meta(p).insert(MASTER, m.to_string()),
            None => self.meta(p).delete(MASTER),
        }
        .map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn override_layer(&self, page: String, id: String) -> Res<Vec<String>> {
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
            .insert(OVERRIDE_OF, id.clone())
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
            .insert(DETACHED, loro(detached)?)
            .map_err(err)?;
        Ok(vec![copy.to_string()])
    }

    pub(super) fn reset_to_master(&self, ids: Vec<String>) -> Res<Vec<String>> {
        for id in &ids {
            if self.page(id).is_err() && self.override_of(self.node(id)?).is_none() {
                return Err(format!("{id} overrides no master layer"));
            }
        }
        for id in ids {
            if let Ok(p) = self.page(&id) {
                let copies: Vec<TreeID> = self
                    .children(p)
                    .into_iter()
                    .filter(|&c| value(&self.meta(c), OVERRIDE_OF).is_some())
                    .collect();
                for c in copies {
                    self.remove(c)?;
                }
                self.meta(p).delete(DETACHED).map_err(err)?;
                continue;
            }
            // Gone with its page earlier in the list.
            let Ok(c) = self.node(&id) else { continue };
            let of = self.override_of(c).unwrap_or_default();
            let p = self.root(c);
            self.remove(c)?;
            let mut detached = self.detached(p);
            detached.retain(|d| *d != of);
            self.meta(p)
                .insert(DETACHED, loro(detached)?)
                .map_err(err)?;
        }
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

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

    #[test]
    fn a_hidden_or_locked_master_layer_is_not_hit_and_a_hidden_one_not_drawn_on_its_pages() {
        let (mut d, p) = empty();
        let m = add_master(&mut d);
        let under = create(&mut d, &m, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        use_master(&mut d, &p, Some(&m)).unwrap();
        let props = |hidden, locked| Props {
            hidden: Some(hidden),
            locked: Some(locked),
            ..Props::default()
        };
        set(&mut d, &under, props(false, true));
        assert_eq!(d.master_hit(&p, 15.0, 15.0, 0.0), None);
        assert_eq!(items(&page_ops(&d)).len(), 1);
        set(&mut d, &under, props(true, false));
        assert_eq!(d.master_hit(&p, 15.0, 15.0, 0.0), None);
        assert!(items(&page_ops(&d)).is_empty());
        assert!(items(&d.render(&m)).is_empty());
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
            d.build_snapshot()
                .pages
                .into_iter()
                .map(|p| p.master)
                .collect()
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
    fn overrides_become_page_layers_when_the_page_leaves_its_master() {
        for delete in [true, false] {
            let (mut d, p) = empty();
            let [a, b] = [add_master(&mut d), add_master(&mut d)];
            let r = create(&mut d, &a, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
            use_master(&mut d, &p, Some(&a)).unwrap();
            let copy = d
                .apply(Command::Override {
                    page: p.clone(),
                    id: r,
                })
                .unwrap()
                .remove(0);
            if delete {
                d.apply(Command::DeleteMaster { id: a }).unwrap();
            } else {
                use_master(&mut d, &p, Some(&b)).unwrap();
            }
            let pg = page(&d);
            assert!(pg.detached.is_empty());
            assert_eq!(pg.children[0].override_of, None);
            d.apply(Command::ResetToMaster { ids: vec![p] }).unwrap();
            assert_eq!(ids(&page(&d).children), [copy]);
        }
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
}
