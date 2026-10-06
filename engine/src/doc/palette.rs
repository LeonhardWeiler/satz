use super::*;

/// Replaces the colours in `v` that `f` maps to another colour.
pub(super) fn map_colors(v: &mut serde_json::Value, f: &impl Fn(&Color) -> Option<Color>) -> bool {
    match v {
        serde_json::Value::Object(o) => o.iter_mut().fold(false, |d, (k, v)| {
            let mapped = match serde_json::from_value::<Color>(v.clone()) {
                Ok(c) if k == "color" || k == "fill" => f(&c),
                _ => None,
            };
            match mapped {
                Some(c) => *v = serde_json::to_value(c).unwrap(),
                None => return map_colors(v, f) | d,
            }
            true
        }),
        serde_json::Value::Array(a) => a.iter_mut().fold(false, |d, v| map_colors(v, f) | d),
        _ => false,
    }
}

pub(super) fn check_value(v: &Value) -> Res<()> {
    match v {
        Value::Color(Color::Variable { .. }) => Err("a variable cannot hold a variable".into()),
        Value::Color(c) => c.check(),
        Value::Number(n) if !n.is_finite() => Err("numbers must be finite".into()),
        Value::Number(_) | Value::Font(_) => Ok(()),
    }
}

impl Doc {
    /// The modes `id` resolves variables in, its own and inherited.
    pub(super) fn active_modes(&self, id: TreeID) -> Modes {
        let mut chain = vec![id];
        while let Some(p) = self
            .tree
            .parent(*chain.last().unwrap())
            .and_then(parent_node)
        {
            chain.push(p);
        }
        let mut modes = Modes::new();
        for n in chain.into_iter().rev() {
            modes.extend(self.modes(n));
        }
        modes
    }

    /// Writes the values of bound variables into `id` and its subtree.
    pub(super) fn settle(&self, id: TreeID, inherited: &Modes, palette: &Palette) -> Res<()> {
        let mut modes = inherited.clone();
        modes.extend(self.modes(id));
        let scope = Scope {
            palette,
            modes: &modes,
        };
        let bindings = self.bindings(id);
        let m = self.meta(id);
        let old = if bindings.is_empty() {
            [0.0; 4]
        } else {
            self.bounds(id)
        };
        let mut frame = old;
        for (prop, var) in bindings {
            let v = match scope.value(&var) {
                Some(&Value::Number(v)) => v,
                Some(Value::Font(f)) if prop == "font" => {
                    let f = loro(f)?;
                    if value(&m, "font").as_ref() != Some(&f) {
                        m.insert("font", f).map_err(err)?;
                    }
                    continue;
                }
                _ => continue,
            };
            let v = match prop.as_str() {
                "opacity" => (v / 100.0).clamp(0.0, 1.0),
                "size" => v.max(0.1),
                _ => v.max(0.0) * MM,
            };
            match prop.as_str() {
                "w" => frame[2] = v,
                "h" => frame[3] = v,
                _ if num(&m, &prop) != v => {
                    m.insert(&prop, v).map_err(err)?;
                }
                _ => {}
            }
        }
        if frame != old {
            self.set_frame(id, frame)?;
        }
        for c in self.children(id) {
            self.settle(c, &modes, palette)?;
        }
        Ok(())
    }

    /// Calls `f` for every node, including deleted ones, with its active modes.
    pub(super) fn each(&self, mut f: impl FnMut(TreeID, &Modes) -> Res<()>) -> Res<()> {
        let mut stack: Vec<_> = self
            .tree
            .roots()
            .into_iter()
            .map(|r| (r, Modes::new()))
            .collect();
        while let Some((id, mut modes)) = stack.pop() {
            modes.extend(self.modes(id));
            f(id, &modes)?;
            stack.extend(self.children(id).into_iter().map(|c| (c, modes.clone())));
        }
        Ok(())
    }

