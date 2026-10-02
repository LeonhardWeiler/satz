use super::*;
use crate::display_list::{CLOSE, CUBIC};
use kurbo::{BezPath, PathEl, Point, Shape as _};
use linesweeper::topology::{Contour, Contours};
use linesweeper::{BinaryOp, FillRule, binary_op};

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// `path` with every subpath closed.
fn bez(path: &[f32]) -> BezPath {
    let at = |p: &[f32], i: usize| Point::new(p[i].into(), p[i + 1].into());
    let mut b = BezPath::new();
    let mut open = false;
    for (v, p) in geom::segments(path) {
        match v {
            MOVE => {
                if open {
                    b.close_path();
                }
                b.move_to(at(p, 0));
                open = true;
            }
            LINE => b.line_to(at(p, 0)),
            CUBIC => b.curve_to(at(p, 0), at(p, 2), at(p, 4)),
            _ => {
                b.close_path();
                open = false;
            }
        }
    }
    if open {
        b.close_path();
    }
    b
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

/// The closed subpaths of `path`.
fn closed(path: &[f32]) -> Vec<BezPath> {
    let mut out = Vec::new();
    let mut sub = Vec::new();
    for (v, p) in geom::segments(path) {
        if v == MOVE {
            sub.clear();
        }
        sub.push(v);
        sub.extend(p);
        if v == CLOSE {
            out.push(bez(&sub));
        }
    }
    out
}

/// The area that the closed subpaths of `path` bound around `at`, with its holes.
fn area(path: &[f32], at: Point) -> Res<BezPath> {
    let subs = closed(path);
    let (ins, outs): (Vec<_>, Vec<_>) = subs.iter().partition(|s| s.winding(at) != 0);
    let [first, ins @ ..] = &ins[..] else {
        return Err("no area there".into());
    };
    let mut c = combine(first, &BezPath::new(), BinaryOp::Union)?;
    for s in ins {
        c = combine(&flat(c.contours()), s, BinaryOp::Intersection)?;
    }
    for s in outs {
        c = combine(&flat(c.contours()), s, BinaryOp::Difference)?;
    }
    let face = c
        .grouped()
        .into_iter()
        .flatten()
        .filter(|&i| c[i].outer && c[i].path.winding(at) != 0)
        .min_by(|&a, &b| c[a].path.area().abs().total_cmp(&c[b].path.area().abs()))
        .ok_or("no area there")?;
    Ok(flat(c.contours().filter(|k| {
        std::ptr::eq(*k, &c[face]) || k.parent == Some(face)
    })))
}

impl Doc {
    pub(super) fn fill_area(&self, id: String, x: f64, y: f64) -> Res<Vec<String>> {
        let id = self.layer(&id)?;
        if self.kind(id) != Some(NodeKind::Shape) {
            return Err("only shapes have areas".into());
        }
        if !x.is_finite() || !y.is_finite() {
            return Err("not a point".into());
        }
        let parent = self.tree.parent(id).ok_or("no parent")?;
        let p = parent_node(parent).ok_or("no parent")?;
        let turn = self.turn(p);
        let to_page = |[x, y]: [f32; 2]| geom::apply(turn, [x.into(), y.into()]).map(|v| v as f32);
        let out = area(&geom::map(&self.outline_of(id)?, to_page), Point::new(x, y))?;
        let back = geom::invert(turn);
        let out = geom::map(&path(&out), |[x, y]| {
            geom::apply(back, [x.into(), y.into()]).map(|v| v as f32)
        });
        let fills: Vec<Fill> = value(&self.meta(id), "fills")
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
            .unwrap_or_default();
        let new = self.create(&p.to_string(), NewKind::Rect, bounds(&out).map(f64::from))?;
        self.make_path(new, &out)?;
        if !fills.is_empty() {
            self.set(
                new,
                Props {
                    fills: Some(fills),
                    ..Props::default()
                },
            )?;
        }
        self.tree.mov_to(new, parent, self.index(id)).map_err(err)?;
        Ok(vec![new.to_string()])
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
    fn a_bucket_fills_the_area_around_a_point_below_the_path() {
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
        ]
        .concat();
        d.apply(Command::SetPath {
            id: v.clone(),
            path,
        })
        .unwrap();
        let mut fill = |x, y| {
            let out = d.apply(Command::FillArea {
                id: v.clone(),
                x,
                y,
            });
            let s = d.snapshot();
            let kids = &s.pages[0].children;
            out.map(|out| {
                assert_eq!(ids(kids), [out[0].clone(), v.clone()]);
                let f = frame(&kids[0]);
                d.apply(Command::Delete { ids: out }).unwrap();
                f
            })
        };
        assert_eq!(fill(15.0, 15.0).unwrap(), [10.0, 10.0, 10.0, 10.0]);
        assert_eq!(fill(5.0, 5.0).unwrap(), [0.0, 0.0, 20.0, 20.0]);
        assert_eq!(fill(25.0, 25.0).unwrap(), [10.0, 10.0, 20.0, 20.0]);
        assert!(fill(50.0, 50.0).is_err());
    }
}
