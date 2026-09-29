use crate::display_list::{CLOSE, CUBIC, LINE, MOVE, rect};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "camelCase")]
pub enum Shape {
    Rect {
        #[serde(default)]
        radius: f32,
        /// Top left, top right, bottom right and bottom left; empty for `radius` at each.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        corners: Vec<f32>,
    },
    /// An arc from `start` degrees clockwise from the right over `sweep` of the full turn,
    /// hollowed to `inner` of its radius.
    Ellipse {
        #[serde(default)]
        start: f32,
        #[serde(default = "one")]
        sweep: f32,
        #[serde(default)]
        inner: f32,
    },
    Polygon {
        count: u32,
    },
    Star {
        count: u32,
        ratio: f32,
    },
    Path {
        path: Vec<f32>,
    },
}

const KAPPA: f32 = 0.552_284_8;

fn one() -> f32 {
    1.0
}
const FLATNESS: usize = 16;

/// The transform [a b c d e f] that turns `frame` by `degrees` counterclockwise around its centre.
pub fn rotation(degrees: f64, [x, y, w, h]: [f64; 4]) -> [f64; 6] {
    let (s, c) = degrees.to_radians().sin_cos();
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    [c, -s, s, c, cx - c * cx - s * cy, cy + s * cx - c * cy]
}

/// The transform that applies `n`, then `m`.
pub fn then([a, b, c, d, e, f]: [f64; 6], m: [f64; 6]) -> [f64; 6] {
    let [p, q, r, s, t, u] = m;
    [
        p * a + r * b,
        q * a + s * b,
        p * c + r * d,
        q * c + s * d,
        p * e + r * f + t,
        q * e + s * f + u,
    ]
}

/// The transform that undoes `m`.
pub fn invert([a, b, c, d, e, f]: [f64; 6]) -> [f64; 6] {
    let det = a * d - b * c;
    let [a, b, c, d] = [d / det, -b / det, -c / det, a / det];
    [a, b, c, d, -(a * e + c * f), -(b * e + d * f)]
}

pub fn apply([a, b, c, d, e, f]: [f64; 6], [x, y]: [f64; 2]) -> [f64; 2] {
    [a * x + c * y + e, b * x + d * y + f]
}

pub fn outline(shape: &Shape, [x, y, w, h]: [f32; 4]) -> Vec<f32> {
    match shape {
        Shape::Rect { radius, corners } => {
            let [a, b, c, d] =
                [0, 1, 2, 3].map(|i| corners.get(i).unwrap_or(radius).min(w / 2.0).min(h / 2.0));
            if a + b + c + d <= 0.0 {
                return rect(x, y, w, h);
            }
            let k = 1.0 - KAPPA;
            let (r0, b0) = (x + w, y + h);
            vec![
                MOVE,
                x + a,
                y,
                LINE,
                r0 - b,
                y,
                CUBIC,
                r0 - b * k,
                y,
                r0,
                y + b * k,
                r0,
                y + b,
                LINE,
                r0,
                b0 - c,
                CUBIC,
                r0,
                b0 - c * k,
                r0 - c * k,
                b0,
                r0 - c,
                b0,
                LINE,
                x + d,
                b0,
                CUBIC,
                x + d * k,
                b0,
                x,
                b0 - d * k,
                x,
                b0 - d,
                LINE,
                x,
                y + a,
                CUBIC,
                x,
                y + a * k,
                x + a * k,
                y,
                x + a,
                y,
                CLOSE,
            ]
        }
        Shape::Ellipse {
            start,
            sweep,
            inner,
        } => {
            let (rx, ry) = (w / 2.0, h / 2.0);
            let (cx, cy) = (x + rx, y + ry);
            let a0 = start.to_radians();
            let a1 = a0 + TAU * sweep.clamp(0.0, 1.0);
            let full = *sweep >= 1.0;
            let mut path = Vec::new();
            arc(&mut path, [cx, cy, rx, ry], a0, a1, MOVE);
            if *inner > 0.0 {
                if full {
                    path.push(CLOSE);
                }
                let verb = if full { MOVE } else { LINE };
                arc(&mut path, [cx, cy, rx * inner, ry * inner], a1, a0, verb);
            } else if !full {
                path.extend([LINE, cx, cy]);
            }
            path.push(CLOSE);
            path
        }
        Shape::Polygon { count } => polygon(*count.max(&3), |_| 1.0, [x, y, w, h]),
        Shape::Star { count, ratio } => polygon(
            2 * count.max(&3),
            |i| if i % 2 == 0 { 1.0 } else { *ratio },
            [x, y, w, h],
        ),
        Shape::Path { path } => map(path, |[u, v]| [x + u * w, y + v * h]),
    }
}

