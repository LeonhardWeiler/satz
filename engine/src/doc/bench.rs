//! `cargo test --release -p engine bench -- --ignored --nocapture`

use super::tests::*;
use super::*;
use std::time::{Duration, Instant};

/// The median time of `runs` calls of `f`.
fn median(runs: usize, mut f: impl FnMut()) -> Duration {
    let mut t: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .collect();
    t.sort();
    t[runs / 2]
}

/// `n` copies of the example page, the text frames of the first 16 threaded into
/// one story of 4 paragraphs per frame: (doc, page ids, frame ids of the story).
fn pages(n: usize) -> (Doc, Vec<String>, Vec<String>) {
    let mut d = Doc::sample();
    let mut ids = vec![page(&d).id];
    for _ in 1..n {
        let id = ids.last().unwrap().clone();
        ids.push(d.apply(Command::DuplicatePage { id }).unwrap().remove(0));
    }
    let s = d.snapshot();
    let frames: Vec<String> = s.pages[..n.min(16)]
        .iter()
        .map(|p| {
            let t = p
                .children
                .iter()
                .find(|c| matches!(c.kind, Kind::Text { .. }));
            t.unwrap().id.clone()
        })
        .collect();
    for f in &frames[1..] {
        set_text(&mut d, f, "");
    }
    let story = vec![SAMPLE; 4 * frames.len()].join("\n");
    set_text(&mut d, &frames[0], &story);
    for f in frames.windows(2) {
        thread(&mut d, &f[0], &f[1]).unwrap();
    }
    (d, ids, frames)
}

#[test]
#[ignore]
fn bench() {
    println!("pages  set_frame  keystroke  render  hit    save   pdf  (median, ms)");
    for n in [1, 8, 32] {
        let (mut d, ids, frames) = pages(n);
        let rect = page(&d).children[0].id.clone();
        let mut x = 0.0;
        let set_frame = median(21, || {
            x = 10.0 - x;
            let cmd = Command::SetFrame {
                id: rect.clone(),
                x,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                crop: false,
            };
            d.apply(cmd).unwrap();
        });
        let frame = &frames[frames.len() / 2];
        let at = flow(&d, frame).1 + 10;
        let keystroke = median(21, || {
            edit(&mut d, frame, [at, at], "X").unwrap();
            d.snapshot();
        });
        let render = median(21, || {
            d.render(&ids[0]);
        });
        let hit = median(21, || {
            d.hit(&ids[0], 60.0, 100.0, 1.0);
        });
        let save = median(21, || {
            d.save();
        });
        let pdf = median(3, || {
            d.pdf("Satz", "2026-09-28T12:00:00Z");
        });
        let [a, b, c, e, g, f] =
            [set_frame, keystroke, render, hit, save, pdf].map(|t| t.as_secs_f64() * 1e3);
        println!("{n:<6} {a:<10.2} {b:<10.2} {c:<7.2} {e:<6.2} {g:<6.2} {f:.0}");
    }
    let (mut d, ids, _) = pages(1);
    cmyk(&mut d);
    for ppi in [36.0, 72.0] {
        let inks = median(5, || {
            d.inks(&ids[0], ppi).unwrap();
        });
        let i = d.inks(&ids[0], ppi).unwrap();
        let image = median(5, || {
            i.image(0b0101, 300.0, true, true);
        });
        let [a, b] = [inks, image].map(|t| t.as_secs_f64() * 1e3);
        println!("inks at {ppi} ppi {a:.1} ms, image {b:.1} ms");
    }
}