    /// Replaces the colours of every node that `f` maps to another colour.
    pub(super) fn recolor(&self, f: impl Fn(&Color, &Scope) -> Option<Color>) -> Res<()> {
        let palette = self.palette();
        self.each(|id, modes| {
            let s = Scope {
                palette: &palette,
                modes,
            };
            let m = self.meta(id);
            for key in ["fills", "strokes", "effects", "fill"] {
                let Some(v) = value(&m, key) else { continue };
                let mut v = serde_json::to_value(v).map_err(err)?;
                if map_colors(&mut v, &|c| f(c, &s)) {
                    m.insert(key, loro(v)?).map_err(err)?;
                }
            }
            let Ok(t) = self.own_text(id) else {
                return Ok(());
            };
            let mut at = 0;
            for d in t.to_delta() {
                let TextDelta::Insert { insert, attributes } = d else {
                    continue;
                };
                let r = at..at + insert.encode_utf16().count();
                at = r.end;
                let Some(v) = attributes.and_then(|a| a.get("fill").cloned()) else {
                    continue;
                };
                let mut v = serde_json::json!({ "fill": v });
                if map_colors(&mut v, &|c| f(c, &s)) {
                    t.mark_utf16(r, "fill", loro(&v["fill"])?).map_err(err)?;
                }
            }
            Ok(())
        })
    }

