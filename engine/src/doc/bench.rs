//! `cargo test --release -p engine bench -- --ignored --nocapture`

use super::tests::*;
use super::*;
use crate::examples::heavy;
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

/// `heavy(n)`, its middle page, the first shape and the story frame on it.
fn heavy_page(n: usize) -> (Doc, String, String, String) {
    let d = heavy(n);
    let s = d.snapshot();
    let p = &s.pages[n / 2];
    let (shape, frame) = (
        p.children[0].id.clone(),
        p.children.last().unwrap().id.clone(),
    );
    let id = p.id.clone();
    drop(s);
    (d, id, shape, frame)
}

#[test]
#[ignore]
fn bench() {
    println!("pages  set_frame  keystroke  json   render  hit    save   (median, ms)");
    for n in [1, 20, 50] {
        let (mut d, page, shape, frame) = heavy_page(n);
        let mut x = 0.0;
        let set_frame = median(21, || {
            x = 10.0 - x;
            let cmd = Command::SetFrame {
                id: shape.clone(),
                x,
                y: 0.0,
                w: 40.0,
                h: 40.0,
                crop: false,
            };
            d.apply(cmd).unwrap();
        });
        let at = flow(&d, &frame).1 + 10;
        let keystroke = median(21, || {
            edit(&mut d, &frame, [at, at], "X").unwrap();
        });
        let json = median(21, || {
            serde_json::to_string(&*d.snapshot()).unwrap();
        });
        let render = median(21, || {
            d.render(&page);
        });
        let hit = median(21, || {
            d.hit(&page, 60.0, 100.0, 1.0);
        });
        let save = median(5, || {
            d.save();
        });
        let [a, b, c, e, f, g] =
            [set_frame, keystroke, json, render, hit, save].map(|t| t.as_secs_f64() * 1e3);
        println!("{n:<6} {a:<10.2} {b:<10.2} {c:<6.2} {e:<7.2} {f:<6.2} {g:.2}");
    }
    let (mut d, page, _, _) = heavy_page(1);
    let pdf = median(3, || {
        pdf(&d);
    });
    println!("pdf of 1 page {:.0} ms", pdf.as_secs_f64() * 1e3);
    cmyk(&mut d);
    for ppi in [36.0, 72.0] {
        let inks = median(5, || {
            d.inks(&page, ppi).unwrap();
        });
        let i = d.inks(&page, ppi).unwrap();
        let image = median(5, || {
            i.image(0b0101, 300.0, true, true);
        });
        let [a, b] = [inks, image].map(|t| t.as_secs_f64() * 1e3);
        println!("inks at {ppi} ppi {a:.1} ms, image {b:.1} ms");
    }
}

/// `PHASE=set_frame|keystroke PAGES=50 cargo test --release -p engine prof -- --ignored`
/// repeats one phase for a profiler such as `perf record -g`.
#[test]
#[ignore]
fn prof() {
    let n = std::env::var("PAGES").map_or(50, |p| p.parse().unwrap());
    let (mut d, _, shape, frame) = heavy_page(n);
    let at = flow(&d, &frame).1 + 10;
    let phase = std::env::var("PHASE").unwrap_or_default();
    let start = Instant::now();
    for i in 0..100 {
        match phase.as_str() {
            "keystroke" => {
                edit(&mut d, &frame, [at, at], "X").unwrap();
            }
            _ => {
                let cmd = Command::SetFrame {
                    id: shape.clone(),
                    x: (i % 2) as f64 * 10.0,
                    y: 0.0,
                    w: 40.0,
                    h: 40.0,
                    crop: false,
                };
                d.apply(cmd).unwrap();
            }
        }
    }
    println!("{:.2} ms each", start.elapsed().as_secs_f64() * 10.0);
}
