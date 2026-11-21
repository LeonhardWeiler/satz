mod color;
mod display_list;
mod doc;
#[cfg(test)]
mod examples;
#[cfg(feature = "io")]
mod fig;
mod geom;
mod image;
#[cfg(feature = "io")]
mod import;
#[cfg(feature = "io")]
mod inks;
mod layout;
mod linebreak;
#[cfg(feature = "io")]
mod pdf;
#[cfg(feature = "io")]
mod pdfx;
mod raster;
mod style;
mod text;
mod variable;
#[cfg(feature = "io")]
mod zip;

pub use display_list::{Op, encode};
pub use doc::{Command, Doc, Snapshot};

use color::ColorMode;
use js_sys::{Uint8Array, Uint32Array};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

thread_local! {
    static ON_PANIC: std::cell::RefCell<Option<js_sys::Function>> = const { std::cell::RefCell::new(None) };
}

type Raw = Box<serde_json::value::RawValue>;

fn raw<T: Serialize + ?Sized>(v: &T) -> serde_json::Result<Raw> {
    serde_json::value::to_raw_value(v)
}

fn fields<T: Serialize>(v: &T) -> serde_json::Result<BTreeMap<String, Raw>> {
    serde_json::from_str(&serde_json::to_string(v)?)
}

fn js<T: Serialize + ?Sized>(v: &T) -> Result<JsValue, JsError> {
    Ok(v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?)
}

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"import type { Changes, Color, ColorMode, Command, ImageInfo, Process, Scope, Typeface, Version } from "../types";"#;

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize)]
pub(crate) struct Delta<'a> {
    #[cfg_attr(test, ts(type = "Array<string | Sheet>"))]
    pages: Raw,
    #[cfg_attr(test, ts(type = "Array<string | Sheet>"))]
    masters: Raw,
    stories: BTreeMap<String, Option<Sent<'a>>>,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize)]
#[serde(untagged)]
enum Sent<'a> {
    Splice(Splice<'a>),
    Story(&'a doc::Story),
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Splice<'a> {
    at: usize,
    cut: usize,
    text: &'a str,
    spans_at: usize,
    spans_cut: usize,
    spans: &'a [text::Span],
}

fn splice<'a>(o: &doc::Story, s: &'a doc::Story) -> Splice<'a> {
    let (a, b) = (o.text.as_bytes(), s.text.as_bytes());
    let mut p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    while !o.text.is_char_boundary(p) {
        p -= 1;
    }
    let most = a.len().min(b.len()) - p;
    let mut q = (a.iter().rev().zip(b.iter().rev()))
        .take(most)
        .take_while(|(x, y)| x == y)
        .count();
    while !o.text.is_char_boundary(a.len() - q) {
        q -= 1;
    }
    let ps = o
        .spans
        .iter()
        .zip(&s.spans)
        .take_while(|(a, b)| a == b)
        .count();
    let qs = (o.spans[ps..].iter().rev().zip(s.spans[ps..].iter().rev()))
        .take_while(|(a, b)| a == b)
        .count();
    let units = |t: &str| t.encode_utf16().count();
    Splice {
        at: units(&o.text[..p]),
        cut: units(&o.text[p..o.text.len() - q]),
        text: &s.text[p..s.text.len() - q],
        spans_at: ps,
        spans_cut: o.spans.len() - ps - qs,
        spans: &s.spans[ps..s.spans.len() - qs],
    }
}

type Drawn = (u32, Vec<u32>, [Option<Rc<doc::Page>>; 2]);

