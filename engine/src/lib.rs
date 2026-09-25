mod color;
mod display_list;
mod doc;
mod geom;
mod linebreak;
mod pdf;
mod raster;
mod style;
mod text;
mod variable;

pub use display_list::{Op, encode};
pub use doc::{Command, Doc, Snapshot};

use js_sys::{Uint8Array, Uint32Array};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Default)]
pub struct Engine {
    doc: Doc,
    list: Vec<u32>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine::default()
    }

    pub fn apply(&mut self, cmd: JsValue) -> Result<Vec<String>, JsError> {
        let cmd: Command = serde_wasm_bindgen::from_value(cmd)?;
        self.doc.apply(cmd).map_err(|e| JsError::new(&e))
    }

    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        let ser = serde_wasm_bindgen::Serializer::json_compatible();
        Ok(self.doc.snapshot().serialize(&ser)?)
    }

    /// Path from the topmost page child down to the deepest node at (x, y) in pt.
    /// `tolerance` in pt widens the hit area of outlines, e.g. for thin lines.
    pub fn hit(&self, page: usize, x: f64, y: f64, tolerance: f64) -> Vec<String> {
        self.doc.hit(page, x, y, tolerance)
    }

    /// The view is invalid after the next call into the engine.
    #[wasm_bindgen(js_name = displayList)]
    pub fn display_list(&mut self, page: usize) -> Uint32Array {
        self.list = encode(&self.doc.render(page));
        unsafe { Uint32Array::view(&self.list) }
    }

    pub fn pdf(&self) -> Vec<u8> {
        let snap = self.doc.snapshot();
        let pages: Vec<_> = (0..snap.pages.len()).map(|i| self.doc.render(i)).collect();
        pdf::pdf(&pages, snap.raster_ppi as f32, snap.color_mode)
    }

    pub fn font(&self, _id: u32) -> Uint8Array {
        unsafe { Uint8Array::view(text::FONT) }
    }
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

/// `color` with swatches replaced by what they stand for.
#[wasm_bindgen]
pub fn resolve(color: JsValue, swatches: JsValue) -> Result<JsValue, JsError> {
    let color: color::Color = serde_wasm_bindgen::from_value(color)?;
    let palette = variable::Palette {
        swatches: serde_wasm_bindgen::from_value(swatches)?,
        ..Default::default()
    };
    let scope = variable::Scope {
        palette: &palette,
        modes: &Default::default(),
    };
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(color.resolve(&scope).serialize(&ser)?)
}
