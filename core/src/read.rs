//! Reading Markdown into the tree, one pass over pulldown-cmark's events.
use crate::tree::{Align, Alert, Block, Inline, Item};
use pulldown_cmark::{Alignment, BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_MATH | Options::ENABLE_GFM
        | Options::ENABLE_SUPERSCRIPT | Options::ENABLE_SUBSCRIPT | Options::ENABLE_HIGHLIGHT | Options::ENABLE_CJK_FRIENDLY_EMPHASIS
}

/// A whole document, read at once.
pub fn read(text: &str) -> Vec<Block> { read_ranged(text).into_iter().map(|(block, _)| block).collect() }

/// Every top-level block with the span of the text it was read from.
pub(crate) fn read_ranged(text: &str) -> Vec<(Block, Range<usize>)> {
    let mut reader = Reader { events: Parser::new_ext(text, options()).into_offset_iter().collect(), at: 0 };
    let mut blocks = Vec::new();
    while reader.at < reader.events.len() {
        let range = reader.events[reader.at].1.clone();
        if let Some(block) = reader.block() { blocks.push((block, range)); }
    }
    blocks
}

struct Reader<'a> { events: Vec<(Event<'a>, Range<usize>)>, at: usize }

impl<'a> Reader<'a> {
    fn next(&mut self) -> Option<Event<'a>> { let event = self.events.get(self.at).map(|(event, _)| event.clone()); self.at += 1; event }

    /// The blocks until the end of the container that is open.
    fn blocks_until_end(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();
        while let Some((event, _)) = self.events.get(self.at) {
            if matches!(event, Event::End(_)) { self.at += 1; break; }
            if let Some(block) = self.block() { blocks.push(block); }
        }
        blocks
    }

    fn block(&mut self) -> Option<Block> {
        match self.next()? {
            Event::Start(Tag::Paragraph) => Some(Block::Paragraph { inlines: self.inlines(TagEnd::Paragraph) }),
            Event::Start(Tag::Heading { level, .. }) => Some(Block::Heading { level: level as u8, inlines: self.inlines(TagEnd::Heading(level)) }),
            Event::Start(Tag::BlockQuote(kind)) => {
                let blocks = self.blocks_until_end();
                Some(match kind {
                    Some(kind) => Block::Alert { kind: match kind { BlockQuoteKind::Note => Alert::Note, BlockQuoteKind::Tip => Alert::Tip, BlockQuoteKind::Important => Alert::Important, BlockQuoteKind::Warning => Alert::Warning, BlockQuoteKind::Caution => Alert::Caution }, blocks },
                    None => Block::Quote { blocks },
                })
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind { CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(), CodeBlockKind::Indented => String::new() };
                let mut text = String::new();
                while let Some(event) = self.next() { match event { Event::Text(part) => text.push_str(&part), Event::End(_) => break, _ => {} } }
                if text.ends_with('\n') { text.pop(); }
                Some(Block::Code { language, text })
            }
            Event::Start(Tag::List(start)) => {
                let mut items = Vec::new();
                while let Some(event) = self.next() {
                    match event {
                        Event::Start(Tag::Item) => items.push(self.item()),
                        Event::End(_) => break,
                        _ => {}
                    }
                }
                Some(Block::List { ordered: start.is_some(), start: start.unwrap_or(1), items })
            }
            Event::Start(Tag::Table(align)) => {
                let align = align.iter().map(|a| match a { Alignment::None => Align::None, Alignment::Left => Align::Left, Alignment::Center => Align::Center, Alignment::Right => Align::Right }).collect();
                let (mut head, mut rows) = (Vec::new(), Vec::new());
                while let Some(event) = self.next() {
                    match event {
                        Event::Start(Tag::TableHead) => head = self.cells(),
                        Event::Start(Tag::TableRow) => rows.push(self.cells()),
                        Event::End(TagEnd::Table) => break,
                        _ => {}
                    }
                }
                Some(Block::Table { align, head, rows })
            }
            Event::Rule => Some(Block::Rule),
            Event::DisplayMath(tex) => Some(Block::Math { tex: tex.trim().to_string() }),
            Event::Start(Tag::HtmlBlock) => {
                let mut text = String::new();
                while let Some(event) = self.next() { match event { Event::Html(part) | Event::Text(part) => text.push_str(&part), Event::End(_) => break, _ => {} } }
                Some(Block::Html { text: text.trim_end().to_string() })
            }
            Event::Html(text) => Some(Block::Html { text: text.trim_end().to_string() }),
            // A loose inline at block level (math alone on a line in some inputs): kept in a paragraph.
            event @ (Event::Text(_) | Event::Code(_) | Event::InlineMath(_)) => { self.at -= 1; let _ = event; Some(Block::Paragraph { inlines: self.inlines_loose() }) }
            Event::Start(_) => { self.skip_container(); None }
            _ => None,
        }
    }

