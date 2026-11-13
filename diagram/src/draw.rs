//! What a diagram is drawn with: paths and words, in pixels of 14px text, y downward, in the colour of the text around
//! it. A client draws them in its own font, so words are measured here by the shape of their characters.
use std::fmt::Write;

/// How a path is filled: not at all, faintly (a node's ground), or solidly (an arrowhead).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Fill { None, Soft, Solid }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    /// SVG path data; stroked `stroke` pixels wide (0 for none), dashed or not.
    Path { d: String, fill: Fill, stroke: f32, dash: bool },
    /// A line of words centred on x, its middle on y.
    Text { x: f32, y: f32, text: String, size: f32, bold: bool },
}

/// A laid-out diagram: the rectangle that holds it (its top left need not be the origin) and what is drawn.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Diagram { pub x: f32, pub y: f32, pub width: f32, pub height: f32, pub items: Vec<Item> }

pub const SIZE: f32 = 14.0;
pub const LINE: f32 = SIZE * 1.4;

/// How wide words are drawn, as a typical sans-serif face draws them.
pub fn width(text: &str, size: f32) -> f32 {
    text.chars().map(|c| match c {
        ' ' => 0.28,
        'i' | 'j' | 'l' | '.' | ',' | ':' | ';' | '\'' | '|' | '!' => 0.26,
        'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '-' => 0.36,
        'm' | 'w' | 'M' | 'W' => 0.84,
        'A'..='Z' => 0.64,
        c if c.is_ascii() => 0.54,
        c if is_wide(c) => 1.0,
        _ => 0.6,
    }).sum::<f32>() * size
}

fn is_wide(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x1F300..=0x1FAFF | 0x20000..=0x3FFFD)
}

/// Words broken into lines no wider than `max`: at `<br>` and newlines always, then at spaces, or between any two
/// CJK characters.
pub fn wrap(text: &str, max: f32) -> Vec<String> {
    let text = text.replace("<br/>", "\n").replace("<br />", "\n").replace("<br>", "\n").replace("\\n", "\n");
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut word = String::new();
        let flush_word = |line: &mut String, word: &mut String, lines: &mut Vec<String>| {
            if word.is_empty() { return; }
            if !line.is_empty() && width(&format!("{line}{word}"), SIZE) > max { lines.push(line.trim_end().to_string()); *line = String::new(); }
            line.push_str(if line.is_empty() { word.trim_start() } else { word });
            word.clear();
        };
        for c in paragraph.chars() {
            if c == ' ' { word.push(c); flush_word(&mut line, &mut word, &mut lines); }
            else if is_wide(c) { flush_word(&mut line, &mut word, &mut lines); word.push(c); flush_word(&mut line, &mut word, &mut lines); }
            else { word.push(c); }
        }
        flush_word(&mut line, &mut word, &mut lines);
        lines.push(line.trim().to_string());
    }
    lines
}

/// Lines of words centred on (x, y).
pub fn label(items: &mut Vec<Item>, lines: &[String], x: f32, y: f32, bold: bool) {
    let top = y - LINE * (lines.len() as f32 - 1.0) / 2.0;
    for (i, line) in lines.iter().enumerate() { items.push(Item::Text { x: r(x), y: r(top + LINE * i as f32), text: line.clone(), size: SIZE, bold }); }
}

pub fn lines_size(lines: &[String]) -> (f32, f32) { (lines.iter().map(|l| width(l, SIZE)).fold(0.0, f32::max), LINE * lines.len() as f32) }

pub fn r(v: f32) -> f32 { (v * 10.0).round() / 10.0 }

/// Path data from commands and points, rounded.
pub struct Pen(pub String);
impl Pen {
    pub fn new() -> Pen { Pen(String::new()) }
    pub fn m(mut self, x: f32, y: f32) -> Pen { let _ = write!(self.0, "M{} {}", r(x), r(y)); self }
    pub fn l(mut self, x: f32, y: f32) -> Pen { let _ = write!(self.0, "L{} {}", r(x), r(y)); self }
    pub fn c(mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) -> Pen { let _ = write!(self.0, "C{} {} {} {} {} {}", r(x1), r(y1), r(x2), r(y2), r(x), r(y)); self }
    pub fn a(mut self, rx: f32, ry: f32, sweep: bool, x: f32, y: f32) -> Pen { let _ = write!(self.0, "A{} {} 0 0 {} {} {}", r(rx), r(ry), u8::from(sweep), r(x), r(y)); self }
    pub fn z(mut self) -> String { self.0.push('Z'); self.0 }
    pub fn done(self) -> String { self.0 }
}

