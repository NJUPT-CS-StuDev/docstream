//! TeX math typeset on the device, the same on every client: read by `tex`, laid out by `layout` from the font's
//! OpenType MATH table, given back as glyph outlines, rules and words that a client draws, or as SVG.
mod layout;
mod tex;

use layout::round;
pub use layout::Item;
use std::collections::BTreeMap;
use std::fmt::Write;

/// A typeset formula, in ems of the text around it: its width, how far it rises above the baseline and falls below,
/// what is drawn (y up from the baseline), and the outline of each glyph it draws (SVG path data, y down, in ems).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Formula { pub width: f32, pub height: f32, pub depth: f32, pub items: Vec<Item>, pub glyphs: BTreeMap<u16, String> }

/// TeX typeset as a display formula (its own line) or inline in text.
pub fn typeset(tex: &str, display: bool) -> Formula {
    let t = layout::typeset(&tex::parse(tex), display);
    let items: Vec<Item> = t.items.into_iter().map(|item| match item {
        Item::Glyph { id, x, y, scale } => Item::Glyph { id, x: round(x), y: round(y), scale: round(scale) },
        Item::Rule { x, y, width, height } => Item::Rule { x: round(x), y: round(y), width: round(width), height: round(height) },
        Item::Text { x, y, size, text } => Item::Text { x: round(x), y: round(y), size: round(size), text },
    }).collect();
    let glyphs = layout::outlines(&items);
    Formula { width: round(t.width), height: round(t.height), depth: round(t.depth), items, glyphs }
}

/// The formula as JSON, for clients that draw it themselves.
pub fn json(tex: &str, display: bool) -> String { serde_json::to_string(&typeset(tex, display)).expect("a formula serializes") }

/// The formula as SVG sized in ems, in the colour of the text around it, sitting on its baseline.
pub fn svg(tex: &str, display: bool) -> String {
    let f = typeset(tex, display);
    let total = round(f.height + f.depth);
    let mut out = String::new();
    let _ = write!(out, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}em" height="{total}em" viewBox="0 {top} {w} {total}" style="vertical-align:{d}em" fill="currentColor" role="img"><title>{title}</title>"#,
        w = f.width, top = -f.height, d = -f.depth, title = escape(tex));
    for item in &f.items {
        match item {
            Item::Glyph { id, x, y, scale } => { let _ = write!(out, r#"<path transform="translate({x} {}) scale({scale})" d="{}"/>"#, round(-y), f.glyphs[id]); }
            Item::Rule { x, y, width, height } => { let _ = write!(out, r#"<rect x="{x}" y="{}" width="{width}" height="{height}"/>"#, round(-y)); }
            Item::Text { x, y, size, text } => { let _ = write!(out, r#"<text x="{x}" y="{}" font-size="{size}">{}</text>"#, round(-y), escape(text)); }
        }
    }
    out.push_str("</svg>");
    out
}

fn escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