/// Cubics along the ellipse `[cx, cy, rx, ry]` from the angle `a0` to `a1`, opened by `verb`.
fn arc(path: &mut Vec<f32>, [cx, cy, rx, ry]: [f32; 4], a0: f32, a1: f32, verb: f32) {
    let n = ((a1 - a0).abs() / FRAC_PI_2).ceil().max(1.0);
    let d = (a1 - a0) / n;
    let k = 4.0 / 3.0 * (d / 4.0).tan();
    path.extend([verb, cx + rx * a0.cos(), cy + ry * a0.sin()]);
    for i in 0..n as usize {
        let (s, e) = (a0 + d * i as f32, a0 + d * (i + 1) as f32);
        path.extend([
            CUBIC,
            cx + rx * (s.cos() - k * s.sin()),
            cy + ry * (s.sin() + k * s.cos()),
            cx + rx * (e.cos() + k * e.sin()),
            cy + ry * (e.sin() - k * e.cos()),
            cx + rx * e.cos(),
            cy + ry * e.sin(),
        ]);
    }
}

/// Points on a circle from the top, clockwise, scaled so their bounds fill the frame.
fn polygon(n: u32, radius: impl Fn(u32) -> f32, frame: [f32; 4]) -> Vec<f32> {
    let mut path = Vec::new();
    for i in 0..n {
        let a = std::f32::consts::TAU * i as f32 / n as f32;
        let v = if i == 0 { MOVE } else { LINE };
        path.extend([v, radius(i) * a.sin(), -radius(i) * a.cos()]);
    }
    path.push(CLOSE);
    fit(&path, frame)
}

/// Maps a path from its own bounds into `frame`.
pub fn fit(path: &[f32], [x, y, w, h]: [f32; 4]) -> Vec<f32> {
    let [l, t, bw, bh] = bounds(path);
    let scale = |v: f32, s: f32| if s > 0.0 { v / s } else { 0.0 };
    map(path, |[u, v]| {
        [x + scale(u - l, bw) * w, y + scale(v - t, bh) * h]
    })
}

pub fn bounds(path: &[f32]) -> [f32; 4] {
    let mut b = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for [x, y] in flatten(path).into_iter().flatten() {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    if b[0] > b[2] {
        return [0.0; 4];
    }
    [b[0], b[1], b[2] - b[0], b[3] - b[1]]
}

/// The verbs of `path` with their coordinates, up to the first malformed one.
pub fn segments(path: &[f32]) -> impl Iterator<Item = (f32, &[f32])> {
    let mut i = 0;
    std::iter::from_fn(move || {
        let v = *path.get(i)?;
        let n = match v {
            MOVE | LINE => 2,
            CUBIC => 6,
            CLOSE => 0,
            _ => return None,
        };
        let p = path.get(i + 1..i + 1 + n)?;
        i += 1 + n;
        Some((v, p))
    })
}

/// Whether a subpath of `path` is closed.
pub fn closed(path: &[f32]) -> bool {
    segments(path).any(|(v, _)| v == CLOSE)
}

/// Whether `path` is well formed: finite points, each subpath opened by a move.
pub fn valid(path: &[f32]) -> bool {
    let mut len = 0;
    for (k, (v, p)) in segments(path).enumerate() {
        if k == 0 && v != MOVE || !p.iter().all(|c| c.is_finite()) {
            return false;
        }
        len += 1 + p.len();
    }
    len == path.len()
}

/// Applies `f` to every point of a path.
pub fn map(path: &[f32], f: impl Fn([f32; 2]) -> [f32; 2]) -> Vec<f32> {
    let mut out = Vec::with_capacity(path.len());
    for (v, p) in segments(path) {
        out.push(v);
        for q in p.chunks(2) {
            out.extend(f([q[0], q[1]]));
        }
    }
    out
}

/// Subpaths as polylines, cubics split into straight segments.
pub fn flatten(path: &[f32]) -> Vec<Vec<[f32; 2]>> {
    let mut out: Vec<Vec<[f32; 2]>> = Vec::new();
    for (v, c) in segments(path) {
        let last = out
            .last()
            .and_then(|s| s.last())
            .copied()
            .unwrap_or([0.0; 2]);
        match v {
            MOVE => out.push(vec![[c[0], c[1]]]),
            LINE => out.last_mut().unwrap().push([c[0], c[1]]),
            CUBIC => {
                let sub = out.last_mut().unwrap();
                for n in 1..=FLATNESS {
                    let t = n as f32 / FLATNESS as f32;
                    let u = 1.0 - t;
                    let [a, b, c2, d] = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    sub.push([
                        a * last[0] + b * c[0] + c2 * c[2] + d * c[4],
                        a * last[1] + b * c[1] + c2 * c[3] + d * c[5],
                    ]);
                }
            }
            _ => {
                let first = out.last().unwrap()[0];
                out.last_mut().unwrap().push(first);
                out.push(vec![first]);
            }
        }
    }
    out
}

const ARROW: f32 = 5.0;

/// Open V at the start or end of the first subpath, sized by the stroke weight.
pub fn arrow(path: &[f32], end: bool, weight: f32) -> Vec<f32> {
    let Some(sub) = flatten(path).into_iter().next().filter(|s| s.len() > 1) else {
        return Vec::new();
    };
    let (tip, from) = match end {
        true => (sub[sub.len() - 1], sub[sub.len() - 2]),
        false => (sub[0], sub[1]),
    };
    let (dx, dy) = (tip[0] - from[0], tip[1] - from[1]);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return Vec::new();
    }
    let l = ARROW * weight;
    let (dx, dy) = (dx / len * l, dy / len * l);
    let (bx, by) = (tip[0] - dx, tip[1] - dy);
    vec![
        MOVE,
        bx + dy,
        by - dx,
        LINE,
        tip[0],
        tip[1],
        LINE,
        bx - dy,
        by + dx,
    ]
}

