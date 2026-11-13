//! docstream for the web: the same reader, its tree handed over as JSON, and the same typesetter, its formulas as SVG.
use wasm_bindgen::prelude::*;

/// A whole document, read at once, as the JSON of its blocks.
#[wasm_bindgen]
pub fn read(text: &str) -> String { serde_json::to_string(&docstream_core::read(text)).unwrap_or_else(|_| "[]".into()) }

/// TeX typeset as SVG sized in ems, in the colour of the text around it; `display` for a formula on its own line.
#[wasm_bindgen]
pub fn math(tex: &str, display: bool) -> String { docstream_math::svg(tex, display) }

/// Text as it streams in; each call gives the whole document as it now stands, as JSON.
#[wasm_bindgen]
pub struct Stream(docstream_core::Stream);

#[wasm_bindgen]
impl Stream {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Stream { Stream(docstream_core::Stream::new()) }
    pub fn push(&mut self, more: &str) -> String { serde_json::to_string(&self.0.push(more)).unwrap_or_default() }
    pub fn finish(&mut self) -> String { serde_json::to_string(&self.0.finish()).unwrap_or_default() }
}

impl Default for Stream { fn default() -> Self { Self::new() } }