    fn item(&mut self) -> Item {
        let mut checked = None;
        let mut blocks = Vec::new();
        let mut loose = Vec::new();
        while let Some((event, _)) = self.events.get(self.at).cloned() {
            match event {
                Event::End(TagEnd::Item) => { self.at += 1; break; }
                Event::TaskListMarker(done) => { checked = Some(done); self.at += 1; }
                // A tight item's text comes without a paragraph around it.
                Event::Text(_) | Event::Code(_) | Event::InlineMath(_) | Event::SoftBreak | Event::HardBreak | Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Highlight | Tag::Superscript | Tag::Subscript | Tag::Link { .. } | Tag::Image { .. }) => {
                    loose.extend(self.inline_one());
                }
                _ => {
                    if !loose.is_empty() { blocks.push(Block::Paragraph { inlines: std::mem::take(&mut loose) }); }
                    if let Some(block) = self.block() { blocks.push(block); }
                }
            }
        }
        if !loose.is_empty() { blocks.push(Block::Paragraph { inlines: loose }); }
        Item { checked, blocks }
    }

    fn cells(&mut self) -> Vec<Vec<Inline>> {
        let mut cells = Vec::new();
        while let Some(event) = self.next() {
            match event {
                Event::Start(Tag::TableCell) => cells.push(self.inlines(TagEnd::TableCell)),
                Event::End(TagEnd::TableHead | TagEnd::TableRow) => break,
                _ => {}
            }
        }
        cells
    }

    fn skip_container(&mut self) {
        let mut depth = 1;
        while let Some(event) = self.next() { match event { Event::Start(_) => depth += 1, Event::End(_) => { depth -= 1; if depth == 0 { break; } } _ => {} } }
    }

    /// Inlines until the given end.
    fn inlines(&mut self, end: TagEnd) -> Vec<Inline> {
        let mut inlines = Vec::new();
        while let Some((event, _)) = self.events.get(self.at) {
            if matches!(event, Event::End(e) if *e == end) { self.at += 1; break; }
            if matches!(event, Event::End(_)) { self.at += 1; continue; }
            inlines.extend(self.inline_one());
        }
        merge(inlines)
    }

    /// Inlines standing loose at block level, up to the next block.
    fn inlines_loose(&mut self) -> Vec<Inline> {
        let mut inlines = Vec::new();
        while let Some((event, _)) = self.events.get(self.at) {
            if !matches!(event, Event::Text(_) | Event::Code(_) | Event::InlineMath(_) | Event::SoftBreak | Event::HardBreak) { break; }
            inlines.extend(self.inline_one());
        }
        merge(inlines)
    }

    /// One inline, with whatever it holds.
    fn inline_one(&mut self) -> Option<Inline> {
        match self.next()? {
            Event::Text(text) => Some(Inline::Text { text: text.to_string() }),
            Event::Code(text) => Some(Inline::Code { text: text.to_string() }),
            Event::InlineMath(tex) => Some(Inline::Math { tex: tex.to_string() }),
            Event::DisplayMath(tex) => Some(Inline::Math { tex: tex.to_string() }),
            Event::Html(text) | Event::InlineHtml(text) => Some(Inline::Text { text: text.to_string() }),
            Event::SoftBreak => Some(Inline::Text { text: "\n".into() }),
            Event::HardBreak => Some(Inline::Break),
            Event::FootnoteReference(name) => Some(Inline::Text { text: format!("[^{name}]") }),
            Event::TaskListMarker(_) | Event::Rule | Event::End(_) => None,
            Event::Start(tag) => Some(match tag {
                Tag::Emphasis => Inline::Emphasis { children: self.inlines(TagEnd::Emphasis) },
                Tag::Strong => Inline::Strong { children: self.inlines(TagEnd::Strong) },
                Tag::Strikethrough => Inline::Strike { children: self.inlines(TagEnd::Strikethrough) },
                Tag::Highlight => Inline::Highlight { children: self.inlines(TagEnd::Highlight) },
                Tag::Superscript => Inline::Superscript { children: self.inlines(TagEnd::Superscript) },
                Tag::Subscript => Inline::Subscript { children: self.inlines(TagEnd::Subscript) },
                Tag::Link { dest_url, .. } => Inline::Link { href: dest_url.to_string(), children: self.inlines(TagEnd::Link) },
                Tag::Image { dest_url, .. } => {
                    let alt = plain(&self.inlines(TagEnd::Image));
                    Inline::Image { src: dest_url.to_string(), alt }
                }
                _ => { self.skip_container(); return None; }
            }),
        }
    }
}

/// Neighbouring text is one text.
fn merge(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        if let (Inline::Text { text }, Some(Inline::Text { text: last })) = (&inline, out.last_mut()) { last.push_str(text); continue; }
        out.push(inline);
    }
    out
}

/// What inlines say as plain text: an image's alt.
pub(crate) fn plain(inlines: &[Inline]) -> String {
    inlines.iter().map(|inline| match inline {
        Inline::Text { text } | Inline::Code { text } => text.clone(),
        Inline::Math { tex } => tex.clone(),
        Inline::Strong { children } | Inline::Emphasis { children } | Inline::Strike { children } | Inline::Highlight { children } | Inline::Superscript { children } | Inline::Subscript { children } | Inline::Link { children, .. } => plain(children),
        Inline::Image { alt, .. } => alt.clone(),
        Inline::Break => "\n".into(),
    }).collect()
}
