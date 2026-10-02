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

/// The best breaks of `items` into lines, each its end and its adjustment ratio. Line
/// `i` is `widths[i]` wide, lines past the end of `widths` as wide as its last.
pub fn break_lines(items: &[Item], widths: &[f32]) -> Vec<(usize, f32)> {
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
    let last = widths.len() - 1;
    // Each break: its item, the break before it and the ratio of the line it ends.
    let mut nodes: Vec<(usize, Option<usize>, f32)> = Vec::new();
    // Breaks a line may start after: the node, its demerits and the line that starts.
    let mut starts: Vec<(Option<usize>, f64, usize)> = vec![(None, 0.0, 0)];
    for j in 0..items.len() {
        let (extra, cost) = match items[j] {
            Item::Glue { .. } if j > 0 && matches!(items[j - 1], Item::Box(_)) => (0.0, 0.0),
            Item::Penalty { width, cost } if cost < f32::INFINITY => (width as f64, cost as f64),
            _ => continue,
        };
        let line = |s: usize, width: f64| {
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
        let lines: Vec<_> = starts
            .iter()
            .map(|&(n, d, l)| {
                let (ld, r) = line(n.map_or(0, |n| nodes[n].0 + 1), widths[l] as f64);
                (n, d + ld, r, (l + 1).min(last))
            })
            .collect();
        let mut fits = lines
            .iter()
            .map(|l| l.2 >= -1.0 && cost != f64::NEG_INFINITY);
        starts.retain(|_| fits.next().unwrap());
        for next in 0..=last {
            let Some(best) = lines
                .iter()
                .filter(|l| l.3 == next)
                .min_by(|a, b| a.1.total_cmp(&b.1))
            else {
                continue;
            };
            nodes.push((j, best.0, best.2 as f32));
            starts.push((Some(nodes.len() - 1), best.1, next));
        }
    }
    let mut breaks = Vec::new();
    let mut at = starts
        .iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .and_then(|s| s.0);
    while let Some(n) = at {
        let (j, prev, r) = nodes[n];
        breaks.push((j, r));
        at = prev;
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
        break_lines(items, &[width]).iter().map(|l| l.0).collect()
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

    #[test]
    fn a_narrower_first_line_takes_fewer_words() {
        let items = words(&[4.0; 8]);
        let ends: Vec<_> = break_lines(&items, &[4.0, 14.0])
            .iter()
            .map(|l| l.0)
            .collect();
        assert_eq!(ends, vec![1, 7, 13, 16]);
    }
}