    /// Turns every process colour into `mode`; spot colours stay CMYK.
    pub(super) fn convert(&self, mode: ColorMode) -> Res<()> {
        self.recolor(|c, _| c.convert(mode))?;
        for name in ["swatches", "variables", "textStyles"] {
            let all: Vec<serde_json::Value> = self.list(name);
            for (i, mut v) in all.into_iter().enumerate() {
                if v["spot"] != true && map_colors(&mut v, &|c| c.convert(mode)) {
                    self.put(name, Some(i), v)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn delete_variable(&self, id: &str) -> Res<()> {
        let (i, _) = self.find::<Variable>("variables", id)?;
        self.recolor(|c, s| {
            matches!(c, Color::Variable { variable, .. } if variable == id).then(|| c.bind(s))
        })?;
        self.each(|n, _| {
            let bindings = self.bindings(n);
            self.unbind(n, |p| bindings[p] == id)
        })?;
        let palette = self.palette();
        let first = Scope {
            palette: &palette,
            modes: &Modes::new(),
        };
        for (j, s) in palette.text_styles.iter().enumerate() {
            if !s.bindings.values().any(|v| v == id) {
                continue;
            }
            let mut v = serde_json::to_value(s).map_err(err)?;
            v.as_object_mut().unwrap().extend(s.values(&first));
            let mut s: TextStyle = serde_json::from_value(v).map_err(err)?;
            s.bindings.retain(|_, v| v != id);
            self.put("textStyles", Some(j), s)?;
        }
        self.doc.get_list("variables").delete(i, 1).map_err(err)
    }

    /// Drops the modes chosen on any node for which `f(collection, mode)` holds.
    pub(super) fn forget_modes(&self, f: impl Fn(&str, &str) -> bool) -> Res<()> {
        self.each(|n, _| {
            let mut modes = self.modes(n);
            let len = modes.len();
            modes.retain(|c, m| !f(c, m));
            if modes.len() < len {
                self.meta(n).insert(MODES, loro(modes)?).map_err(err)?;
            }
            Ok(())
        })
    }

    pub(super) fn bindings(&self, id: TreeID) -> BTreeMap<String, String> {
        self.json(id, BINDINGS)
    }

    pub(super) fn unbind(&self, id: TreeID, drop: impl Fn(&str) -> bool) -> Res<()> {
        let mut bindings = self.bindings(id);
        let n = bindings.len();
        bindings.retain(|p, _| !drop(p));
        if bindings.len() < n {
            self.meta(id)
                .insert(BINDINGS, loro(bindings)?)
                .map_err(err)?;
        }
        Ok(())
    }

    pub(super) fn swatches(&self) -> Vec<Swatch> {
        self.list("swatches")
    }

    pub(super) fn palette(&self) -> Palette {
        Palette {
            swatches: self.swatches(),
            collections: self.list("collections"),
            variables: self.list("variables"),
            text_styles: self.list("textStyles"),
        }
    }

    pub(super) fn variables(&self, collection: &str) -> impl Iterator<Item = (usize, Variable)> {
        let all: Vec<Variable> = self.list("variables");
        let collection = collection.to_string();
        all.into_iter()
            .enumerate()
            .filter(move |(_, v)| v.collection == collection)
    }

    pub(super) fn modes(&self, id: TreeID) -> Modes {
        self.json(id, MODES)
    }

    pub(super) fn new_id(&self) -> String {
        format!("{}@{}", self.doc.len_ops(), self.doc.peer_id())
    }

    pub(super) fn check_swatch(&self, swatch: &Swatch) -> Res<()> {
        swatch.check()?;
        if self
            .swatches()
            .iter()
            .any(|s| s.id != swatch.id && s.name == swatch.name)
        {
            return Err(format!("a swatch named {} exists", swatch.name));
        }
        Ok(())
    }

    pub(super) fn swatch(&self, id: &str) -> Res<(usize, Swatch)> {
        self.find("swatches", id)
    }

    pub(super) fn add_swatch(&self, name: String, color: Color, spot: bool) -> Res<Vec<String>> {
        let id = self.new_id();
        let swatch = Swatch {
            id: id.clone(),
            name,
            color,
            spot,
        };
        self.check_swatch(&swatch)?;
        self.put("swatches", None, swatch)?;
        Ok(vec![id])
    }

    pub(super) fn set_swatch(
        &self,
        id: String,
        name: Option<String>,
        color: Option<Color>,
        spot: Option<bool>,
    ) -> Res<Vec<String>> {
        let (i, mut swatch) = self.swatch(&id)?;
        swatch.name = name.unwrap_or(swatch.name);
        swatch.color = color.unwrap_or(swatch.color);
        swatch.spot = spot.unwrap_or(swatch.spot);
        self.check_swatch(&swatch)?;
        self.put("swatches", Some(i), swatch)?;
        Ok(vec![])
    }

    pub(super) fn delete_swatch(&self, id: String) -> Res<Vec<String>> {
        let (i, _) = self.swatch(&id)?;
        let used = |c: &Color| matches!(c, Color::Swatch { swatch, .. } if *swatch == id);
        self.recolor(|c, s| used(c).then(|| c.resolve(s)))?;
        let palette = self.palette();
        let scope = Scope {
            palette: &palette,
            modes: &Modes::new(),
        };
        for (j, mut v) in palette.variables.iter().cloned().enumerate() {
            let mut changed = false;
            for value in v.values.values_mut() {
                if let Value::Color(c) = value
                    && used(c)
                {
                    *c = c.resolve(&scope);
                    changed = true;
                }
            }
            if changed {
                self.put("variables", Some(j), v)?;
            }
        }
        self.doc.get_list("swatches").delete(i, 1).map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn set_collection(&self, id: String, name: String) -> Res<Vec<String>> {
        let (i, mut c) = self.find::<Collection>("collections", &id)?;
        c.name = name;
        self.put("collections", Some(i), c)?;
        Ok(vec![])
    }

    pub(super) fn delete_collection(&self, id: String) -> Res<Vec<String>> {
        let (i, _) = self.find::<Collection>("collections", &id)?;
        for (_, v) in self.variables(&id).collect::<Vec<_>>() {
            self.delete_variable(&v.id)?;
        }
        self.forget_modes(|c, _| c == id)?;
        self.doc.get_list("collections").delete(i, 1).map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn set_mode(
        &self,
        collection: String,
        id: String,
        name: String,
    ) -> Res<Vec<String>> {
        let (i, mut c) = self.find::<Collection>("collections", &collection)?;
        let m = c
            .modes
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| format!("no mode {id}"))?;
        m.name = name;
        self.put("collections", Some(i), c)?;
        Ok(vec![])
    }

    pub(super) fn delete_mode(&self, collection: String, id: String) -> Res<Vec<String>> {
        let (i, mut c) = self.find::<Collection>("collections", &collection)?;
        if !c.modes.iter().any(|m| m.id == id) {
            return Err(format!("no mode {id}"));
        }
        if c.modes.len() == 1 {
            return Err("a collection keeps one mode".into());
        }
        c.modes.retain(|m| m.id != id);
        self.put("collections", Some(i), c)?;
        for (j, mut v) in self.variables(&collection).collect::<Vec<_>>() {
            v.values.remove(&id);
            self.put("variables", Some(j), v)?;
        }
        self.forget_modes(|c, m| c == collection && m == id)?;
        Ok(vec![])
    }

    pub(super) fn add_collection(&self, name: String) -> Res<Vec<String>> {
        let id = self.new_id();
        let mode = format!("{id}.0");
        self.put(
            "collections",
            None,
            Collection {
                id: id.clone(),
                name,
                modes: vec![Mode {
                    id: mode.clone(),
                    name: "Mode 1".into(),
                }],
            },
        )?;
        Ok(vec![id, mode])
    }

    pub(super) fn add_mode(&self, collection: String, name: String) -> Res<Vec<String>> {
        let (i, mut c) = self.find::<Collection>("collections", &collection)?;
        let id = self.new_id();
        let first = c.modes[0].id.clone();
        c.modes.push(Mode {
            id: id.clone(),
            name,
        });
        self.put("collections", Some(i), c)?;
        for (i, mut v) in self.variables(&collection) {
            v.values.insert(id.clone(), v.values[&first].clone());
            self.put("variables", Some(i), v)?;
        }
        Ok(vec![id])
    }

    pub(super) fn add_variable(
        &self,
        collection: String,
        name: String,
        value: Value,
    ) -> Res<Vec<String>> {
        let (_, c) = self.find::<Collection>("collections", &collection)?;
        check_value(&value)?;
        if self.variables(&collection).any(|(_, v)| v.name == name) {
            return Err(format!("a variable named {name} exists"));
        }
        let id = self.new_id();
        let values = c.modes.into_iter().map(|m| (m.id, value.clone()));
        self.put(
            "variables",
            None,
            Variable {
                id: id.clone(),
                collection,
                name,
                values: values.collect(),
            },
        )?;
        Ok(vec![id])
    }

    pub(super) fn set_variable(
        &self,
        id: String,
        name: Option<String>,
        mode: Option<String>,
        value: Option<Value>,
    ) -> Res<Vec<String>> {
        let (i, mut v) = self.find::<Variable>("variables", &id)?;
        if let Some(name) = name {
            if self
                .variables(&v.collection)
                .any(|(_, o)| o.id != id && o.name == name)
            {
                return Err(format!("a variable named {name} exists"));
            }
            v.name = name;
        }
        if let Some(value) = value {
            let (_, c) = self.find::<Collection>("collections", &v.collection)?;
            let mode = mode.unwrap_or_else(|| c.modes[0].id.clone());
            let old = v
                .values
                .get(&mode)
                .ok_or_else(|| format!("no mode {mode}"))?;
            check_value(&value)?;
            if !old.same_kind(&value) {
                return Err("a variable keeps its type".into());
            }
            v.values.insert(mode, value);
        }
        self.put("variables", Some(i), v)?;
        Ok(vec![])
    }

    pub(super) fn bind(
        &self,
        id: String,
        prop: String,
        variable: Option<String>,
    ) -> Res<Vec<String>> {
        if let Some(v) = &variable {
            let (_, var) = self.find::<Variable>("variables", v)?;
            let font = prop == "font";
            if !var
                .values
                .values()
                .all(|v| matches!(v, Value::Font(_)) == font && !matches!(v, Value::Color(_)))
            {
                return Err("a variable binds to a property of its type".into());
            }
        }
        if let Ok((i, mut s)) = self.find::<TextStyle>("textStyles", &id) {
            let own = serde_json::to_value(&s.attrs).map_err(err)?;
            if !STYLED.contains(&prop.as_str()) || prop != "font" && !own[&prop].is_f64() {
                return Err(format!("{prop} cannot be bound"));
            }
            match variable {
                Some(v) => s.bindings.insert(prop, v),
                None => s.bindings.remove(&prop),
            };
            self.put("textStyles", Some(i), s)?;
            return Ok(vec![]);
        }
        let n = self.node(&id)?;
        let story = prop == "size" || prop == "font";
        let n = if story { self.story(n) } else { n };
        if !BINDABLE.contains(&prop.as_str()) {
            return Err(format!("{prop} cannot be bound"));
        }
        if story && variable.is_some() {
            let t = self.text(n)?;
            if prop == "size" {
                self.detach(n, None, |_| true)?;
            }
            if t.len_utf16() > 0 {
                t.unmark_utf16(0..t.len_utf16(), &prop).map_err(err)?;
            }
        }
        let mut bindings = self.bindings(n);
        match variable {
            Some(v) => bindings.insert(prop, v),
            None => bindings.remove(&prop),
        };
        self.meta(n)
            .insert(BINDINGS, loro(bindings)?)
            .map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn use_mode(
        &self,
        id: String,
        collection: String,
        mode: Option<String>,
    ) -> Res<Vec<String>> {
        let n = self.node(&id)?;
        let (_, c) = self.find::<Collection>("collections", &collection)?;
        let mut modes = self.modes(n);
        match mode {
            Some(m) if c.modes.iter().any(|x| x.id == m) => modes.insert(collection, m),
            Some(m) => return Err(format!("no mode {m}")),
            None => modes.remove(&collection),
        };
        self.meta(n).insert(MODES, loro(modes)?).map_err(err)?;
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

    #[test]
    fn concurrent_edits_to_a_variable_merge_field_by_field() {
        let (mut d, _) = empty();
        let (c, light) = collection(&mut d, "Theme");
        let v = variable(&mut d, &c, "Gap", Value::Number(4.0)).unwrap();
        let mut a = Doc::load(&d.save()).unwrap();
        let mut b = Doc::load(&d.save()).unwrap();
        a.apply(Command::SetVariable {
            id: v.clone(),
            name: Some("Space".into()),
            mode: None,
            value: None,
        })
        .unwrap();
        set_value(&mut b, &v, &light, Value::Number(8.0)).unwrap();
        a.doc
            .import(&b.doc.export(ExportMode::all_updates()).unwrap())
            .unwrap();
        let m = Doc::load(&a.save()).unwrap().snapshot();
        let var = &m.palette.variables[0];
        assert_eq!(
            (var.name.as_str(), &var.values[&light]),
            ("Space", &Value::Number(8.0))
        );
    }

    #[test]
    fn switching_to_cmyk_converts_every_process_colour_but_spots_and_undo_restores_them() {
        let mut d = Doc::sample();
        assert_eq!(d.snapshot().color_mode, ColorMode::Rgb);
        let t = text(&mut d, "Hi there");
        let red = Color::Rgb(0xff0000ff);
        let range = TextProps {
            fill: Some(red.clone()),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 2]), range).unwrap();
        let spot = swatch(&mut d, "Spot", process(0.0, 1.0, 0.0, 0.0), true).unwrap();
        let rgb = swatch(&mut d, "Red", red.clone(), false).unwrap();
        let before = d.snapshot();
        cmyk(&mut d);
        let s = d.snapshot();
        assert_eq!((s.color_mode, s.raster_ppi), (ColorMode::Cmyk, 300.0));
        assert_eq!(
            s.pages[0].children[0].style.fills,
            [Fill::solid(
                Color::Rgb(0xe8452cff).convert(ColorMode::Cmyk).unwrap()
            )]
        );
        let color = |id: &str| {
            s.palette
                .swatches
                .iter()
                .find(|w| w.id == id)
                .unwrap()
                .color
                .clone()
        };
        assert_eq!(color(&spot), process(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Some(color(&rgb)), red.convert(ColorMode::Cmyk));
        assert!(!problems(&d).iter().any(|i| i.1 == Problem::Rgb));
        d.apply(Command::Undo).unwrap();
        let s = d.snapshot();
        assert_eq!((&s.pages, &s.palette), (&before.pages, &before.palette));
    }

    #[test]
    fn new_layers_in_a_cmyk_document_get_process_gray_black_and_white() {
        let (mut d, p) = empty();
        cmyk(&mut d);
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let t = create(&mut d, &p, NewKind::Text, [0.0; 4]);
        let l = create(&mut d, &p, NewKind::Line, [0.0; 4]);
        let f = create(&mut d, &p, NewKind::Frame, [0.0; 4]);
        let pg = page(&d);
        let style = |id: &str| {
            pg.children
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .style
                .clone()
        };
        assert_eq!(style(&r).fills, [Fill::solid(process(0.0, 0.0, 0.0, 0.15))]);
        assert_eq!(style(&t).fills, [Fill::solid(process(0.0, 0.0, 0.0, 1.0))]);
        assert_eq!(
            style(&l).strokes,
            [Fill::solid(process(0.0, 0.0, 0.0, 1.0))]
        );
        assert_eq!(style(&f).fills, [Fill::solid(process(0.0, 0.0, 0.0, 0.0))]);
    }

    #[test]
    fn cmyk_colours_render_through_the_fogra51_preview() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let cyan = Color::Cmyk {
            cmyk: [1.0, 0.0, 0.0, 0.0],
            alpha: 0.5,
        };
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![
                    Fill::solid(process(0.0, 0.0, 0.0, 0.0)),
                    Fill::solid(cyan),
                ]),
                ..Props::default()
            },
        );
        let colors: Vec<[f32; 4]> = page_ops(&d)
            .into_iter()
            .filter_map(|op| match op {
                Op::FillPath {
                    paint: Paint::Solid { color, .. },
                    ..
                } => Some(color),
                _ => None,
            })
            .collect();
        let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.03);
        assert!(near(colors[0], [1.0; 4]), "{colors:?}");
        assert!(near(colors[1], [0.0, 0.641, 0.896, 0.5]), "{colors:?}");
    }

    #[test]
    fn swatch_names_are_unique() {
        let mut d = Doc::sample();
        let red = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        swatch(&mut d, "Blue", Color::Rgb(0x0000ffff), false).unwrap();
        assert!(swatch(&mut d, "Red", process(0.0, 1.0, 1.0, 0.0), true).is_err());
        let rename = |name: &str| Command::SetSwatch {
            id: red.clone(),
            name: Some(name.into()),
            color: None,
            spot: None,
        };
        assert!(d.apply(rename("Blue")).is_err());
        d.apply(rename("Red")).unwrap();
        let names: Vec<_> = d
            .build_snapshot()
            .palette
            .swatches
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["Red", "Blue"]);
    }

