use super::*;

/// The grid of a table frame, whose text layers with a `CELL` are its cells.
pub(super) const TABLE: &str = "table";
/// Where a cell sits in its table: column, row, and the columns and rows it spans.
pub(super) const CELL: &str = "cell";
const PADDING: f64 = 4.0;
const COLUMN: f64 = 60.0;

/// The column widths and least row heights in pt of a table; a row grows to its
/// cells' text, and the table scales its columns to its width and its rows to a
/// height above theirs.
#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<f64>,
    pub rows: Vec<f64>,
}

impl Table {
    fn sound(&self) -> bool {
        let ok = |v: &f64, min: f64| v.is_finite() && *v >= min && *v <= MAX_LENGTH;
        !self.columns.is_empty()
            && !self.rows.is_empty()
            && self.columns.iter().all(|v| ok(v, 1e-3))
            && self.rows.iter().all(|v| ok(v, 0.0))
    }
}

type Cell = [usize; 4];

fn ends(sizes: &[f64]) -> Vec<f64> {
    std::iter::once(0.0)
        .chain(sizes.iter().scan(0.0, |s, v| {
            *s += v;
            Some(*s)
        }))
        .collect()
}

impl Doc {
    pub(super) fn table(&self, id: TreeID) -> Option<Table> {
        from_loro(value(&self.meta(id), TABLE)?).filter(Table::sound)
    }

    /// The cells of the table `id` with where they sit, clamped to its grid.
    fn cells(&self, id: TreeID, t: &Table) -> Vec<(TreeID, Cell)> {
        let (nc, nr) = (t.columns.len(), t.rows.len());
        self.children(id)
            .into_iter()
            .filter(|&c| self.kind(c) == Some(NodeKind::Text))
            .filter_map(|c| {
                let [x, y, w, h]: Cell = from_loro(value(&self.meta(c), CELL)?)?;
                let (x, y) = (x.min(nc - 1), y.min(nr - 1));
                Some((c, [x, y, w.clamp(1, nc - x), h.clamp(1, nr - y)]))
            })
            .collect()
    }

    fn put_table(&self, id: TreeID, t: &Table) -> Res<()> {
        self.meta(id).insert(TABLE, loro(t)?).map_err(err)?;
        let [x, y, ..] = self.bounds(id);
        let (w, h) = (t.columns.iter().sum(), t.rows.iter().sum());
        self.set_frame(id, [x, y, w, h])
    }

    fn put_cell(&self, id: TreeID, c: Cell) -> Res<()> {
        self.meta(id).insert(CELL, loro(c)?).map_err(err)
    }

    /// Makes the new frame `id` a table of 3 by 3 empty cells filling it.
    pub(super) fn fill_table(&self, id: TreeID) -> Res<()> {
        let [x, y, w, h] = self.bounds(id);
        let t = Table {
            columns: vec![if w > 0.0 { w / 3.0 } else { COLUMN }; 3],
            rows: vec![h / 3.0; 3],
        };
        self.put_table(id, &t)?;
        let mode = self.color_mode();
        for r in 0..3 {
            for c in 0..3 {
                let n = self.create(&id.to_string(), NewKind::Text, [x, y, 1.0, 1.0])?;
                self.set(
                    n,
                    Props {
                        sizing: Some(Sizing::default()),
                        strokes: Some(vec![Fill::solid(Color::black(mode))]),
                        stroke_weight: Some(0.5),
                        stroke_align: Some(Align::Center),
                        inset_top: Some(PADDING),
                        inset_right: Some(PADDING),
                        inset_bottom: Some(PADDING),
                        inset_left: Some(PADDING),
                        ..Props::default()
                    },
                )?;
                self.put_cell(n, [c, r, 1, 1])?;
            }
        }
        Ok(())
    }

