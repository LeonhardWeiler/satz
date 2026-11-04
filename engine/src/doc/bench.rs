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
    let (mut d, _) = empty();
    let t = text(&mut d, &"ab".repeat(1500));
    for i in 0..1500 {
        format(
            &mut d,
            &t,
            Some([2 * i, 2 * i + 1]),
            sized(10.0 + i as f64 % 7.0),
        )
        .unwrap();
    }
    let at = 1500;
    let keystroke = median(21, || {
        edit(&mut d, &t, [at, at], "X").unwrap();
    });
    println!(
        "keystroke in 1500 runs {:.2} ms",
        keystroke.as_secs_f64() * 1e3
    );
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

/// `SEED=1 STEPS=2000 cargo test -p engine fuzz -- --ignored --nocapture` applies random
/// commands to `heavy(2)` and renders, rasterizes, exports and reloads after each one.
#[test]
#[ignore]
fn fuzz() {
    use serde_json::{Value, json};
    let env = |k, d| std::env::var(k).map_or(d, |v| v.parse().unwrap());
    let mut seed: u64 = env("SEED", 1);
    let mut rand = |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    const COMMANDS: &[(&str, &[&str])] = &[
        ("create", &["parent", "kind", "x", "y", "w", "h"]),
        ("setFrame", &["id", "x", "y", "w", "h", "crop"]),
        ("setText", &["id", "text"]),
        ("editText", &["id", "range", "text"]),
        ("format", &["id", "range", "size"]),
        ("scaleText", &["id", "by"]),
        ("addTextStyle", &["name", "size"]),
        ("deleteTextStyle", &["id"]),
        ("set", &["id", "prop"]),
        ("setPath", &["id", "path"]),
        ("fitImage", &["id", "fit"]),
        ("flip", &["id", "vertical"]),
        ("flatten", &["id"]),
        ("boolean", &["ids", "op"]),
        ("fillArea", &["id", "x", "y"]),
        ("delete", &["ids"]),
        ("group", &["ids", "frame"]),
        ("ungroup", &["ids"]),
        ("mask", &["ids"]),
        ("autoLayout", &["ids"]),
        ("move", &["ids", "parent", "index"]),
        ("order", &["ids", "to"]),
        ("duplicate", &["ids"]),
        ("copy", &["ids"]),
        ("paste", &["above", "page"]),
        ("addPage", &["after"]),
        ("duplicatePage", &["id"]),
        ("setPage", &["id", "width", "height", "bleed", "scale"]),
        ("deletePage", &["id"]),
        ("movePages", &["ids", "index"]),
        ("addMaster", &["like", "layers"]),
        ("deleteMaster", &["id"]),
        ("useMaster", &["page", "master"]),
        ("override", &["page", "id"]),
        ("resetToMaster", &["ids"]),
        ("thread", &["from", "to"]),
        ("unthread", &["id"]),
        ("addSwatch", &["name", "color", "spot"]),
        ("deleteSwatch", &["id"]),
        ("addCollection", &["name"]),
        ("addMode", &["collection", "name"]),
        ("addVariable", &["collection", "name", "value"]),
        ("bind", &["id", "prop", "variable"]),
        ("undo", &[]),
        ("redo", &[]),
    ];
    const NUMBERS: &[f64] = &[-50.0, 0.0, 0.5, 1.0, 10.0, 100.0, 1e6];
    let mut d = heavy(2);
    let steps = env("STEPS", 500);
    for step in 0..steps {
        let mut ids = Vec::new();
        fn collect(v: &Value, ids: &mut Vec<String>) {
            match v {
                Value::Object(o) => o.iter().for_each(|(k, v)| match v {
                    Value::String(s) if k == "id" => ids.push(s.clone()),
                    _ => collect(v, ids),
                }),
                Value::Array(a) => a.iter().for_each(|v| collect(v, ids)),
                _ => {}
            }
        }
        collect(&serde_json::to_value(&*d.snapshot()).unwrap(), &mut ids);
        let (kind, fields) = COMMANDS[rand(COMMANDS.len())];
        let mut cmd = json!({ "type": kind });
        for &f in fields {
            let id = |r: usize| json!(ids[r % ids.len()]);
            let n = NUMBERS[rand(NUMBERS.len())];
            cmd[f] = match f {
                "ids" | "above" => json!(
                    (0..1 + rand(3))
                        .map(|_| id(rand(1 << 30)))
                        .collect::<Vec<_>>()
                ),
                "kind" => {
                    json!(["rect", "ellipse", "star", "line", "path", "text", "frame"][rand(7)])
                }
                "x" | "y" | "w" | "h" | "by" | "width" | "height" | "bleed" | "size" => json!(n),
                "index" => json!(rand(4)),
                "range" => json!([rand(8), 8 + rand(40)]),
                "text" | "name" => json!(["", "a", "Hallo Welt\n", "\u{fb01} x\u{200b}y"][rand(4)]),
                "path" => json!((0..rand(13)).map(|_| n as f32).collect::<Vec<_>>()),
                "crop" | "vertical" | "frame" | "layers" | "spot" | "scale" => json!(rand(2) == 1),
                "fit" => json!(["cover", "contain", "frame"][rand(3)]),
                "op" => json!(["union", "subtract", "intersect", "exclude"][rand(4)]),
                "to" if kind == "order" => json!(["forward", "backward", "front", "back"][rand(4)]),
                "prop" if kind == "bind" => json!(["opacity", "fills", "gap"][rand(3)]),
                "prop" => {
                    let k = [
                        "opacity",
                        "rotation",
                        "radius",
                        "hidden",
                        "gap",
                        "paddingTop",
                    ][rand(6)];
                    cmd[k] = if k == "hidden" {
                        json!(rand(2) == 1)
                    } else {
                        json!(n)
                    };
                    continue;
                }
                "color" => json!(rand(0x1000000)),
                "value" => json!({ "number": n }),
                _ if rand(5) == 0 => Value::Null,
                _ => id(rand(1 << 30)),
            };
        }
        let Ok(cmd) = serde_json::from_value::<Command>(cmd.clone()) else {
            continue;
        };
        if std::env::var("VERBOSE").is_ok() {
            println!("{step} {cmd:?}");
        }
        let _ = d.apply(cmd);
        let s = d.snapshot();
        for p in &s.pages {
            d.render(&p.id);
            d.png(&p.id, 9.0);
            d.inks(&p.id, 9.0);
        }
        drop(s);
        pdf(&d);
        d = Doc::load(&d.save()).unwrap();
    }
}