    #[test]
    fn swatches_are_added_renamed_and_deleted_with_undo() {
        let mut d = Doc::sample();
        assert_eq!(d.snapshot().palette.swatches, []);
        let id = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        d.apply(Command::SetSwatch {
            id: id.clone(),
            name: Some("Signal".into()),
            color: None,
            spot: None,
        })
        .unwrap();
        assert_eq!(
            d.snapshot().palette.swatches,
            [Swatch {
                id: id.clone(),
                name: "Signal".into(),
                color: Color::Rgb(0xff0000ff),
                spot: false,
            }]
        );
        d.apply(Command::DeleteSwatch { id: id.clone() }).unwrap();
        assert_eq!(d.snapshot().palette.swatches, []);
        d.apply(Command::Undo).unwrap();
        assert_eq!(d.snapshot().palette.swatches[0].id, id);
    }

    #[test]
    fn a_fill_bound_to_a_swatch_follows_the_swatch() {
        let (mut d, p) = empty();
        let id = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(bound(&id, 1.0))]),
                ..Props::default()
            },
        );
        assert_eq!(solid_colors(&d), [[1.0, 0.0, 0.0, 1.0]]);
        d.apply(Command::SetSwatch {
            id,
            name: None,
            color: Some(Color::Rgb(0x0000ffff)),
            spot: None,
        })
        .unwrap();
        assert_eq!(solid_colors(&d), [[0.0, 0.0, 1.0, 1.0]]);
    }

    #[test]
    fn spot_swatches_have_a_cmyk_alternate_and_render_their_tint() {
        let (mut d, p) = empty();
        assert!(swatch(&mut d, "HKS 57", Color::Rgb(0xff00ffff), true).is_err());
        let id = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.0), true).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![
                    Fill::solid(bound(&id, 1.0)),
                    Fill::solid(bound(&id, 0.0)),
                ]),
                ..Props::default()
            },
        );
        let c = solid_colors(&d);
        assert!(near_rgba(c[0], [0.907, 0.0, 0.501, 1.0]), "{c:?}");
        assert!(near_rgba(c[1], [1.0; 4]), "{c:?}");
    }

    #[test]
    fn deleting_a_swatch_detaches_what_uses_it() {
        let (mut d, p) = empty();
        let red = swatch(&mut d, "Red", Color::Rgb(0xff0000ff), false).unwrap();
        let spot = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.2), true).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(Color::Swatch {
                    swatch: red.clone(),
                    tint: 1.0,
                    alpha: 0.5,
                })]),
                strokes: Some(vec![Fill::solid(bound(&spot, 0.5))]),
                ..Props::default()
            },
        );
        d.apply(Command::DeleteSwatch { id: red }).unwrap();
        d.apply(Command::DeleteSwatch { id: spot }).unwrap();
        let style = &page(&d).children[0].style;
        assert_eq!(style.fills, [Fill::solid(0xff000080)]);
        assert_eq!(
            style.strokes,
            [Fill::solid(Color::Cmyk {
                cmyk: [0.0, 0.5, 0.0, 0.1],
                alpha: 1.0
            })]
        );
    }

    #[test]
    fn colours_out_of_range_are_rejected() {
        let (mut d, p) = empty();
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        let fill = |c| Props {
            fills: Some(vec![Fill::solid(c)]),
            ..Props::default()
        };
        let set = |d: &mut Doc, props| {
            d.apply(Command::Set {
                id: r.clone(),
                props,
            })
        };
        assert!(set(&mut d, fill(process(0.0, 1.5, 0.0, 0.0))).is_err());
        assert!(set(&mut d, fill(bound("x", -0.5))).is_err());
        assert!(swatch(&mut d, "X", process(0.0, 0.0, 0.0, 2.0), false).is_err());
        assert!(set(&mut d, fill(process(0.0, 1.0, 0.0, 0.0))).is_ok());
    }

    #[test]
    fn a_fill_bound_to_a_colour_variable_takes_the_mode_of_its_frame_or_page() {
        let (mut d, p) = empty();
        let (c, light) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let bg = variable(&mut d, &c, "Bg", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        set_value(&mut d, &bg, &dark, Value::Color(Color::Rgb(0x0000ffff))).unwrap();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 10.0, 10.0]);
        let r = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(var(&bg))]),
                ..Props::default()
            },
        );
        let red = [1.0, 0.0, 0.0, 1.0];
        let blue = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(solid_colors(&d)[1], red);
        use_mode(&mut d, &p, &c, Some(&dark));
        assert_eq!(solid_colors(&d)[1], blue);
        use_mode(&mut d, &f, &c, Some(&light));
        assert_eq!(solid_colors(&d)[1], red);
        let s = d.snapshot();
        assert_eq!(s.pages[0].modes[&c], dark);
        let fr = &s.pages[0].children[0];
        assert_eq!(fr.modes[&c], light);
        assert_eq!(children(fr)[0].style.fills, [Fill::solid(var(&bg))]);
        use_mode(&mut d, &f, &c, None);
        assert_eq!(solid_colors(&d)[1], blue);
    }

    #[test]
    fn a_range_bound_to_a_colour_variable_takes_the_mode_of_the_page_it_shows_on() {
        let (mut d, p1) = empty();
        let p2 = add_page(&mut d, None);
        let (c, _) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let ink = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        set_value(&mut d, &ink, &dark, Value::Color(Color::Rgb(0x0000ffff))).unwrap();
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p2, [0.0, 0.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        let red = TextProps {
            fill: Some(var(&ink)),
            ..TextProps::default()
        };
        format(&mut d, &a, Some([0, 5]), red).unwrap();
        let m = add_master(&mut d);
        let t = create(&mut d, &m, NewKind::Text, [0.0, 200.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Yo");
        let red = TextProps {
            fill: Some(var(&ink)),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 2]), red).unwrap();
        use_master(&mut d, &p2, Some(&m)).unwrap();
        use_mode(&mut d, &p2, &c, Some(&dark));
        let glyphs = |d: &Doc, p: &str| {
            d.render(p)
                .into_iter()
                .filter_map(|o| match o {
                    Op::GlyphRun {
                        paint: Paint::Solid { color, .. },
                        ..
                    } => Some(color),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(glyphs(&d, &p1), [[1.0, 0.0, 0.0, 1.0]]);
        assert_eq!(glyphs(&d, &p2), [[0.0, 0.0, 1.0, 1.0]; 2]);
    }

    #[test]
    fn bound_numbers_follow_their_variable_in_mm_and_percent_until_set_directly() {
        let (mut d, p) = empty();
        let (c, _) = collection(&mut d, "Sizes");
        let big = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Big".into(),
            })
            .unwrap()
            .remove(0);
        let size = variable(&mut d, &c, "Size", Value::Number(10.0)).unwrap();
        let half = variable(&mut d, &c, "Half", Value::Number(50.0)).unwrap();
        set_value(&mut d, &size, &big, Value::Number(20.0)).unwrap();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 100.0]);
        let r = create(&mut d, &f, NewKind::Rect, [5.0, 5.0, 1.0, 1.0]);
        for prop in ["w", "radius", "strokeWeight"] {
            bind(&mut d, &r, prop, Some(&size)).unwrap();
        }
        bind(&mut d, &r, "opacity", Some(&half)).unwrap();
        let rect = |d: &Doc| {
            let pg = page(d);
            let n = &children(&pg.children[0])[0];
            let radius = match n.kind {
                Kind::Shape(Shape::Rect { radius, .. }) => radius,
                _ => -1.0,
            };
            (n.w, radius, n.style.stroke_weight, n.style.opacity)
        };
        let mm = |v: f64| v * MM;
        assert_eq!(rect(&d), (mm(10.0), mm(10.0) as f32, mm(10.0) as f32, 0.5));
        assert_eq!(page(&d).children[0].bindings, Default::default());
        assert_eq!(children(&page(&d).children[0])[0].bindings["w"], size);
        use_mode(&mut d, &f, &c, Some(&big));
        assert_eq!(rect(&d).0, mm(20.0));
        d.apply(Command::Undo).unwrap();
        assert_eq!(rect(&d).0, mm(10.0));
        set(
            &mut d,
            &r,
            Props {
                radius: Some(1.0),
                ..Props::default()
            },
        );
        d.apply(Command::SetFrame {
            id: r.clone(),
            x: 5.0,
            y: 5.0,
            w: 3.0,
            h: 1.0,
            crop: false,
        })
        .unwrap();
        set_value(&mut d, &size, &big, Value::Number(30.0)).unwrap();
        use_mode(&mut d, &f, &c, Some(&big));
        assert_eq!(rect(&d), (3.0, 1.0, mm(30.0) as f32, 0.5));
        let pg = page(&d);
        let bindings = &children(&pg.children[0])[0].bindings;
        assert_eq!(
            bindings.keys().collect::<Vec<_>>(),
            ["opacity", "strokeWeight"]
        );
        bind(&mut d, &r, "strokeWeight", None).unwrap();
        assert!(bind(&mut d, &r, "name", Some(&size)).is_err());
        let color = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0))).unwrap();
        assert!(bind(&mut d, &r, "w", Some(&color)).is_err());
    }

    #[test]
    fn collections_modes_and_variables_are_renamed_and_deleted_and_users_keep_their_values() {
        let (mut d, p) = empty();
        let (c, first) = collection(&mut d, "Theme");
        let dark = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Dark".into(),
            })
            .unwrap()
            .remove(0);
        let spot = swatch(&mut d, "HKS 57", process(0.0, 1.0, 0.0, 0.0), true).unwrap();
        let ink = variable(&mut d, &c, "Ink", Value::Color(Color::Rgb(0xff0000ff))).unwrap();
        let size = variable(&mut d, &c, "Size", Value::Number(2.0)).unwrap();
        assert!(variable(&mut d, &c, "Ink", Value::Number(1.0)).is_err());
        assert!(set_value(&mut d, &ink, &dark, Value::Number(1.0)).is_err());
        assert!(set_value(&mut d, &ink, &dark, Value::Color(var(&ink))).is_err());
        set_value(&mut d, &ink, &dark, Value::Color(bound(&spot, 0.5))).unwrap();
        d.apply(Command::SetCollection {
            id: c.clone(),
            name: "Brand".into(),
        })
        .unwrap();
        d.apply(Command::SetMode {
            collection: c.clone(),
            id: dark.clone(),
            name: "Night".into(),
        })
        .unwrap();
        let s = d.snapshot();
        assert_eq!(s.palette.collections[0].name, "Brand");
        assert_eq!(s.palette.collections[0].modes[1].name, "Night");

        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(Color::Variable {
                    variable: ink.clone(),
                    alpha: 0.5,
                })]),
                ..Props::default()
            },
        );
        bind(&mut d, &r, "radius", Some(&size)).unwrap();
        use_mode(&mut d, &r, &c, Some(&dark));
        d.apply(Command::DeleteVariable { id: ink.clone() })
            .unwrap();
        d.apply(Command::DeleteVariable { id: size.clone() })
            .unwrap();
        let n = &page(&d).children[0];
        assert_eq!(
            n.style.fills,
            [Fill::solid(Color::Swatch {
                swatch: spot.clone(),
                tint: 0.5,
                alpha: 0.5
            })]
        );
        assert!(n.bindings.is_empty());
        assert_eq!(
            n.kind,
            Kind::Shape(Shape::Rect {
                radius: 2.0 * MM as f32,
                corners: vec![]
            })
        );

        let ink = variable(&mut d, &c, "Ink", Value::Color(bound(&spot, 1.0))).unwrap();
        d.apply(Command::DeleteSwatch { id: spot }).unwrap();
        assert_eq!(
            d.snapshot().palette.variables[0].values[&first],
            Value::Color(process(0.0, 1.0, 0.0, 0.0))
        );
        use_mode(&mut d, &p, &c, Some(&first));
        assert!(
            d.apply(Command::DeleteMode {
                collection: c.clone(),
                id: first.clone(),
            })
            .is_ok()
        );
        assert_eq!(page(&d).modes, Modes::new());
        assert_eq!(
            d.snapshot().palette.variables[0]
                .values
                .keys()
                .collect::<Vec<_>>(),
            [&dark]
        );
        assert!(
            d.apply(Command::DeleteMode {
                collection: c.clone(),
                id: dark,
            })
            .is_err()
        );
        set(
            &mut d,
            &r,
            Props {
                fills: Some(vec![Fill::solid(var(&ink))]),
                ..Props::default()
            },
        );
        d.apply(Command::DeleteCollection { id: c }).unwrap();
        let s = d.snapshot();
        assert_eq!(s.pages[0].children[0].modes, Modes::new());
        assert!(s.palette.collections.is_empty() && s.palette.variables.is_empty());
        assert_eq!(
            s.pages[0].children[0].style.fills,
            [Fill::solid(process(0.0, 1.0, 0.0, 0.0))]
        );
    }
}
