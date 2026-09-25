#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    Box(f32),
    Glue {
        width: f32,
        stretch: f32,
        shrink: f32,
    },
    Penalty {
        width: f32,
        cost: f32,
    },
}

const LINE_PENALTY: f64 = 10.0;
const INF_BAD: f64 = 1e10;
const OVERFULL: f64 = 1e24;

pub fn break_lines(items: &[Item], width: f32) -> Vec<(usize, f32)> {
    let mut sum = vec![[0.0f64; 3]; items.len() + 1];
    for (k, it) in items.iter().enumerate() {
        let d = match *it {
            Item::Box(w) => [w, 0.0, 0.0],
            Item::Glue {
                width,
                stretch,
                shrink,
            } => [width, stretch, shrink],
            Item::Penalty { .. } => [0.0; 3],
        };
        sum[k + 1] = [0, 1, 2].map(|c| sum[k][c] + d[c] as f64);
    }
    let width = width as f64;
    let mut prev = vec![None; items.len()];
    let mut starts: Vec<(Option<usize>, f64)> = vec![(None, 0.0)];
    for j in 0..items.len() {
        let (extra, cost) = match items[j] {
            Item::Glue { .. } if j > 0 && matches!(items[j - 1], Item::Box(_)) => (0.0, 0.0),
            Item::Penalty { width, cost } if cost < f32::INFINITY => (width as f64, cost as f64),
            _ => continue,
        };
        let line = |s: usize| {
            let [w, y, z] = [0, 1, 2].map(|c| sum[j][c] - sum[s][c]);
            let slack = width - w - extra;
            let r = match slack {
                0.0 => 0.0,
                _ if slack > 0.0 && y > 0.0 => slack / y,
                _ if slack < 0.0 && z > 0.0 => slack / z,
                _ => f64::INFINITY.copysign(slack),
            };
            let b = (100.0 * r.abs().powi(3)).min(INF_BAD);
            let p = if cost.is_finite() {
                cost.signum() * cost * cost
            } else {
                0.0
            };
            let d = (LINE_PENALTY + b).powi(2) + p;
            let d = if r < -1.0 {
                d - OVERFULL * (slack + z)
            } else {
                d
            };
            (d, r)
        };
        let best = starts
            .iter()
            .map(|&(i, d)| {
                let (ld, r) = line(i.map_or(0, |i| i + 1));
                (i, d + ld, r)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        prev[j] = Some((best.0, best.2 as f32));
        if cost == f64::NEG_INFINITY {
            starts.clear();
        }
        starts.push((Some(j), best.1));
    }
    let mut breaks = Vec::new();
    let mut at = starts.last().and_then(|s| s.0);
    while let Some(j) = at {
        let (i, r) = prev[j].unwrap();
        breaks.push((j, r));
        at = i;
    }
    breaks.reverse();
    breaks
}

#[cfg(test)]
mod tests {
    use super::*;
    use Item::*;

    const SPACE: Item = Glue {
        width: 1.0,
        stretch: 2.0,
        shrink: 1.0,
    };
    const END: [Item; 2] = [
        Glue {
            width: 0.0,
            stretch: 1e6,
            shrink: 0.0,
        },
        Penalty {
            width: 0.0,
            cost: f32::NEG_INFINITY,
        },
    ];

    fn ends(items: &[Item], width: f32) -> Vec<usize> {
        break_lines(items, width).iter().map(|l| l.0).collect()
    }

    fn words(ws: &[f32]) -> Vec<Item> {
        let mut v = Vec::new();
        for (i, w) in ws.iter().enumerate() {
            if i > 0 {
                v.push(SPACE);
            }
            v.push(Box(*w));
        }
        v.extend(END);
        v
    }

    #[test]
    fn text_that_fits_is_one_line() {
        let items = words(&[10.0, 20.0]);
        assert_eq!(ends(&items, 100.0), vec![items.len() - 1]);
    }

    #[test]
    fn shrinks_a_line_instead_of_leaving_an_unjustifiable_one() {
        let items = words(&[2.0, 2.0, 2.0, 5.0, 5.0, 2.0]);
        assert_eq!(ends(&items, 10.0), vec![5, 9, 12]);
    }

    #[test]
    fn a_word_wider_than_the_frame_gets_its_own_line() {
        let items = words(&[15.0, 2.0]);
        assert_eq!(ends(&items, 10.0), vec![1, 4]);
    }
}
