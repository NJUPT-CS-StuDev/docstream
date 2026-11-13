//! A document drawn as HTML, for pages built ahead of time (a blog): the same tree every client draws, its formulas
//! typeset and its Mermaid diagrams laid out as SVG in the colour of the text, nothing left for a script to do.
//! Code is given as `<pre data-language>` for the page's own highlighter; images keep the source the text gave them.
use docstream_core::{read, Alert, Align, Block, Inline};
use std::collections::HashMap;
use std::fmt::Write;

/// Markdown drawn as HTML.
pub fn html(text: &str) -> String { render(&read(text)) }

/// Blocks drawn as HTML.
pub fn render(blocks: &[Block]) -> String {
    let mut out = String::new();
    let mut ids = HashMap::new();
    for block in blocks { block_html(&mut out, block, &mut ids); out.push('\n'); }
    out
}

fn block_html(out: &mut String, block: &Block, ids: &mut HashMap<String, usize>) {
    match block {
        Block::Paragraph { inlines } => { out.push_str("<p>"); inlines_html(out, inlines); out.push_str("</p>"); }
        Block::Heading { level, inlines } => {
            let id = slug(&plain(inlines), ids);
            let _ = write!(out, "<h{level} id=\"{}\">", attr(&id));
            inlines_html(out, inlines);
            let _ = write!(out, "</h{level}>");
        }
        Block::Quote { blocks } => { out.push_str("<blockquote>"); for b in blocks { block_html(out, b, ids); } out.push_str("</blockquote>"); }
        Block::Alert { kind, blocks } => {
            let (class, title) = match kind { Alert::Note => ("note", "说明"), Alert::Tip => ("tip", "提示"), Alert::Important => ("important", "重要"), Alert::Warning => ("warning", "注意"), Alert::Caution => ("caution", "小心") };
            let _ = write!(out, "<div class=\"markdown-alert markdown-alert-{class}\"><p class=\"markdown-alert-title\">{title}</p>");
            for b in blocks { block_html(out, b, ids); }
            out.push_str("</div>");
        }
        Block::List { ordered, start, items } => {
            if *ordered { if *start == 1 { out.push_str("<ol>"); } else { let _ = write!(out, "<ol start=\"{start}\">"); } } else { out.push_str("<ul>"); }
            for item in items {
                match item.checked {
                    Some(done) => { let _ = write!(out, "<li class=\"task-list-item\"><input type=\"checkbox\" disabled{}> ", if done { " checked" } else { "" }); }
                    None => out.push_str("<li>"),
                }
                // A tight item's single paragraph is drawn as its text alone.
                match item.blocks.as_slice() {
                    [Block::Paragraph { inlines }] => inlines_html(out, inlines),
                    blocks => for b in blocks { block_html(out, b, ids); },
                }
                out.push_str("</li>");
            }
            out.push_str(if *ordered { "</ol>" } else { "</ul>" });
        }
        Block::Code { language, text } => {
            if language == "mermaid" {
                if let Some(svg) = docstream_diagram::svg(text) { let _ = write!(out, "<figure class=\"diagram\">{svg}</figure>"); return; }
            }
            let _ = write!(out, "<pre data-language=\"{}\"><code>{}</code></pre>", attr(language), text_escape(text));
        }
        Block::Math { tex } => { let _ = write!(out, "<div class=\"math math-display\">{}</div>", docstream_math::svg(tex, true)); }
        Block::Table { align, head, rows } => {
            let style = |i: usize| match align.get(i) { Some(Align::Left) => " style=\"text-align:left\"", Some(Align::Center) => " style=\"text-align:center\"", Some(Align::Right) => " style=\"text-align:right\"", _ => "" };
            out.push_str("<div class=\"table\"><table><thead><tr>");
            for (i, cell) in head.iter().enumerate() { let _ = write!(out, "<th{}>", style(i)); inlines_html(out, cell); out.push_str("</th>"); }
            out.push_str("</tr></thead><tbody>");
            for row in rows {
                out.push_str("<tr>");
                for (i, cell) in row.iter().enumerate() { let _ = write!(out, "<td{}>", style(i)); inlines_html(out, cell); out.push_str("</td>"); }
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table></div>");
        }
        Block::Rule => out.push_str("<hr>"),
        Block::Html { text } => { let _ = write!(out, "<pre class=\"raw-html\">{}</pre>", text_escape(text)); }
    }
}

fn inlines_html(out: &mut String, inlines: &[Inline]) { for inline in inlines { inline_html(out, inline); } }

fn inline_html(out: &mut String, inline: &Inline) {
    let wrap = |out: &mut String, tag: &str, children: &[Inline]| { let _ = write!(out, "<{tag}>"); inlines_html(out, children); let _ = write!(out, "</{tag}>"); };
    match inline {
        Inline::Text { text } => out.push_str(&text_escape(text)),
        Inline::Strong { children } => wrap(out, "strong", children),
        Inline::Emphasis { children } => wrap(out, "em", children),
        Inline::Strike { children } => wrap(out, "del", children),
        Inline::Highlight { children } => wrap(out, "mark", children),
        Inline::Superscript { children } => wrap(out, "sup", children),
        Inline::Subscript { children } => wrap(out, "sub", children),
        Inline::Code { text } => { let _ = write!(out, "<code>{}</code>", text_escape(text)); }
        Inline::Math { tex, display } => { let _ = write!(out, "<span class=\"math math-{}\">{}</span>", if *display { "display" } else { "inline" }, docstream_math::svg(tex, *display)); }
        Inline::Link { href, children } => { let _ = write!(out, "<a href=\"{}\">", attr(&safe(href))); inlines_html(out, children); out.push_str("</a>"); }
        Inline::Image { src, alt } => { let _ = write!(out, "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">", attr(&safe(src)), attr(alt)); }
        Inline::Break => out.push_str("<br>"),
    }
}

/// A link's target, unless it would run script.
fn safe(href: &str) -> String {
    let scheme = href.trim_start().to_ascii_lowercase();
    if scheme.starts_with("javascript:") || scheme.starts_with("vbscript:") || scheme.starts_with("data:text/html") { "#".into() } else { href.to_string() }
}

fn plain(inlines: &[Inline]) -> String {
    inlines.iter().map(|inline| match inline {
        Inline::Text { text } | Inline::Code { text } => text.clone(),
        Inline::Math { tex, .. } => tex.clone(),
        Inline::Strong { children } | Inline::Emphasis { children } | Inline::Strike { children } | Inline::Highlight { children }
        | Inline::Superscript { children } | Inline::Subscript { children } | Inline::Link { children, .. } => plain(children),
        Inline::Image { alt, .. } => alt.clone(),
        Inline::Break => " ".into(),
    }).collect()
}

/// A heading's anchor as GitHub makes it: lower case, words joined by hyphens, punctuation dropped, CJK kept; a
/// repeated one numbered.
fn slug(text: &str, ids: &mut HashMap<String, usize>) -> String {
    let base: String = text.trim().to_lowercase().chars().filter_map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { Some(c) } else if c.is_whitespace() { Some('-') } else { None }).collect();
    let n = ids.entry(base.clone()).or_insert(0);
    let id = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
    *n += 1;
    id
}

fn text_escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
fn attr(s: &str) -> String { text_escape(s).replace('"', "&quot;") }