pub fn near(path: &[f32], x: f32, y: f32, tolerance: f32) -> bool {
    flatten(path).iter().any(|sub| {
        sub.iter().zip(sub.iter().skip(1)).any(|(a, b)| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx * dx + dy * dy;
            let t = if len > 0.0 {
                (((x - a[0]) * dx + (y - a[1]) * dy) / len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (x - a[0] - t * dx).hypot(y - a[1] - t * dy) <= tolerance
        })
    })
}

/// Nonzero winding; open subpaths count as closed, like a fill.
pub fn contains(path: &[f32], x: f32, y: f32) -> bool {
    let mut winding = 0;
    for sub in flatten(path) {
        for (a, b) in sub.iter().zip(sub.iter().skip(1).chain(sub.first())) {
            if (a[1] <= y) != (b[1] <= y) {
                let cross = (b[0] - a[0]) * (y - a[1]) - (x - a[0]) * (b[1] - a[1]);
                winding += if b[1] > a[1] {
                    (cross > 0.0) as i32
                } else {
                    -((cross < 0.0) as i32)
                };
            }
        }
    }
    winding != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_closed_by_its_verbs_not_by_a_coordinate_equal_to_close() {
        assert!(!closed(&[MOVE, CLOSE, 0.0, LINE, 10.0, CLOSE]));
        assert!(closed(&rect(0.0, 0.0, 1.0, 1.0)));
    }

    #[test]
    fn a_rect_without_radius_is_its_frame() {
        let path = outline(
            &Shape::Rect {
                radius: 0.0,
                corners: vec![],
            },
            [1.0, 2.0, 3.0, 4.0],
        );
        assert_eq!(
            path,
            [
                MOVE, 1.0, 2.0, LINE, 4.0, 2.0, LINE, 4.0, 6.0, LINE, 1.0, 6.0, CLOSE
            ]
        );
    }

    #[test]
    fn a_rounded_rect_contains_its_middle_but_not_its_corners() {
        let path = outline(
            &Shape::Rect {
                radius: 5.0,
                corners: vec![],
            },
            [0.0, 0.0, 20.0, 20.0],
        );
        assert!(contains(&path, 10.0, 10.0));
        assert!(contains(&path, 10.0, 0.5));
        assert!(!contains(&path, 0.5, 0.5));
        assert!(!contains(&path, 19.5, 19.5));
        assert!(!contains(&path, 25.0, 10.0));
    }

    fn ellipse(start: f32, sweep: f32, inner: f32) -> Shape {
        Shape::Ellipse {
            start,
            sweep,
            inner,
        }
    }

    #[test]
    fn an_ellipse_opens_to_an_arc_and_hollows_to_a_ring() {
        let ring = outline(&ellipse(0.0, 1.0, 0.5), [0.0, 0.0, 20.0, 20.0]);
        assert!(!contains(&ring, 10.0, 10.0));
        assert!(contains(&ring, 10.0, 1.0));
        let half = outline(&ellipse(0.0, 0.5, 0.0), [0.0, 0.0, 20.0, 20.0]);
        assert!(contains(&half, 10.0, 15.0));
        assert!(!contains(&half, 10.0, 5.0));
        let arc = outline(&ellipse(90.0, 0.25, 0.5), [0.0, 0.0, 20.0, 20.0]);
        assert!(contains(&arc, 5.0, 15.0));
        assert!(!contains(&arc, 15.0, 15.0));
        assert!(!contains(&arc, 9.0, 11.0));
    }

    #[test]
    fn corners_round_each_corner_on_its_own() {
        let shape = Shape::Rect {
            radius: 5.0,
            corners: vec![0.0, 8.0, 0.0, 8.0],
        };
        let path = outline(&shape, [0.0, 0.0, 20.0, 20.0]);
        assert!(contains(&path, 0.5, 0.5));
        assert!(!contains(&path, 19.5, 0.5));
        assert!(contains(&path, 19.5, 19.5));
        assert!(!contains(&path, 0.5, 19.5));
    }

    #[test]
    fn an_ellipse_fills_its_frame_but_not_the_corners() {
        let path = outline(&ellipse(0.0, 1.0, 0.0), [0.0, 0.0, 20.0, 10.0]);
        assert!(contains(&path, 10.0, 5.0));
        assert!(contains(&path, 19.5, 5.0));
        assert!(contains(&path, 10.0, 0.2));
        assert!(!contains(&path, 1.0, 1.0));
    }

    #[test]
    fn a_polygon_points_up_and_fills_its_frame() {
        let path = outline(&Shape::Polygon { count: 3 }, [0.0, 0.0, 10.0, 10.0]);
        assert!(contains(&path, 5.0, 0.5));
        assert!(contains(&path, 0.5, 9.8));
        assert!(contains(&path, 9.5, 9.8));
        assert!(!contains(&path, 1.0, 1.0));
    }

    #[test]
    fn a_star_has_points_and_notches() {
        let star = Shape::Star {
            count: 5,
            ratio: 0.38,
        };
        let path = outline(&star, [0.0, 0.0, 10.0, 10.0]);
        assert!(contains(&path, 5.0, 5.0));
        assert!(contains(&path, 5.0, 0.3));
        assert!(!contains(&path, 3.0, 2.0));
        assert!(!contains(&path, 5.0, 9.5));
    }

    #[test]
    fn a_path_scales_from_unit_space_into_its_frame() {
        let line = Shape::Path {
            path: vec![MOVE, 0.0, 0.0, LINE, 1.0, 1.0],
        };
        let path = outline(&line, [10.0, 10.0, 20.0, 10.0]);
        assert_eq!(path, [MOVE, 10.0, 10.0, LINE, 30.0, 20.0]);
        assert!(near(&path, 20.0, 15.2, 0.5));
        assert!(!near(&path, 20.0, 10.0, 0.5));
        assert!(near(&path, 31.0, 20.0, 1.5));
    }

    #[test]
    fn near_covers_the_closing_edge() {
        let path = outline(
            &Shape::Rect {
                radius: 0.0,
                corners: vec![],
            },
            [0.0, 0.0, 10.0, 10.0],
        );
        assert!(near(&path, 0.2, 5.0, 0.5));
        assert!(!near(&path, 5.0, 5.0, 0.5));
    }

    #[test]
    fn arrow_heads_point_along_the_line_ends() {
        let path = [MOVE, 0.0, 0.0, LINE, 10.0, 0.0];
        assert_eq!(
            arrow(&path, true, 1.0),
            [MOVE, 5.0, -5.0, LINE, 10.0, 0.0, LINE, 5.0, 5.0]
        );
        assert_eq!(
            arrow(&path, false, 2.0),
            [MOVE, 10.0, 10.0, LINE, 0.0, 0.0, LINE, 10.0, -10.0]
        );
    }

    #[test]
    fn bounds_follow_the_curve_not_its_control_points() {
        let path = [MOVE, 0.0, 0.0, CUBIC, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0];
        let [x, y, w, h] = bounds(&path);
        assert_eq!([x, y, w], [0.0, 0.0, 10.0]);
        assert!((h - 7.5).abs() < 0.01, "{h}");
        assert_eq!(bounds(&[]), [0.0; 4]);
    }
}