/// A rectangle with rounded corners.
pub fn rect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> String {
    let k = radius.min(w / 2.0).min(h / 2.0);
    if k <= 0.0 { return Pen::new().m(x, y).l(x + w, y).l(x + w, y + h).l(x, y + h).z(); }
    Pen::new().m(x + k, y).l(x + w - k, y).a(k, k, true, x + w, y + k).l(x + w, y + h - k).a(k, k, true, x + w - k, y + h)
        .l(x + k, y + h).a(k, k, true, x, y + h - k).l(x, y + k).a(k, k, true, x + k, y).z()
}

/// A filled arrowhead whose tip is at (x, y), pointing along (dx, dy).
pub fn arrowhead(x: f32, y: f32, dx: f32, dy: f32) -> Item {
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (ux, uy) = (dx / len, dy / len);
    let (bx, by) = (x - ux * 9.0, y - uy * 9.0);
    let (px, py) = (-uy * 4.5, ux * 4.5);
    Item::Path { d: Pen::new().m(x, y).l(bx + px, by + py).l(x - ux * 6.5, y - uy * 6.5).l(bx - px, by - py).z(), fill: Fill::Solid, stroke: 0.0, dash: false }
}

/// An open arrowhead (two strokes) at (x, y), pointing along (dx, dy).
pub fn open_head(x: f32, y: f32, dx: f32, dy: f32) -> Item {
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (ux, uy) = (dx / len, dy / len);
    let (bx, by) = (x - ux * 9.0, y - uy * 9.0);
    let (px, py) = (-uy * 5.0, ux * 5.0);
    Item::Path { d: Pen::new().m(bx + px, by + py).l(x, y).l(bx - px, by - py).done(), fill: Fill::None, stroke: 1.4, dash: false }
}

/// A cross at (x, y): a message that is lost, an edge that ends in x.
pub fn cross(x: f32, y: f32) -> Item {
    Item::Path { d: Pen::new().m(x - 5.0, y - 5.0).l(x + 5.0, y + 5.0).m(x + 5.0, y - 5.0).l(x - 5.0, y + 5.0).done(), fill: Fill::None, stroke: 1.6, dash: false }
}

/// A small circle at (x, y).
pub fn dot(x: f32, y: f32, radius: f32, fill: Fill) -> Item {
    Item::Path { d: Pen::new().m(x - radius, y).a(radius, radius, true, x + radius, y).a(radius, radius, true, x - radius, y).z(), fill, stroke: if fill == Fill::Solid { 0.0 } else { 1.4 }, dash: false }
}

pub fn svg(d: &Diagram, title: &str) -> String {
    let mut out = String::new();
    let _ = write!(out, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="{x} {y} {w} {h}" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" font-size="{SIZE}" text-anchor="middle" dominant-baseline="central" role="img"><title>{}</title>"#, escape(title), x = d.x, y = d.y, w = r(d.width), h = r(d.height));
    for item in &d.items {
        match item {
            Item::Path { d, fill, stroke, dash } => {
                let fill = match fill { Fill::None => "", Fill::Soft => r#" fill="currentColor" fill-opacity="0.06""#, Fill::Solid => r#" fill="currentColor""# };
                let stroke = if *stroke == 0.0 { r#" stroke="none""#.to_string() } else { format!(r#" stroke-width="{stroke}""#) };
                let dash = if *dash { r#" stroke-dasharray="5 4""# } else { "" };
                let _ = write!(out, r#"<path d="{d}"{fill}{stroke}{dash}/>"#);
            }
            Item::Text { x, y, text, size, bold } => {
                let weight = if *bold { r#" font-weight="600""# } else { "" };
                let size = if *size == SIZE { String::new() } else { format!(r#" font-size="{size}""#) };
                let _ = write!(out, r#"<text x="{x}" y="{y}" fill="currentColor" stroke="none"{size}{weight}>{}</text>"#, escape(text));
            }
        }
    }
    out.push_str("</svg>");
    out
}

fn escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
