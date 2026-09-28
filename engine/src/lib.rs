mod color;
mod display_list;
mod doc;
#[cfg(test)]
mod examples;
mod geom;
mod image;
mod inks;
mod layout;
mod linebreak;
mod pdf;
mod pdfx;
mod raster;
mod style;
mod text;
mod variable;

pub use display_list::{Op, encode};
pub use doc::{Command, Doc, Snapshot};

use color::ColorMode;
use js_sys::{Uint8Array, Uint32Array};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Default)]
pub struct Engine {
    doc: Doc,
    list: Vec<u32>,
    overlay: Vec<u32>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        console_error_panic_hook::set_once();
        Engine::default()
    }

    /// An empty document of `pages` pages `w` × `h` pt.
    pub fn blank(w: f64, h: f64, pages: usize, facing: bool) -> Engine {
        Engine {
            doc: Doc::blank(w, h, pages, facing, ColorMode::Cmyk),
            list: Vec::new(),
            overlay: Vec::new(),
        }
    }

    pub fn apply(&mut self, cmd: JsValue) -> Result<Vec<String>, JsError> {
        let cmd: Command = serde_wasm_bindgen::from_value(cmd)?;
        self.doc.apply(cmd).map_err(|e| JsError::new(&e))
    }

    pub fn snapshot(&self) -> Result<String, JsError> {
        Ok(serde_json::to_string(&*self.doc.snapshot())?)
    }

    /// Path from the topmost page child down to the deepest node at (x, y) in pt.
    /// `tolerance` in pt widens the hit area of outlines, e.g. for thin lines.
    pub fn hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        self.doc.hit(page, x, y, tolerance)
    }

    /// The layer of the page's master at (x, y) in pt that the page shows.
    #[wasm_bindgen(js_name = masterHit)]
    pub fn master_hit(&self, page: &str, x: f64, y: f64, tolerance: f64) -> Option<String> {
        self.doc.master_hit(page, x, y, tolerance)
    }

    /// The view is invalid after the next call into the engine.
    #[wasm_bindgen(js_name = displayList)]
    pub fn display_list(&mut self, page: &str, proof: bool) -> Uint32Array {
        self.list = encode(&if proof {
            self.doc.proof(page)
        } else {
            self.doc.render(page)
        });
        unsafe { Uint32Array::view(&self.list) }
    }

    /// The display list of the page `id` as its inverted C, M and Y plate, or its K
    /// plate; the view is invalid after the next call into the engine.
    pub fn plate(&mut self, page: &str, k: bool) -> Uint32Array {
        self.list = encode(&self.doc.plate(page, k));
        unsafe { Uint32Array::view(&self.list) }
    }

    /// Screen colours of CMYK on a grid of 17 steps per ink as RGBA pixels, C across
    /// and Y down within a tile, M across and K down between tiles.
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

    /// [x, top, bottom] in pt of a caret before the UTF-16 `index` of the text `id`.
    pub fn caret(&self, id: &str, index: u32) -> Result<Vec<f64>, JsError> {
        Ok(self
            .doc
            .caret(id, index as usize)
            .map_err(|e| JsError::new(&e))?
            .to_vec())
    }

    /// The transform [a b c d e f] that turns the layer `id` on its page.
    pub fn turn(&self, id: &str) -> Result<Vec<f32>, JsError> {
        Ok(self.doc.turn_of(id).map_err(|e| JsError::new(&e))?.to_vec())
    }

    /// The frame of the thread of the text `id` that holds the UTF-16 `index`.
    #[wasm_bindgen(js_name = textFrame)]
    pub fn text_frame(&self, id: &str, index: u32) -> Result<String, JsError> {
        self.doc
            .text_frame(id, index as usize)
            .map_err(|e| JsError::new(&e))
    }

    /// The UTF-16 index of the character boundary of the text `id` nearest (x, y) in pt.
    #[wasm_bindgen(js_name = textIndex)]
    pub fn text_index(&self, id: &str, x: f64, y: f64) -> Result<u32, JsError> {
        let i = self
            .doc
            .text_index(id, x, y)
            .map_err(|e| JsError::new(&e))?;
        Ok(i as u32)
    }

    /// UTF-16 start and end of the line of the text `id` that holds `index`.
    #[wasm_bindgen(js_name = textLine)]
    pub fn text_line(&self, id: &str, index: u32) -> Result<Vec<u32>, JsError> {
        let l = self.doc.text_line(id, index as usize);
        Ok(l.map_err(|e| JsError::new(&e))?.map(|i| i as u32).to_vec())
    }

    /// Display list of the selection from `anchor` to `focus` in the text `id` on the
    /// page `page`, or of a caret `caret_width` pt wide. The view is invalid after the
    /// next call.
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

    /// The inks of the page `page` as it prints at `ppi`.
    pub fn inks(&self, page: &str, ppi: f32) -> Option<inks::Inks> {
        self.doc.inks(page, ppi)
    }

    /// The document as a PDF titled `title`, made at `date` in ISO 8601 UTC.
    pub fn pdf(&self, title: &str, date: &str) -> Vec<u8> {
        self.doc.pdf(title, date)
    }

    pub fn save(&self) -> Vec<u8> {
        self.doc.save()
    }

    /// Replaces the document; on an error it stays as it was.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.doc = Doc::load(bytes).map_err(|e| JsError::new(&e))?;
        Ok(())
    }

    /// Changes whenever the document does.
    pub fn version(&self) -> String {
        self.doc.version()
    }

    pub fn font(&self, id: u32) -> Uint8Array {
        Uint8Array::from(&text::font_bytes(id)[..])
    }

    /// Adds a TrueType or OpenType font and returns its name and hash.
    #[wasm_bindgen(js_name = addFont)]
    pub fn add_font(&mut self, bytes: &[u8]) -> Result<JsValue, JsError> {
        let face = self.doc.add_font(bytes).map_err(|e| JsError::new(&e))?;
        Ok(serde_wasm_bindgen::to_value(&face)?)
    }

    /// Removes the added font `hash`.
    #[wasm_bindgen(js_name = removeFont)]
    pub fn remove_font(&mut self, hash: &str) -> Result<(), JsError> {
        self.doc.remove_font(hash).map_err(|e| JsError::new(&e))
    }

    /// The number of font ids, removed ones included.
    #[wasm_bindgen(js_name = fontsAdded)]
    pub fn fonts_added(&self) -> usize {
        text::fonts_added()
    }

    /// Adds a PNG or JPEG file to the document's images and returns its hash and
    /// size in pixels, for the command `placeImage`.
    #[wasm_bindgen(js_name = addImage)]
    pub fn add_image(&mut self, bytes: &[u8]) -> Result<JsValue, JsError> {
        let info = self.doc.add_image(bytes).map_err(|e| JsError::new(&e))?;
        Ok(serde_wasm_bindgen::to_value(&info)?)
    }

    /// The file of the image `id` of display lists.
    pub fn image(&self, id: u32) -> Uint8Array {
        Uint8Array::from(&image::bytes(id)[..])
    }
}

