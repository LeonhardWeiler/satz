use super::*;
use crate::display_list::{CLOSE, CUBIC};
use kurbo::{BezPath, ParamCurve, PathEl, Point, Shape as _, Vec2};
use linesweeper::topology::{Contour, Contours, Topology, WindingNumber};
use linesweeper::{BinaryOp, FillRule, binary_op};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// The subpaths of `path`, open ones left open.
fn subpaths(path: &[f32]) -> Vec<BezPath> {
    let at = |p: &[f32], i: usize| Point::new(p[i].into(), p[i + 1].into());
    let mut out: Vec<BezPath> = Vec::new();
    for (v, p) in geom::segments(path) {
        if v == MOVE {
            out.push(BezPath::new());
        }
        let Some(b) = out.last_mut() else { continue };
        match v {
            MOVE => b.move_to(at(p, 0)),
            LINE => b.line_to(at(p, 0)),
            CUBIC => b.curve_to(at(p, 0), at(p, 2), at(p, 4)),
            _ => b.close_path(),
        }
    }
    out
}

fn is_closed(b: &BezPath) -> bool {
    matches!(b.elements().last(), Some(PathEl::ClosePath))
}

/// `path` with every subpath closed.
fn bez(path: &[f32]) -> BezPath {
    subpaths(path)
        .into_iter()
        .flat_map(|mut b| {
            if !is_closed(&b) {
                b.close_path();
            }
            b.into_elements()
        })
        .collect()
}

fn path(b: &BezPath) -> Vec<f32> {
    let xy = |p: Point| [p.x as f32, p.y as f32];
    let mut out = Vec::new();
    let mut last = Point::ZERO;
    for el in b.elements() {
        match *el {
            PathEl::MoveTo(p) => out.extend([[MOVE].as_slice(), &xy(p)].concat()),
            PathEl::LineTo(p) => out.extend([[LINE].as_slice(), &xy(p)].concat()),
            PathEl::QuadTo(c, p) => {
                let a = last + (c - last) * (2.0 / 3.0);
                let b = p + (c - p) * (2.0 / 3.0);
                out.extend([[CUBIC].as_slice(), &xy(a), &xy(b), &xy(p)].concat());
            }
            PathEl::CurveTo(a, b, p) => {
                out.extend([[CUBIC].as_slice(), &xy(a), &xy(b), &xy(p)].concat())
            }
            PathEl::ClosePath => out.push(CLOSE),
        }
        last = el.end_point().unwrap_or(last);
    }
    out
}

fn combine(a: &BezPath, b: &BezPath, op: BinaryOp) -> Res<Contours> {
    binary_op(a, b, FillRule::NonZero, op).map_err(err)
}

fn flat<'a>(c: impl Iterator<Item = &'a Contour>) -> BezPath {
    c.flat_map(|c| c.path.elements().to_vec()).collect()
}

/// A winding number that every line raises, so that a line run there and back stays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Lines(u32);

impl std::ops::Add for Lines {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Lines(self.0.wrapping_add(o.0))
    }
}

impl std::ops::AddAssign for Lines {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o;
    }
}

impl WindingNumber for Lines {
    type Tag = ();
    fn single((): (), _: bool) -> Self {
        Lines(1)
    }
    fn of_tag(&self, (): ()) -> Self {
        *self
    }
}

