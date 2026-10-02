use crate::color::{Ink, apart, preview_pixels, separate_pixels};
use crate::display_list::{Op, recolor};
use crate::image::{self, plate};
use crate::raster::rasterize;
use wasm_bindgen::prelude::*;

const OVER: [u8; 4] = [204, 47, 131, 255];

/// A page as it prints at a preview resolution: the coverage of each ink per pixel,
/// C, M, Y and K and then each spot colour, and the pixels of RGB colours that print
/// visibly different. The pixels cover the page and its bleed.
#[wasm_bindgen]
pub struct Inks {
    width: u32,
    height: u32,
    spots: Vec<(String, [f32; 4])>,
    plates: Vec<Vec<u8>>,
    gamut: Vec<bool>,
}

impl Inks {
    /// The inks of the page `ops` begins with, `ppi` pixels per inch.
    pub fn new(ops: &[Op], ppi: f32) -> Option<Inks> {
        let Some(&Op::Page {
            width,
            height,
            bleed,
        }) = ops.first()
        else {
            return None;
        };
        let (ops, rect) = (
            &ops[1..],
            [-bleed, -bleed, width + 2.0 * bleed, height + 2.0 * bleed],
        );
        let found = std::cell::RefCell::new(Vec::new());
        let overprint = std::cell::Cell::new(false);
        recolor(ops, &|c, ink, over| {
            if let Ink::Spot { name, cmyk, .. } = ink {
                found.borrow_mut().push((name.clone(), *cmyk));
            }
            overprint.set(overprint.get() || over);
            *c
        });
        let mut spots = found.into_inner();
        spots.sort_by(|a, b| a.0.cmp(&b.0));
        spots.dedup_by(|a, b| a.0 == b.0);
        let inks: Vec<Option<&str>> = [None; 4]
            .into_iter()
            .chain(spots.iter().map(|s| Some(s.0.as_str())))
            .collect();
        let (mut w, mut h, mut plates) = (0, 0, Vec::new());
        let size = if overprint.get() { 1 } else { 3 };
        for (n, chunk) in inks.chunks(size).enumerate() {
            let at = |k: usize| (k < chunk.len()).then_some(n * size + k);
            let cover = |c: &[f32; 4], ink: &Ink, k: usize| match (ink, at(k).and_then(|i| inks[i]))
            {
                (Ink::Spot { name, tint, .. }, Some(spot)) if name == spot => *tint,
                (Ink::Spot { .. }, _) | (_, Some(_)) => 0.0,
                _ => at(k).map_or(0.0, |i| ink.cmyk(c)[i]),
            };
            let pick = |c: [f32; 4]| {
                std::array::from_fn(|k| at(k).and_then(|i| c.get(i)).map_or(0.0, |v| *v))
            };
            let images = |id, size| plate(id, size, pick);
            let recolored = recolor(ops, &|c, ink, over| {
                let v = [0, 1, 2].map(|k| cover(c, ink, k));
                [
                    v[0],
                    v[1],
                    v[2],
                    if over && v[0] == 0.0 { 0.0 } else { c[3] },
                ]
            });
            let px = rasterize(&recolored, rect, ppi, &images)?;
            (w, h) = (px.width(), px.height());
            for k in 0..chunk.len() {
                plates.push(px.data().chunks(4).map(|p| p[k]).collect());
            }
        }
        let rgb = rasterize(
            &recolor(ops, &|c, ink, _| {
                if *ink == Ink::Rgb { *c } else { [0.0; 4] }
            }),
            rect,
            ppi,
            &|id, _| image::pixmap(id),
        )?
        .take_demultiplied();
        let printed = preview_pixels(&separate_pixels(&rgb));
        let gamut = rgb
            .chunks(4)
            .zip(printed.chunks(3))
            .map(|(a, b)| a[3] > 127 && apart(rgb_of(a), rgb_of(b)))
            .collect();
        Some(Inks {
            width: w,
            height: h,
            spots,
            plates,
            gamut,
        })
    }

    /// The total coverage of each pixel in percent.
    fn totals(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.gamut.len()).map(|i| self.plates.iter().map(|p| p[i] as f32).sum::<f32>() / 2.55)
    }
}

fn rgb_of(p: &[u8]) -> [f32; 3] {
    [p[0], p[1], p[2]].map(|v| v as f32 / 255.0)
}

#[wasm_bindgen]
impl Inks {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The names of the spot colours, in the order of their plates after K.
    pub fn spots(&self) -> Vec<String> {
        self.spots.iter().map(|s| s.0.clone()).collect()
    }

    /// The highest coverage of each plate in percent.
    pub fn max(&self) -> Vec<f32> {
        self.plates
            .iter()
            .map(|p| p.iter().copied().max().unwrap_or(0) as f32 / 2.55)
            .collect()
    }

    /// The total coverage of each pixel in percent, row by row.
    pub fn coverage(&self) -> Vec<f32> {
        self.totals().collect()
    }

    /// RGBA pixels over the page: the plates `on` (bit 0 for C … bit 3 for K, then the
    /// spots) as they print if a spot is off, magenta where the coverage is above
    /// `limit` % if `over`, and hatched where a colour is out of gamut if `gamut`.
    pub fn image(&self, on: u32, limit: f32, over: bool, gamut: bool) -> Vec<u8> {
        let n = self.gamut.len();
        let spots = (4..self.plates.len()).all(|p| on & 1 << p != 0);
        let mut out = vec![0u8; n * 4];
        if !spots {
            let cmyk: Vec<u8> = (0..n)
                .flat_map(|i| {
                    let mut c = [0.0f32; 4];
                    for (p, plate) in self
                        .plates
                        .iter()
                        .enumerate()
                        .filter(|(p, _)| on & 1 << p != 0)
                    {
                        let v = plate[i] as f32 / 255.0;
                        let alt = if p < 4 {
                            std::array::from_fn(|k| if k == p { 1.0 } else { 0.0 })
                        } else {
                            self.spots[p - 4].1
                        };
                        for k in 0..4 {
                            c[k] += v * alt[k];
                        }
                    }
                    c.map(|v| (v.min(1.0) * 255.0).round() as u8)
                })
                .collect();
            for (o, p) in out.chunks_mut(4).zip(preview_pixels(&cmyk).chunks(3)) {
                o.copy_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        let w = self.width as usize;
        for (i, total) in self.totals().enumerate() {
            let o = &mut out[i * 4..i * 4 + 4];
            if over && total > limit {
                o.copy_from_slice(&OVER);
            } else if gamut && self.gamut[i] {
                o.copy_from_slice(if (i % w + i / w) % 4 < 2 {
                    &[70, 70, 70, 230]
                } else {
                    &[255, 255, 255, 140]
                });
            }
        }
        out
    }
}