    /// A new empty cell at `at` in `table` that looks like the cell `like`.
    fn new_cell(&self, table: TreeID, at: Cell, like: TreeID) -> Res<()> {
        let n = self.new_node(table, None)?;
        let m = self.meta(n);
        let from = self.meta(like);
        for (k, v) in from.get_value().into_map().ok().into_iter().flat_map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
        }) {
            if !["text", "name", CELL, NEXT, OVERRIDE_OF, ANCHORED].contains(&&*k) {
                m.insert(&k, v).map_err(err)?;
            }
        }
        m.insert_container("text", LoroText::new()).map_err(err)?;
        let [x, y, ..] = self.bounds(table);
        self.set_frame(n, [x, y, 1.0, 1.0])?;
        self.put_cell(n, at)
    }

    /// The table `id` or the table of the cell `id`.
    fn table_of(&self, id: &str) -> Res<(TreeID, Table)> {
        let mut n = self.layer(id)?;
        if self.table(n).is_none() && value(&self.meta(n), CELL).is_some() {
            n = self
                .tree
                .parent(n)
                .and_then(parent_node)
                .ok_or("no table")?;
        }
        Ok((n, self.table(n).ok_or(format!("{id} is not in a table"))?))
    }

    /// Adds a row or, with `column`, a column before index `at` of the table of `id`
    /// (the table or one of its cells), sized and styled like its neighbour.
    pub(super) fn table_insert(&self, id: String, column: bool, at: usize) -> Res<Vec<String>> {
        let (n, old) = self.table_of(&id)?;
        let mut t = old.clone();
        let a = usize::from(!column);
        let sizes = if column { &mut t.columns } else { &mut t.rows };
        if at > sizes.len() {
            return Err("index outside the table".into());
        }
        if sizes.len() >= 1000 {
            return Err("a table has at most 1000 rows and columns".into());
        }
        sizes.insert(at, sizes[at.saturating_sub(1).min(sizes.len() - 1)]);
        let across = if column {
            old.rows.len()
        } else {
            old.columns.len()
        };
        let (mut covered, mut like) = (vec![false; across], vec![None; across]);
        for (c, mut cell) in self.cells(n, &old) {
            let (pos, span) = (cell[a], cell[a + 2]);
            let range = cell[1 - a]..cell[1 - a] + cell[3 - a];
            if pos >= at {
                cell[a] += 1;
            } else if pos + span > at {
                cell[a + 2] += 1;
                range.clone().for_each(|i| covered[i] = true);
            }
            if pos + span == at || (at == 0 && pos == 0) {
                range.for_each(|i| like[i] = Some(c));
            }
            self.put_cell(c, cell)?;
        }
        self.put_table(n, &t)?;
        for i in (0..across).filter(|&i| !covered[i]) {
            if let Some(l) = like[i] {
                let mut cell = [0, 0, 1, 1];
                (cell[a], cell[1 - a]) = (at, i);
                self.new_cell(n, cell, l)?;
            }
        }
        Ok(vec![])
    }

    /// Removes the row or, with `column`, the column `at` of the table of `id`, and
    /// the cells only in it.
    pub(super) fn table_delete(&self, id: String, column: bool, at: usize) -> Res<Vec<String>> {
        let (n, old) = self.table_of(&id)?;
        let mut t = old.clone();
        let a = usize::from(!column);
        let sizes = if column { &mut t.columns } else { &mut t.rows };
        if at >= sizes.len() {
            return Err("index outside the table".into());
        }
        if sizes.len() == 1 {
            return Err("a table keeps 1 row and column".into());
        }
        sizes.remove(at);
        for (c, mut cell) in self.cells(n, &old) {
            let (pos, span) = (cell[a], cell[a + 2]);
            if pos == at && span == 1 {
                self.remove(c)?;
                continue;
            }
            if pos > at {
                cell[a] -= 1;
            } else if pos + span > at {
                cell[a + 2] -= 1;
            }
            self.put_cell(c, cell)?;
        }
        self.put_table(n, &t).map(|()| vec![])
    }

    /// Merges the cells `ids`, which make a rectangle of one table, into the first,
    /// which takes their text one paragraph after another.
    pub(super) fn merge_cells(&self, ids: Vec<String>) -> Res<Vec<String>> {
        let (n, t) = self.table_of(ids.first().ok_or("no cells")?)?;
        let cells = self.cells(n, &t);
        let mut chosen = Vec::new();
        for id in &ids {
            let c = self.layer(id)?;
            let cell = cells
                .iter()
                .find(|x| x.0 == c)
                .ok_or(format!("{id} is not a cell of the table"))?;
            if !chosen.contains(cell) {
                chosen.push(*cell);
            }
        }
        if chosen.len() < 2 {
            return Err("merging takes 2 cells or more".into());
        }
        let lo = |i: usize| chosen.iter().map(|c| c.1[i]).min().unwrap();
        let hi = |i: usize| chosen.iter().map(|c| c.1[i] + c.1[i + 2]).max().unwrap();
        let rect = [lo(0), lo(1), hi(0) - lo(0), hi(1) - lo(1)];
        let inside = |c: &Cell| {
            c[0] < rect[0] + rect[2]
                && c[0] + c[2] > rect[0]
                && c[1] < rect[1] + rect[3]
                && c[1] + c[3] > rect[1]
        };
        let area: usize = chosen.iter().map(|c| c.1[2] * c.1[3]).sum();
        if area != rect[2] * rect[3] || cells.iter().any(|c| inside(&c.1) && !chosen.contains(c)) {
            return Err("the cells do not make a rectangle".into());
        }
        chosen.sort_by_key(|c| (c.1[1], c.1[0]));
        let (keep, _) = chosen[0];
        let text = self.own_text(keep)?;
        for &(c, _) in &chosen[1..] {
            let own = self.own_text(c)?;
            if own.len_utf16() > 0 {
                let mut at = text.len_utf16();
                if at > 0 {
                    text.insert_utf16(at, "\n").map_err(err)?;
                    at += 1;
                }
                let mut delta = vec![TextDelta::Retain {
                    retain: at,
                    attributes: None,
                }];
                delta.extend(own.to_delta());
                text.apply_delta(&delta).map_err(err)?;
            }
            self.remove(c)?;
        }
        self.put_cell(keep, rect)?;
        Ok(vec![keep.to_string()])
    }

    /// Splits the merged cell `id` into one cell per column and row it spans; the
    /// first keeps its text.
    pub(super) fn split_cell(&self, id: String) -> Res<Vec<String>> {
        let (n, t) = self.table_of(&id)?;
        let c = self.layer(&id)?;
        let [x, y, w, h] = self
            .cells(n, &t)
            .into_iter()
            .find(|x| x.0 == c)
            .ok_or(format!("{id} is not a cell"))?
            .1;
        if w * h == 1 {
            return Err(format!("{id} is not merged"));
        }
        self.put_cell(c, [x, y, 1, 1])?;
        for (i, j) in (y..y + h)
            .flat_map(|j| (x..x + w).map(move |i| (i, j)))
            .skip(1)
        {
            self.new_cell(n, [i, j, 1, 1], c)?;
        }
        Ok(vec![])
    }

    /// Sets the column widths and least row heights of the table `id`, as many as it
    /// has.
    pub(super) fn size_table(&self, id: String, t: Table) -> Res<Vec<String>> {
        let (n, old) = self.table_of(&id)?;
        if t.columns.len() != old.columns.len() || t.rows.len() != old.rows.len() {
            return Err("as many columns and rows as the table has".into());
        }
        if !t.sound() {
            return Err("column widths are above 0 and row heights not below".into());
        }
        self.put_table(n, &t).map(|()| vec![])
    }

    /// The heights of the rows of the table `id`: at least their least height, and
    /// enough for the text of their cells, a cell spanning rows growing the last and a
    /// threaded cell not at all.
    fn row_heights(&self, id: TreeID, t: &Table) -> Res<Vec<f64>> {
        let xs = ends(&t.columns);
        let mut cells = self.cells(id, t);
        cells.sort_by_key(|c| c.1[3]);
        let mut rows = t.rows.clone();
        for (c, [i, j, cw, ch]) in cells {
            if self.story(c) != c || self.next_of(c).is_some() {
                continue;
            }
            let need = self.natural(c, xs[i + cw] - xs[i])?;
            let have: f64 = rows[j..j + ch].iter().sum();
            if need > have {
                rows[j + ch - 1] += need - have;
            }
        }
        Ok(rows)
    }

    /// Scales the columns of the table `id` to the width `w` and its rows to `h`.
    pub(super) fn scale_table(&self, id: TreeID, w: f64, h: f64) -> Res<()> {
        let Some(mut t) = self.table(id) else {
            return Ok(());
        };
        let rows = self.row_heights(id, &t)?;
        let (sw, sh): (f64, f64) = (t.columns.iter().sum(), rows.iter().sum());
        if w > 0.0 {
            t.columns
                .iter_mut()
                .for_each(|c| *c = (*c * w / sw).max(1e-3));
        }
        if h > 0.0 && sh > 0.0 {
            t.rows = rows.iter().map(|r| r * h / sh).collect();
        }
        self.meta(id).insert(TABLE, loro(&t)?).map_err(err)
    }

    /// Sets the frames of the cells of the table `id` and the table's size.
    pub(super) fn lay_out_table(&self, id: TreeID, t: Table) -> Res<()> {
        let [x, y, ..] = self.bounds(id);
        let (xs, ys) = (ends(&t.columns), ends(&self.row_heights(id, &t)?));
        let apart = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).any(|(a, b)| (a - b).abs() > 1e-9);
        let frame = [x, y, xs[xs.len() - 1], ys[ys.len() - 1]];
        if apart(frame, self.bounds(id)) {
            self.set_frame(id, frame)?;
        }
        for (c, [i, j, cw, ch]) in self.cells(id, &t) {
            let f = [x + xs[i], y + ys[j], xs[i + cw] - xs[i], ys[j + ch] - ys[j]];
            if apart(f, self.bounds(c)) {
                self.set_frame(c, f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::draw::address;
    use super::super::tests::*;
    use super::*;

    /// The cells of the table `id`, in reading order, with their frames and text.
    fn grid(d: &Doc, id: &str) -> Vec<(Cell, [f64; 4], String)> {
        let n = d.node(id).unwrap();
        let t = d.table(n).unwrap();
        let mut out: Vec<_> = d
            .cells(n, &t)
            .into_iter()
            .map(|(c, cell)| (cell, d.bounds(c), d.own_text(c).unwrap().to_string()))
            .collect();
        out.sort_by_key(|c| (c.0[1], c.0[0]));
        out
    }

    fn cell(d: &Doc, id: &str, at: [usize; 2]) -> String {
        let n = d.node(id).unwrap();
        let t = d.table(n).unwrap();
        let (c, _) = d.cells(n, &t).into_iter().find(|c| c.1[..2] == at).unwrap();
        c.to_string()
    }

    #[test]
    fn a_table_grows_its_rows_to_their_text_and_inserts_deletes_merges_and_splits() {
        let (mut d, p) = empty();
        let id = create(&mut d, &p, NewKind::Table, [10.0, 10.0, 300.0, 90.0]);
        let g = grid(&d, &id);
        assert_eq!(g.len(), 9);
        assert_eq!(g[4].0, [1, 1, 1, 1]);
        let names: Vec<_> = children(&page(&d).children[0])
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert!(
            ["A1", "B1", "C1", "A2", "B2", "C2", "A3", "B3", "C3"]
                .iter()
                .all(|n| names.contains(&n.to_string())),
            "{names:?}"
        );
        assert_eq!(address(27, 9), "AB10");
        assert_eq!(g[4].1, [110.0, 40.0, 100.0, 30.0]);

        let a = cell(&d, &id, [0, 0]);
        set_text(&mut d, &a, "one\ntwo\nthree");
        let g = grid(&d, &id);
        let h = g[0].1[3];
        assert!(h > 40.0, "{h}");
        assert_eq!(g[1].1[3], h);
        assert_eq!(g[3].1[1], 10.0 + h);
        assert_eq!(d.bounds(d.node(&id).unwrap())[3], h + 60.0);

        let insert = |d: &mut Doc, column, at| {
            d.apply(Command::TableInsert {
                id: a.clone(),
                column,
                at,
            })
        };
        insert(&mut d, false, 1).unwrap();
        insert(&mut d, true, 3).unwrap();
        let g = grid(&d, &id);
        assert_eq!(g.len(), 16);
        assert_eq!(d.bounds(d.node(&id).unwrap())[2], 400.0);
        assert!(g.iter().all(|c| c.1[2] == 100.0 && c.0[2..] == [1, 1]));
        assert!(insert(&mut d, false, 9).is_err());

        let b = cell(&d, &id, [1, 1]);
        let c = cell(&d, &id, [1, 2]);
        let e = cell(&d, &id, [2, 1]);
        set_text(&mut d, &c, "below");
        let merge = |d: &mut Doc, ids: &[&String]| {
            d.apply(Command::MergeCells {
                ids: ids.iter().map(|s| s.to_string()).collect(),
            })
        };
        assert!(merge(&mut d, &[&b, &e, &c]).is_err());
        assert!(merge(&mut d, &[&b]).is_err());
        merge(&mut d, &[&c, &b]).unwrap();
        let at = |d: &Doc| {
            grid(d, &id)
                .into_iter()
                .find(|g| g.0[..2] == [1, 1])
                .unwrap()
        };
        assert_eq!((at(&d).0, at(&d).2), ([1, 1, 1, 2], "below".into()));
        assert_eq!(grid(&d, &id).len(), 15);
        insert(&mut d, false, 2).unwrap();
        assert_eq!(at(&d).0, [1, 1, 1, 3]);
        assert_eq!(grid(&d, &id).len(), 18);
        let delete = |d: &mut Doc, column, at| {
            d.apply(Command::TableDelete {
                id: id.clone(),
                column,
                at,
            })
        };
        delete(&mut d, false, 2).unwrap();
        assert_eq!(grid(&d, &id).len(), 15);
        d.apply(Command::SplitCell { id: b.clone() }).unwrap();
        assert!(d.apply(Command::SplitCell { id: b.clone() }).is_err());
        assert_eq!(grid(&d, &id).len(), 16);

        delete(&mut d, true, 0).unwrap();
        let g = grid(&d, &id);
        assert_eq!(g.len(), 12);
        assert_eq!(g[0].1[0], 10.0);
        assert!(d.node(&a).is_err());

        set_frame(&mut d, &id, [10.0, 10.0, 600.0, 200.0]);
        let g = grid(&d, &id);
        assert!(g.iter().all(|c| close(c.1[2], 200.0)), "{g:?}");
        assert!(close(d.bounds(d.node(&id).unwrap())[3], 200.0));
        let size = |d: &mut Doc, columns: Vec<f64>| {
            d.apply(Command::SizeTable {
                id: id.clone(),
                columns,
                rows: vec![10.0; 4],
            })
        };
        size(&mut d, vec![50.0, 150.0, 100.0]).unwrap();
        assert!(close(grid(&d, &id)[1].1[2], 150.0));
        assert!(size(&mut d, vec![50.0]).is_err());
        assert!(size(&mut d, vec![50.0, 0.0, 1.0]).is_err());
        for _ in 0..11 {
            d.apply(Command::Undo).unwrap();
        }
        assert_eq!(grid(&d, &id).len(), 9);
    }
}
