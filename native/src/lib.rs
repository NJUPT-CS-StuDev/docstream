//! docstream for the native clients, Android and Windows: the same reader, its tree handed over as JSON through UniFFI, and the same
//! typesetter and diagram layout, their drawings as paths, rules and words.
use std::sync::Mutex;

uniffi::setup_scaffolding!();

/// A whole document, read at once, as the JSON of its blocks.
#[uniffi::export]
pub fn read(text: String) -> String { serde_json::to_string(&docstream_core::read(&text)).unwrap_or_else(|_| "[]".into()) }

/// TeX typeset, as the JSON of what to draw (docstream_math::Formula); `display` for a formula on its own line.
#[uniffi::export]
pub fn math(tex: String, display: bool) -> String { docstream_math::json(&tex, display) }

/// A Mermaid diagram, as the JSON of what to draw (docstream_diagram::Diagram), or nothing for a kind not laid out.
#[uniffi::export]
pub fn diagram(source: String) -> Option<String> { docstream_diagram::json(&source) }

/// Text as it streams in; each call gives the whole document as it now stands, as JSON.
#[derive(uniffi::Object)]
pub struct DocStream(Mutex<docstream_core::Stream>);

#[uniffi::export]
impl DocStream {
    #[uniffi::constructor]
    pub fn new() -> Self { DocStream(Mutex::new(docstream_core::Stream::new())) }
    pub fn push(&self, more: String) -> String { serde_json::to_string(&self.0.lock().unwrap().push(&more)).unwrap_or_default() }
    pub fn finish(&self) -> String { serde_json::to_string(&self.0.lock().unwrap().finish()).unwrap_or_default() }
}