/// FNV-1a hash of `bytes` in hex, which names fonts and images by their content.
pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

/// The name and hash of the font `bytes`.
#[wasm_bindgen]
pub fn typeface(bytes: &[u8]) -> Result<JsValue, JsError> {
    let face = text::typeface(bytes).map_err(|e| JsError::new(&e))?;
    Ok(serde_wasm_bindgen::to_value(&face)?)
}

/// The letters that the page numbers of the master `name` show.
#[wasm_bindgen]
pub fn prefix(name: &str) -> String {
    doc::prefix(name)
}

/// Screen preview of a CMYK colour as 0xRRGGBB.
#[wasm_bindgen]
pub fn preview(c: f32, m: f32, y: f32, k: f32) -> u32 {
    let [r, g, b] = color::to_rgb([c, m, y, k]).map(|v| (v * 255.0).round() as u32);
    r << 16 | g << 8 | b
}

/// CMYK of the 0xRRGGBB colour `rgb` in the document's print condition.
#[wasm_bindgen(js_name = toCmyk)]
pub fn to_cmyk(rgb: u32) -> Vec<f32> {
    color::separate(rgb).to_vec()
}

/// `"black"`, `"white"` or `"gray"` in the colour mode `mode`.
#[wasm_bindgen]
pub fn neutral(name: &str, mode: JsValue) -> Result<JsValue, JsError> {
    let mode = serde_wasm_bindgen::from_value(mode)?;
    let color = match name {
        "black" => color::Color::black(mode),
        "white" => color::Color::white(mode),
        _ => color::Color::gray(mode),
    };
    Ok(serde_wasm_bindgen::to_value(&color)?)
}

/// `color` with swatches and variables replaced by what they stand for in `scope`,
/// a palette with `modes`.
#[wasm_bindgen]
pub fn resolve(color: JsValue, scope: JsValue) -> Result<JsValue, JsError> {
    #[derive(serde::Deserialize)]
    struct Owned {
        #[serde(flatten)]
        palette: variable::Palette,
        #[serde(default)]
        modes: variable::Modes,
    }
    let color: color::Color = serde_wasm_bindgen::from_value(color)?;
    let s: Owned = serde_wasm_bindgen::from_value(scope)?;
    let scope = variable::Scope {
        palette: &s.palette,
        modes: &s.modes,
    };
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(color.resolve(&scope).serialize(&ser)?)
}