#[wasm_bindgen]
#[derive(Default)]
pub struct Engine {
    doc: Doc,
    plates: HashMap<(String, bool), (u32, Vec<u32>)>,
    overlay: Vec<u32>,
    lists: HashMap<String, Drawn>,
    seen: Option<Rc<Snapshot>>,
    fresh: HashSet<String>,
    versions: u32,
    sent: Option<Rc<Snapshot>>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        console_error_panic_hook::set_once();
        Engine::default()
    }

    #[wasm_bindgen(js_name = onPanic)]
    pub fn on_panic(f: js_sys::Function) {
        console_error_panic_hook::set_once();
        ON_PANIC.set(Some(f));
        std::panic::set_hook(Box::new(|info| {
            console_error_panic_hook::hook(info);
            ON_PANIC.with_borrow(|f| f.as_ref().map(|f| f.call0(&JsValue::NULL)));
        }));
    }

    pub fn trap(&mut self) {
        panic!("trap");
    }

    pub fn sample() -> Engine {
        Engine {
            doc: Doc::sample(),
            ..Engine::default()
        }
    }

    pub fn blank(w: f64, h: f64, pages: usize, facing: bool) -> Engine {
        Engine {
            doc: Doc::blank(w, h, pages, facing, ColorMode::Cmyk),
            ..Engine::default()
        }
    }

    pub fn apply(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Command")] cmd: JsValue,
    ) -> Result<Vec<String>, JsError> {
        let cmd: Command = serde_wasm_bindgen::from_value(cmd)?;
        self.doc.apply(cmd).map_err(|e| JsError::new(&e))
    }

    pub fn notes(&self) -> Vec<String> {
        self.doc.notes()
    }

    pub fn snapshot(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&*self.doc.snapshot())?)
    }

    #[wasm_bindgen(unchecked_return_type = "Changes")]
    pub fn changes(&mut self) -> Result<JsValue, JsError> {
        let snap = self.doc.snapshot();
        let old = self.sent.replace(snap.clone());
        let old = old.as_deref();
        let sent: HashMap<&str, &Rc<doc::Page>> = old
            .iter()
            .flat_map(|s| s.pages.iter().chain(&s.masters))
            .map(|p| (p.id.as_str(), p))
            .collect();
        let sheets = |list: &[Rc<doc::Page>]| -> serde_json::Result<Raw> {
            let list = list.iter().map(|p| {
                let old = sent.get(p.id.as_str());
                if old.is_some_and(|o| Rc::ptr_eq(o, p)) {
                    return raw(&p.id);
                }
                let mut same = old.is_some_and(|o| {
                    o.head() == p.head()
                        && (o.children.iter().map(|n| &n.id)).eq(p.children.iter().map(|n| &n.id))
                });
                let children = (p.children.iter().enumerate())
                    .map(|(i, n)| {
                        let o = old.and_then(|o| doc::same_layer(&o.children, i, &n.id));
                        if o.is_some_and(|o| Rc::ptr_eq(o, n) || o == n) {
                            return raw(&n.id);
                        }
                        let v = raw(n)?;
                        if o.map(raw).transpose()?.is_some_and(|o| o.get() == v.get()) {
                            return raw(&n.id);
                        }
                        same = false;
                        Ok(v)
                    })
                    .collect::<serde_json::Result<Vec<_>>>()?;
                if same {
                    return raw(&p.id);
                }
                let mut v = fields(&p.head())?;
                v.insert("children".into(), raw(&children)?);
                raw(&v)
            });
            raw(&list.collect::<serde_json::Result<Vec<_>>>()?)
        };
        let mut stories = BTreeMap::new();
        for (id, s) in &snap.stories {
            match old.and_then(|o| o.stories.get(id)) {
                Some(o) if Rc::ptr_eq(o, s) => {}
                Some(o) => _ = stories.insert(id.to_string(), Some(Sent::Splice(splice(o, s)))),
                None => _ = stories.insert(id.to_string(), Some(Sent::Story(s))),
            }
        }
        for id in old.iter().flat_map(|o| o.stories.keys()) {
            if !snap.stories.contains_key(id) {
                stories.insert(id.to_string(), None);
            }
        }
        let mut out = fields(&Snapshot {
            pages: Vec::new(),
            masters: Vec::new(),
            stories: Default::default(),
            ..(*snap).clone()
        })?;
        out.extend(fields(&Delta {
            pages: sheets(&snap.pages)?,
            masters: sheets(&snap.masters)?,
            stories,
        })?);
        js_sys::JSON::parse(&serde_json::to_string(&out)?).map_err(|_| JsError::new("changes"))
    }

    #[wasm_bindgen(js_name = frameRange)]
    pub fn frame_range(&self, id: &str) -> Result<Vec<usize>, JsError> {
        Ok(self
            .doc
            .frame_range(id)
            .map_err(|e| JsError::new(&e))?
            .to_vec())
    }

    pub fn hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        self.doc.hit(page, x, y, tolerance)
    }

    #[wasm_bindgen(js_name = masterHit)]
    pub fn master_hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Option<String> {
        self.doc.master_hit(page, x, y, tolerance)
    }

    #[wasm_bindgen(js_name = displayList)]
    pub fn display_list(&mut self, page: &str) -> Uint32Array {
        unsafe { Uint32Array::view(&self.drawn(page).1) }
    }

    #[wasm_bindgen(js_name = listVersion)]
    pub fn list_version(&mut self, page: &str) -> u32 {
        self.drawn(page).0
    }

    pub fn plate(&mut self, page: &str, k: bool) -> Uint32Array {
        let version = self.drawn(page).0;
        let key = (page.to_string(), k);
        if self.plates.get(&key).is_none_or(|p| p.0 != version) {
            let list = encode(&self.doc.plate(page, k));
            self.plates.insert(key.clone(), (version, list));
        }
        unsafe { Uint32Array::view(&self.plates[&key].1) }
    }

    #[wasm_bindgen(js_name = imageSource)]
    pub fn image_source(&self, id: u32) -> Vec<f64> {
        image::source(id).map_or(Vec::new(), |(source, f)| {
            [source as f64]
                .into_iter()
                .chain(f.map(f64::from))
                .collect()
        })
    }

    pub fn separation(&self) -> Vec<u8> {
        const N: usize = 33;
        let v = |i: usize| ((i * 255 + (N - 1) / 2) / (N - 1)) as u8;
        let rgba: Vec<u8> = (0..N.pow(3))
            .flat_map(|i| {
                let (x, b) = (i % (N * N), i / (N * N));
                [v(x % N), v(x / N), v(b), 255]
            })
            .collect();
        let cmyk = color::separate_pixels(&rgba);
        let cmy = cmyk.chunks(4).flat_map(|p| [p[0], p[1], p[2], 255]);
        let k = cmyk.chunks(4).flat_map(|p| [p[3], p[3], p[3], 255]);
        cmy.chain(k).collect()
    }

    pub fn lut(&self) -> Vec<u8> {
        const N: usize = 17;
        let v = |i: usize| ((i * 255 + (N - 1) / 2) / (N - 1)) as u8;
        let cmyk: Vec<u8> = (0..N.pow(4))
            .flat_map(|i| {
                let (x, y) = (i % (N * N), i / (N * N));
                [v(x % N), v(x / N), v(y % N), v(y / N)]
            })
            .collect();
        color::preview_pixels(&cmyk)
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect()
    }

    pub fn caret(&self, id: &str, index: u32) -> Result<Vec<f64>, JsError> {
        Ok(self
            .doc
            .caret(id, index as usize)
            .map_err(|e| JsError::new(&e))?
            .to_vec())
    }

    pub fn turn(&self, id: &str) -> Result<Vec<f32>, JsError> {
        Ok(self.doc.turn_of(id).map_err(|e| JsError::new(&e))?.to_vec())
    }

    #[wasm_bindgen(js_name = pathOf)]
    pub fn path_of(&self, id: &str) -> Result<Vec<f32>, JsError> {
        self.doc.path_of(id).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = canThread)]
    pub fn can_thread(&self, from: &str, to: &str) -> bool {
        self.doc.threadable(from, to).is_ok()
    }

    #[wasm_bindgen(js_name = textFrame)]
    pub fn text_frame(&self, id: &str, index: u32) -> Result<String, JsError> {
        self.doc
            .text_frame(id, index as usize)
            .map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = textIndex)]
    pub fn text_index(&self, id: &str, x: f64, y: f64) -> Result<u32, JsError> {
        let i = self
            .doc
            .text_index(id, x, y)
            .map_err(|e| JsError::new(&e))?;
        Ok(i as u32)
    }

    #[wasm_bindgen(js_name = textLine)]
    pub fn text_line(&self, id: &str, index: u32) -> Result<Vec<u32>, JsError> {
        let l = self.doc.text_line(id, index as usize);
        Ok(l.map_err(|e| JsError::new(&e))?.map(|i| i as u32).to_vec())
    }

    #[wasm_bindgen(js_name = textOverlay)]
    pub fn text_overlay(
        &mut self,
        id: &str,
        anchor: u32,
        focus: u32,
        caret_width: f32,
        page: &str,
    ) -> Result<Uint32Array, JsError> {
        let ops = self
            .doc
            .text_overlay(id, anchor as usize, focus as usize, caret_width, page)
            .map_err(|e| JsError::new(&e))?;
        self.overlay = encode(&ops);
        Ok(unsafe { Uint32Array::view(&self.overlay) })
    }

    #[cfg(feature = "io")]
    pub fn inks(&self, page: &str, ppi: f32) -> Option<inks::Inks> {
        self.doc.inks(page, ppi)
    }

    pub fn printed(&self, page: &str) -> Option<u64> {
        self.doc.printed(page)
    }

    pub fn png(&self, page: &str, ppi: f32) -> Option<Vec<u8>> {
        self.doc.png(page, ppi)
    }

    #[cfg(feature = "io")]
    pub fn pdf(&self, title: &str, date: &str, indices: &[u32]) -> Result<Vec<u8>, JsError> {
        self.doc
            .pdf(title, date, indices)
            .map_err(|e| JsError::new(&e))
    }

    #[cfg(feature = "io")]
    pub fn package(&self, title: &str, date: &str, indices: &[u32]) -> Result<Vec<u8>, JsError> {
        self.doc
            .package(title, date, indices)
            .map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = setProfile)]
    pub fn set_profile(&mut self, icc: Option<Vec<u8>>) -> Result<(), JsError> {
        self.doc.set_profile(icc).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = setToday)]
    pub fn set_today(&mut self, y: u32, m: u32, d: u32) -> Result<(), JsError> {
        self.doc.set_today(y, m, d).map_err(|e| JsError::new(&e))
    }

    pub fn save(&self) -> Vec<u8> {
        self.doc.save()
    }

    pub fn clip(&self) -> Vec<u8> {
        self.doc.clip_out()
    }

    #[wasm_bindgen(js_name = takeClip)]
    pub fn take_clip(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.doc.take_clip(bytes).map_err(|e| JsError::new(&e))
    }

    #[cfg(feature = "io")]
    #[wasm_bindgen(js_name = importFile)]
    pub fn import_file(bytes: &[u8], name: &str) -> Result<Vec<u8>, JsError> {
        import::import(bytes, name).map_err(|e| JsError::new(&e))
    }

    pub fn load(&mut self, bytes: &[u8], updates: Vec<Uint8Array>) -> Result<(), JsError> {
        let mut doc = Doc::load(bytes).map_err(|e| JsError::new(&e))?;
        for u in updates {
            doc.sync(&u.to_vec()).map_err(|e| JsError::new(&e))?;
        }
        doc.retain_images().map_err(|e| JsError::new(&e))?;
        self.doc = doc;
        Ok(())
    }

    #[wasm_bindgen(unchecked_return_type = "Version[]")]
    pub fn versions(&self) -> Result<JsValue, JsError> {
        js(&self.doc.versions())
    }

    pub fn preview(&mut self, peer: Option<String>, counter: i32) -> Result<(), JsError> {
        let v = peer.as_deref().map(|p| (p, counter));
        self.doc.preview(v).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = versionVector)]
    pub fn version_vector(&self) -> Vec<u8> {
        self.doc.version_vector()
    }

    pub fn updates(&self, since: &[u8]) -> Vec<u8> {
        self.doc.updates(since)
    }

    pub fn sync(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.doc.sync(bytes).map_err(|e| JsError::new(&e))?;
        self.doc.retain_images().map_err(|e| JsError::new(&e))
    }

    pub fn version(&self) -> String {
        self.doc.version()
    }

    pub fn font(&self, id: u32) -> Uint8Array {
        Uint8Array::from(&text::font_bytes(id)[..])
    }

    pub fn outline(&self, font: u32, size: f32, glyphs: &[u16], positions: &[f32]) -> Vec<f32> {
        text::outline(font, size, glyphs, positions)
    }

    #[wasm_bindgen(unchecked_return_type = "[number, number[]]")]
    pub fn characters(&self, hash: Option<String>) -> Result<JsValue, JsError> {
        js(&text::characters(hash.as_deref()))
    }

    #[wasm_bindgen(js_name = addFont, unchecked_return_type = "Typeface")]
    pub fn add_font(&mut self, bytes: &[u8]) -> Result<JsValue, JsError> {
        js(&self.doc.add_font(bytes).map_err(|e| JsError::new(&e))?)
    }

    #[wasm_bindgen(js_name = removeFont)]
    pub fn remove_font(&mut self, hash: &str) -> Result<(), JsError> {
        self.doc.remove_font(hash).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = fontsAdded)]
    pub fn fonts_added(&self) -> usize {
        text::fonts_added()
    }

    #[wasm_bindgen(js_name = fontsBundled)]
    pub fn fonts_bundled(&self) -> usize {
        text::BUNDLED
    }

    #[wasm_bindgen(js_name = addImage, unchecked_return_type = "ImageInfo")]
    pub fn add_image(&mut self, bytes: &[u8], link: Option<String>) -> Result<JsValue, JsError> {
        js(&self
            .doc
            .add_image(bytes, link)
            .map_err(|e| JsError::new(&e))?)
    }

    #[wasm_bindgen(js_name = registerImage, unchecked_return_type = "ImageInfo")]
    pub fn register_image(&mut self, bytes: &[u8]) -> Result<JsValue, JsError> {
        js(&self
            .doc
            .register_image(bytes)
            .map_err(|e| JsError::new(&e))?)
    }

    pub fn image(&self, id: u32) -> Uint8Array {
        Uint8Array::from(&image::bytes(id)[..])
    }

    #[wasm_bindgen(js_name = imageFile)]
    pub fn image_file(&self, hash: &str) -> Option<Uint8Array> {
        image::id(hash).map(|id| self.image(id))
    }
}

