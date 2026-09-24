mod display_list;
mod doc;
mod linebreak;
mod pdf;
mod text;

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

    pub fn apply(&mut self, cmd: JsValue) -> Result<(), JsError> {
        let cmd: Command = serde_wasm_bindgen::from_value(cmd)?;
        self.doc.apply(cmd).map_err(|e| JsError::new(&e))
    }

    pub fn snapshot(&self) -> Result<JsValue, JsError> {
        let ser = serde_wasm_bindgen::Serializer::json_compatible();
        Ok(self.doc.snapshot().serialize(&ser)?)
    }

    /// The view is invalid after the next call into the engine.
    #[wasm_bindgen(js_name = displayList)]
    pub fn display_list(&mut self, page: usize) -> Uint32Array {
        self.list = encode(&self.doc.render(page));
        unsafe { Uint32Array::view(&self.list) }
    }

    pub fn pdf(&self) -> Vec<u8> {
        let pages = self.doc.snapshot().pages.len();
        pdf::pdf(&(0..pages).map(|i| self.doc.render(i)).collect::<Vec<_>>())
    }

    pub fn font(&self, _id: u32) -> Uint8Array {
        unsafe { Uint8Array::view(text::FONT) }
    }
}