/// The boundaries of the faces that the lines of `path` cut the plane into, crossings
/// included; each comes with a point just inside its face.
fn faces(path: &[f32]) -> Res<Vec<(BezPath, Point)>> {
    let subs: Vec<BezPath> = subpaths(path)
        .into_iter()
        .map(|mut b| {
            if !is_closed(&b) {
                let back = b.reverse_subpaths();
                b.extend(back.elements()[1..].iter().copied());
                b.close_path();
            }
            b
        })
        .collect();
    let t = Topology::<Lines>::from_paths(subs.iter().map(|s| (s, ())), 1e-6).map_err(err)?;
    let pos = t.compute_positions();
    let ends = |b: &BezPath| {
        let s = b.segments();
        let (first, last) = (s.clone().next(), s.last());
        first.zip(last).map(|(f, l)| (f, l.end()))
    };
    // The sweep joins points that nearly meet with segments too short to have a
    // direction; their ends count as one point.
    let mut same = HashMap::new();
    let key = |same: &HashMap<_, _>, p: Point| {
        let mut k = (p.x.to_bits(), p.y.to_bits());
        while let Some(&n) = same.get(&k) {
            k = n;
        }
        k
    };
    let mut edges = Vec::new();
    for i in t.segment_indices() {
        let p = &pos[i].path;
        let Some((f, end)) = ends(p) else { continue };
        if (end - f.start()).hypot() < 1e-9 {
            let (a, b) = (key(&same, f.start()), key(&same, end));
            if a != b {
                same.insert(a, b);
            }
            continue;
        }
        edges.extend([p.clone(), p.reverse_subpaths()]);
    }
    let mut out: HashMap<_, Vec<(f64, usize)>> = HashMap::new();
    for (h, e) in edges.iter().enumerate() {
        let Some((f, _)) = ends(e) else { continue };
        let d = f.eval(0.01) - f.start();
        out.entry(key(&same, f.start()))
            .or_default()
            .push((d.y.atan2(d.x), h));
    }
    for v in out.values_mut() {
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let next = |h: usize| {
        let (_, end) = ends(&edges[h])?;
        let v = out.get(&key(&same, end))?;
        let i = v.iter().position(|&(_, e)| e == h ^ 1)?;
        Some(v[(i + v.len() - 1) % v.len()].1)
    };
    let mut seen = vec![false; edges.len()];
    let mut faces = Vec::new();
    for h in 0..edges.len() {
        let mut b = BezPath::new();
        let mut e = h;
        while !seen[e] {
            seen[e] = true;
            let els = edges[e].elements();
            if b.is_empty() {
                b.push(els[0]);
            }
            b.extend(els[1..].iter().copied());
            let Some(n) = next(e) else { break };
            e = n;
        }
        let Some((f, _)) = ends(&edges[h]).filter(|_| !b.is_empty()) else {
            continue;
        };
        b.close_path();
        let d = f.eval(0.51) - f.eval(0.49);
        let inside = f.eval(0.5) + Vec2::new(-d.y, d.x).normalize() * 1e-3;
        faces.push((b, inside));
    }
    Ok(faces)
}

/// The area that the lines of `faces` enclose around `at`, with its holes.
fn area(faces: &[(BezPath, Point)], at: Point) -> Res<BezPath> {
    let up = |b: &BezPath| {
        if b.area() < 0.0 {
            b.reverse_subpaths()
        } else {
            b.clone()
        }
    };
    let (ins, outs): (Vec<_>, Vec<_>) = faces
        .iter()
        .map(|f| &f.0)
        .filter(|b| b.area().abs() > 1e-9)
        .partition(|b| b.winding(at) != 0);
    let face = ins
        .into_iter()
        .min_by(|a, b| a.area().abs().total_cmp(&b.area().abs()))
        .ok_or("no area there")?;
    let holes: BezPath = outs.iter().flat_map(|b| up(b).into_elements()).collect();
    let c = combine(&up(face), &holes, BinaryOp::Difference)?;
    let out = flat(c.contours());
    if out.is_empty() {
        return Err("no area there".into());
    }
    Ok(out)
}

/// The areas of `path` around the points `at` that lie in one.
pub(super) fn areas(lines: &[f32], at: &[f32]) -> Res<Vec<f32>> {
    let faces = faces(lines)?;
    let mut out = Vec::new();
    for (_, p) in geom::segments(at) {
        if let Ok(a) = area(&faces, Point::new(p[0].into(), p[1].into())) {
            out.extend(path(&a));
        }
    }
    Ok(out)
}

/// The points `seeds` moved from the lines `old` to `new`: each into the area of
/// `new` that overlaps its area most, or left where it is.
pub(super) fn follow(old: &[f32], new: &[f32], seeds: &[f32]) -> Vec<f32> {
    let (Ok(was), Ok(now)) = (faces(old), faces(new)) else {
        return seeds.to_vec();
    };
    let mut found: Vec<(Point, BezPath)> = Vec::new();
    for &(_, p) in &now {
        if !found.iter().any(|(_, a)| a.winding(p) != 0)
            && let Ok(a) = area(&now, p)
        {
            found.push((p, a));
        }
    }
    let overlap = |a: &BezPath, b: &BezPath| {
        combine(a, b, BinaryOp::Intersection).map_or(0.0, |c| flat(c.contours()).area().abs())
    };
    let mut out = Vec::new();
    for (_, s) in geom::segments(seeds) {
        let seed = Point::new(s[0].into(), s[1].into());
        let to = area(&was, seed).ok().and_then(|a| {
            let near = found
                .iter()
                .filter(|(_, b)| b.bounding_box().overlaps(a.bounding_box()));
            near.map(|(p, b)| (overlap(&a, b), *p, b))
                .filter(|o| o.0 > 0.0)
                .max_by(|x, y| x.0.total_cmp(&y.0))
                .map(|(_, p, b)| if b.winding(seed) != 0 { seed } else { p })
        });
        let p = to.unwrap_or(seed);
        out.extend([MOVE, p.x as f32, p.y as f32]);
    }
    out
}

impl Doc {
    /// The lines of the path `id` in its frame, unturned.
    pub(super) fn lines_of(&self, id: TreeID) -> Res<Vec<f32>> {
        let unturn = geom::invert(geom::rotation(
            num(&self.meta(id), "rotation"),
            self.bounds(id),
        ));
        Ok(geom::map(&self.outline_of(id)?, |q| {
            geom::apply(unturn, q.map(f64::from)).map(|v| v as f32)
        }))
    }

    /// Fills the area of the path `id` around [x, y] in its parent's space, or empties it
    /// when it is filled.
    pub(super) fn fill_area(&self, id: String, x: f64, y: f64) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        let m = self.meta(id);
        if value(&m, "shape") != Some("path".into()) {
            return Err("only paths have areas".into());
        }
        if !x.is_finite() || !y.is_finite() {
            return Err("not a point".into());
        }
        let frame = self.bounds(id);
        let parent = parent_node(self.tree.parent(id).ok_or("no parent")?).ok_or("no parent")?;
        let unturn = geom::invert(geom::rotation(num(&m, "rotation"), frame));
        let lines = self.lines_of(id)?;
        let [x, y] = geom::apply(unturn, geom::apply(geom::invert(self.turn(parent)), [x, y]));
        let [l, t, w, h] = frame.map(|v| v as f32);
        let faces = faces(&lines)?;
        let at = Point::new(x, y);
        let here = area(&faces, at)?;
        let fills: Vec<Fill> = self.json(id, "fills");
        let saved: Vec<f32> = self.json(id, "areas");
        let mut seeds: Vec<Point> = geom::segments(&saved)
            .map(|(_, p)| p)
            .map(|p| Point::new((l + p[0] * w).into(), (t + p[1] * h).into()))
            .collect();
        if seeds.is_empty() && fills.iter().any(|f| f.visible) {
            let all = bez(&lines);
            for &(_, p) in &faces {
                if all.winding(p) == 0
                    || seeds
                        .iter()
                        .any(|s| area(&faces, *s).is_ok_and(|a| a.winding(p) != 0))
                {
                    continue;
                }
                if area(&faces, p).is_ok() {
                    seeds.push(p);
                }
            }
        }
        match seeds.iter().position(|s| here.winding(*s) != 0) {
            Some(i) => _ = seeds.remove(i),
            None => seeds.push(at),
        }
        let scale = |v: f64, from: f32, size: f32| {
            if size > 0.0 {
                (v as f32 - from) / size
            } else {
                0.0
            }
        };
        let areas: Vec<f32> = seeds
            .iter()
            .flat_map(|p| [MOVE, scale(p.x, l, w), scale(p.y, t, h)])
            .collect();
        let fills = if areas.is_empty() {
            Vec::new()
        } else if fills.iter().any(|f| f.visible) {
            fills
        } else {
            vec![Fill::solid(Color::gray(self.color_mode()))]
        };
        self.set(
            id,
            Props {
                fills: Some(fills),
                ..Props::default()
            },
        )?;
        m.insert("areas", loro(serde_json::to_value(areas).map_err(err)?)?)
            .map_err(err)?;
        Ok(vec![])
    }

    pub(super) fn boolean(&self, ids: Vec<String>, op: BooleanOp) -> Res<Vec<String>> {
        let ids = self.sorted(&ids)?;
        let [bottom, rest @ ..] = &ids[..] else {
            return Err("nothing to combine".into());
        };
        if rest.is_empty() {
            return Err("a boolean needs two shapes or more".into());
        }
        for &id in &ids {
            self.layer(&id.to_string())?;
            if self.kind(id) != Some(NodeKind::Shape) {
                return Err("only shapes combine".into());
            }
            if self.root(id) != self.root(*bottom) {
                return Err("the shapes are on different pages".into());
            }
        }
        let parent = |id: TreeID| {
            self.tree
                .parent(id)
                .and_then(parent_node)
                .ok_or("no parent")
        };
        let mut shapes = Vec::new();
        for &id in &ids {
            let turn = self.turn(parent(id)?);
            let to_page =
                |[x, y]: [f32; 2]| geom::apply(turn, [x.into(), y.into()]).map(|v| v as f32);
            shapes.push(bez(&geom::map(&self.outline_of(id)?, to_page)));
        }
        let op = match op {
            BooleanOp::Union => BinaryOp::Union,
            BooleanOp::Subtract => BinaryOp::Difference,
            BooleanOp::Intersect => BinaryOp::Intersection,
            BooleanOp::Exclude => BinaryOp::Xor,
        };
        let mut out = shapes[0].clone();
        for s in &shapes[1..] {
            out = flat(combine(&out, s, op)?.contours());
        }
        if out.is_empty() {
            return Err("the shapes leave nothing".into());
        }
        let back = geom::invert(self.turn(parent(*bottom)?));
        let out = geom::map(&path(&out), |[x, y]| {
            geom::apply(back, [x.into(), y.into()]).map(|v| v as f32)
        });
        self.make_path(*bottom, &out)?;
        self.delete(rest.iter().map(|id| id.to_string()).collect())?;
        Ok(vec![bottom.to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

    #[test]
    fn booleans_combine_shapes_into_the_bottommost_and_delete_the_rest() {
        let (mut d, p) = empty();
        let combine = |d: &mut Doc, op| {
            let a = create(d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
            let b = create(d, &p, NewKind::Ellipse, [10.0, 10.0, 20.0, 20.0]);
            let out = d.apply(Command::Boolean {
                ids: vec![b.clone(), a.clone()],
                op,
            });
            (a, out)
        };
        for (op, inside, outside) in [
            (BooleanOp::Union, [25.0, 20.0], [25.0, 5.0]),
            (BooleanOp::Subtract, [5.0, 5.0], [15.0, 15.0]),
            (BooleanOp::Intersect, [15.0, 15.0], [5.0, 5.0]),
            (BooleanOp::Exclude, [25.0, 20.0], [15.0, 15.0]),
        ] {
            let (a, out) = combine(&mut d, op);
            assert_eq!(out.unwrap(), std::slice::from_ref(&a));
            let s = d.snapshot();
            let kids = &s.pages[0].children;
            assert_eq!(ids(kids), std::slice::from_ref(&a));
            assert!(matches!(&kids[0].kind, Kind::Shape(Shape::Path { .. })));
            assert_eq!(
                hits(&d, inside[0], inside[1], 0.0),
                std::slice::from_ref(&a),
                "{op:?}"
            );
            assert!(hits(&d, outside[0], outside[1], 0.0).is_empty(), "{op:?}");
            d.apply(Command::Delete { ids: vec![a] }).unwrap();
        }
        let far = create(&mut d, &p, NewKind::Rect, [100.0, 100.0, 5.0, 5.0]);
        let near = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 5.0, 5.0]);
        let disjoint = Command::Boolean {
            ids: vec![far.clone(), near],
            op: BooleanOp::Intersect,
        };
        assert!(d.apply(disjoint).is_err());
        let t = text(&mut d, "Hi");
        let with_text = Command::Boolean {
            ids: vec![far, t],
            op: BooleanOp::Union,
        };
        assert!(d.apply(with_text).is_err());
    }

    #[test]
    fn a_bucket_fills_and_empties_the_areas_that_lines_enclose_in_the_path() {
        let (mut d, p) = empty();
        let v = create(&mut d, &p, NewKind::Path, [0.0, 0.0, 1.0, 1.0]);
        let square = |x: f32| {
            [
                MOVE,
                x,
                x,
                LINE,
                x + 20.0,
                x,
                LINE,
                x + 20.0,
                x + 20.0,
                LINE,
                x,
                x + 20.0,
                CLOSE,
            ]
        };
        let path = [
            square(0.0).as_slice(),
            &square(10.0),
            &[MOVE, 0.0, 40.0, LINE, 40.0, 0.0],
            &[MOVE, 50.0, 0.0, LINE, 50.0, 30.0],
            &[MOVE, 45.0, 5.0, LINE, 70.0, 5.0],
            &[MOVE, 45.0, 15.0, LINE, 60.0, 0.0],
        ]
        .concat();
        d.apply(Command::SetPath {
            id: v.clone(),
            path,
        })
        .unwrap();
        let mut fill = |x, y| {
            d.apply(Command::FillArea {
                id: v.clone(),
                x,
                y,
            })?;
            assert_eq!(d.snapshot().pages[0].children.len(), 1);
            Ok::<_, String>(
                page_ops(&d)
                    .iter()
                    .filter_map(|o| match o {
                        Op::FillPath { path, .. } => Some(bounds(path).map(|v| v.round())),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(fill(15.0, 15.0).unwrap(), [[10.0, 10.0, 10.0, 10.0]]);
        assert_eq!(fill(25.0, 25.0).unwrap(), [[10.0, 10.0, 20.0, 20.0]]);
        assert_eq!(fill(15.0, 15.0).unwrap(), [[10.0, 10.0, 20.0, 20.0]]);
        assert_eq!(fill(25.0, 25.0).unwrap(), Vec::<[f32; 4]>::new());
        assert_eq!(fill(5.0, 5.0).unwrap(), [[0.0, 0.0, 20.0, 20.0]]);
        assert_eq!(fill(51.0, 6.0).unwrap(), [[0.0, 0.0, 55.0, 20.0]]);
        assert!(fill(80.0, 80.0).is_err());
    }

    #[test]
    fn the_area_is_the_face_the_lines_bound_where_they_cross_at_nearly_one_point() {
        let at = |lines: &[f32], x: f32, y: f32| {
            areas(lines, &[MOVE, x, y]).map(|a| bounds(&a).map(|v| (v * 10.0).round() / 10.0))
        };
        let star = [
            MOVE, 50.0, 0.0, LINE, 79.0, 90.0, LINE, 2.0, 35.0, LINE, 98.0, 35.0, LINE, 21.0, 90.0,
            CLOSE,
        ];
        assert_eq!(at(&star, 50.0, 50.0), Ok([31.9, 35.0, 36.3, 34.3]));
        let curve = [
            MOVE, 0.0, 0.0, CUBIC, 30.0, -10.0, 60.0, 50.0, 0.0, 40.0, MOVE, 5.0, -10.0, LINE, 5.0,
            60.0,
        ];
        assert_eq!(at(&curve, 15.0, 20.0).unwrap()[0], 5.0);
    }

    #[test]
    fn filled_areas_stay_while_a_point_of_the_lines_moves() {
        let (mut d, p) = empty();
        let v = create(&mut d, &p, NewKind::Path, [0.0, 0.0, 1.0, 1.0]);
        let shape = |d: &mut Doc, k: f32| {
            let path = vec![
                MOVE,
                0.0,
                0.0,
                LINE,
                100.0,
                0.0,
                LINE,
                100.0 + k,
                100.0,
                LINE,
                0.0,
                100.0,
                CLOSE,
                MOVE,
                -10.0,
                37.3,
                LINE,
                110.0 + k * 0.7,
                61.1,
                MOVE,
                33.3,
                -5.0,
                LINE,
                70.0,
                105.0,
            ];
            d.apply(Command::SetPath {
                id: v.clone(),
                path,
            })
            .unwrap();
        };
        shape(&mut d, 0.0);
        for [x, y] in [[20.0, 20.0], [80.0, 80.0]] {
            d.apply(Command::FillArea {
                id: v.clone(),
                x,
                y,
            })
            .unwrap();
        }
        for i in 0..100 {
            shape(&mut d, (i as f32 * 0.37).sin() * 40.0);
            let areas: usize = page_ops(&d)
                .iter()
                .map(|o| match o {
                    Op::FillPath { path, .. } => {
                        geom::segments(path).filter(|s| s.0 == MOVE).count()
                    }
                    _ => 0,
                })
                .sum();
            assert_eq!(areas, 2, "step {i}");
        }
    }

    #[test]
    fn a_bucket_on_a_filled_path_empties_only_the_area_it_hits() {
        let (mut d, p) = empty();
        let v = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 20.0, 20.0]);
        d.apply(Command::Flatten { id: v.clone() }).unwrap();
        let s = d.snapshot();
        let Kind::Shape(Shape::Path { path, .. }) = &s.pages[0].children[0].kind else {
            panic!("a flattened rect is a path");
        };
        d.apply(Command::SetPath {
            id: v.clone(),
            path: [
                geom::fit(path, [0.0, 0.0, 20.0, 20.0]).as_slice(),
                &[MOVE, 10.0, 0.0, LINE, 10.0, 20.0],
            ]
            .concat(),
        })
        .unwrap();
        d.apply(Command::FillArea {
            id: v.clone(),
            x: 5.0,
            y: 5.0,
        })
        .unwrap();
        let fills = |d: &Doc| {
            page_ops(d)
                .iter()
                .filter_map(|o| match o {
                    Op::FillPath { path, .. } => Some(bounds(path).map(|v| v.round())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(fills(&d), [[10.0, 0.0, 10.0, 20.0]]);
        d.apply(Command::Flip {
            id: v.clone(),
            vertical: false,
        })
        .unwrap();
        assert_eq!(fills(&d), [[0.0, 0.0, 10.0, 20.0]]);
        d.apply(Command::SetPath {
            id: v,
            path: vec![
                MOVE, 0.0, 0.0, LINE, 40.0, 0.0, LINE, 40.0, 20.0, LINE, 0.0, 20.0, CLOSE, MOVE,
                10.0, 0.0, LINE, 10.0, 20.0,
            ],
        })
        .unwrap();
        assert_eq!(fills(&d), [[0.0, 0.0, 10.0, 20.0]]);
    }
}