impl Engine {
    fn drawn(&mut self, page: &str) -> &Drawn {
        let snap = self.doc.snapshot();
        if !self.seen.as_ref().is_some_and(|s| Rc::ptr_eq(s, &snap)) {
            let ids: HashSet<&str> = snap
                .pages
                .iter()
                .chain(&snap.masters)
                .map(|p| p.id.as_str())
                .collect();
            self.lists.retain(|id, _| ids.contains(id.as_str()));
            self.plates.retain(|(id, _), _| ids.contains(id.as_str()));
            self.fresh.clear();
            self.seen = Some(snap.clone());
        }
        if self.fresh.insert(page.to_string()) {
            let of = |id: &str| snap.pages.iter().chain(&snap.masters).find(|p| p.id == id);
            let p = of(page);
            let from = [p, p.and_then(|p| of(p.master.as_deref()?))].map(|p| p.cloned());
            let entry = self.lists.entry(page.to_string()).or_default();
            let same = |(a, b): (&Option<Rc<doc::Page>>, &Option<Rc<doc::Page>>)| match (a, b) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (a, b) => a.is_none() && b.is_none(),
            };
            if p.is_some() && entry.2.iter().zip(&from).all(same) {
                debug_assert_eq!(entry.1, encode(&self.doc.render(page)));
            } else {
                let list = encode(&self.doc.render(page));
                if entry.1 != list {
                    self.versions += 1;
                    (entry.0, entry.1) = (self.versions, list);
                }
                entry.2 = from;
            }
        }
        &self.lists[page]
    }
}

pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

#[wasm_bindgen(unchecked_return_type = "Typeface")]
pub fn typeface(bytes: &[u8]) -> Result<JsValue, JsError> {
    js(&text::typeface(bytes).map_err(|e| JsError::new(&e))?)
}

#[cfg(feature = "io")]
#[wasm_bindgen]
pub fn zip(names: Vec<String>, data: Vec<Uint8Array>, date: &str) -> Vec<u8> {
    let files: Vec<_> = names
        .into_iter()
        .zip(data.iter().map(Uint8Array::to_vec))
        .collect();
    zip::zip(&files, date)
}

#[wasm_bindgen]
pub fn prefix(name: &str) -> String {
    doc::prefix(name)
}

#[wasm_bindgen]
pub fn preview(c: f32, m: f32, y: f32, k: f32) -> u32 {
    let [r, g, b] = color::to_rgb([c, m, y, k]).map(|v| (v * 255.0).round() as u32);
    r << 16 | g << 8 | b
}

#[wasm_bindgen(js_name = toCmyk)]
pub fn to_cmyk(rgb: u32) -> Vec<f32> {
    color::separate(rgb).to_vec()
}

#[wasm_bindgen(unchecked_return_type = "Color")]
pub fn neutral(
    #[wasm_bindgen(unchecked_param_type = "\"black\" | \"white\" | \"gray\"")] name: &str,
    #[wasm_bindgen(unchecked_param_type = "ColorMode")] mode: JsValue,
) -> Result<JsValue, JsError> {
    let mode = serde_wasm_bindgen::from_value(mode)?;
    let color = match name {
        "black" => color::Color::black(mode),
        "white" => color::Color::white(mode),
        _ => color::Color::gray(mode),
    };
    js(&color)
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Deserialize)]
pub(crate) struct Scope {
    swatches: Vec<color::Swatch>,
    collections: Vec<variable::Collection>,
    variables: Vec<variable::Variable>,
    modes: variable::Modes,
}

