//! docstream for Android (and any JVM): the same reader, its tree handed over as JSON through UniFFI, and the same
//! typesetter, its formulas as glyph outlines, rules and words to draw.
use std::sync::Mutex;

uniffi::setup_scaffolding!();

/// A whole document, read at once, as the JSON of its blocks.
#[uniffi::export]
pub fn read(text: String) -> String { serde_json::to_string(&docstream_core::read(&text)).unwrap_or_else(|_| "[]".into()) }

/// TeX typeset, as the JSON of what to draw (docstream_math::Formula); `display` for a formula on its own line.
#[uniffi::export]
pub fn math(tex: String, display: bool) -> String { docstream_math::json(&tex, display) }

/// Text as it streams in; each call gives the whole document as it now stands, as JSON.
#[derive(uniffi::Object)]
pub struct Stream(Mutex<docstream_core::Stream>);

#[uniffi::export]
impl Stream {
    #[uniffi::constructor]
    pub fn new() -> Self { Stream(Mutex::new(docstream_core::Stream::new())) }
    pub fn push(&self, more: String) -> String { serde_json::to_string(&self.0.lock().unwrap().push(&more)).unwrap_or_default() }
    pub fn finish(&self) -> String { serde_json::to_string(&self.0.lock().unwrap().finish()).unwrap_or_default() }
}