#[wasm_bindgen(unchecked_return_type = "Process")]
pub fn resolve(
    #[wasm_bindgen(unchecked_param_type = "Color")] color: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Scope")] scope: JsValue,
) -> Result<JsValue, JsError> {
    let color: color::Color = serde_wasm_bindgen::from_value(color)?;
    let s: Scope = serde_wasm_bindgen::from_value(scope)?;
    let palette = variable::Palette {
        swatches: s.swatches,
        collections: s.collections,
        variables: s.variables,
        text_styles: Vec::new(),
    };
    let scope = variable::Scope {
        palette: &palette,
        modes: &s.modes,
    };
    js(&color.resolve(&scope))
}

#[cfg(test)]
fn fixture(path: &str, data: &[u8]) {
    let path = format!("{}/../web/src/{path}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("SATZ_WRITE_FIXTURES").is_some() {
        std::fs::write(&path, data).unwrap();
    }
    assert!(
        std::fs::read(&path).is_ok_and(|f| f == data),
        "{path} is stale; SATZ_WRITE_FIXTURES=1 cargo test fixture writes it"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_story_splices_into_the_one_sent_before() {
        let story = |text: &str, sizes: &[(usize, f64)]| doc::Story {
            text: text.into(),
            spans: sizes
                .iter()
                .map(|&(len, size)| text::Span {
                    len,
                    attrs: text::Attrs {
                        size,
                        ..Default::default()
                    },
                })
                .collect(),
        };
        let cases = [
            (story("aaaa", &[(4, 1.0)]), story("aaaaa", &[(5, 1.0)])),
            (
                story("ab€cd", &[(2, 1.0), (3, 2.0)]),
                story("abcd", &[(2, 1.0), (2, 2.0)]),
            ),
            (
                story("x😀y", &[(4, 1.0)]),
                story("x😀😀y", &[(1, 1.0), (4, 3.0), (1, 1.0)]),
            ),
            (story("", &[]), story("new", &[(3, 1.0)])),
        ];
        for (o, s) in cases {
            let v = serde_json::to_value(splice(&o, &s)).unwrap();
            let n = |k: &str| v[k].as_u64().unwrap() as usize;
            let mut text: Vec<u16> = o.text.encode_utf16().collect();
            text.splice(
                n("at")..n("at") + n("cut"),
                v["text"].as_str().unwrap().encode_utf16(),
            );
            let spans = serde_json::to_value(&o.spans).unwrap();
            let mut spans = spans.as_array().unwrap().clone();
            let new = v["spans"].as_array().unwrap().clone();
            spans.splice(n("spansAt")..n("spansAt") + n("spansCut"), new);
            assert_eq!(String::from_utf16(&text).unwrap(), s.text);
            assert_eq!(
                spans,
                *serde_json::to_value(&s.spans).unwrap().as_array().unwrap()
            );
        }
    }

    #[test]
    fn a_display_list_keeps_its_version_until_its_page_changes() {
        let mut e = Engine::sample();
        let a = e.doc.snapshot().pages[0].id.clone();
        let b = e
            .doc
            .apply(Command::DuplicatePage { id: a.clone() })
            .unwrap()
            .remove(0);
        let rect = e.doc.snapshot().pages[0].children[0].id.clone();
        let (va, vb) = (e.drawn(&a).0, e.drawn(&b).0);
        assert_ne!(va, vb);
        let cmd = Command::SetFrame {
            id: rect,
            x: 5.0,
            y: 5.0,
            w: 50.0,
            h: 50.0,
            crop: false,
        };
        e.doc.apply(cmd).unwrap();
        assert_ne!(e.drawn(&a).0, va);
        assert_eq!(e.drawn(&b).0, vb);
        let list = encode(&e.doc.render(&b));
        assert_eq!(e.drawn(&b).1, list);
    }
}
