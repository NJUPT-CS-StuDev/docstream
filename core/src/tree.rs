//! The document tree: what a document is, independent of how any client draws it. Serialized as JSON with a
//! `type` tag on every node, the same shape for the web, Android and Windows.
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    Quote { blocks: Vec<Block> },
    /// A GitHub alert: > [!NOTE] and its kin.
    Alert { kind: Alert, blocks: Vec<Block> },
    List { ordered: bool, start: u64, items: Vec<Item> },
    Code { language: String, text: String },
    /// Display math, as TeX.
    Math { tex: String },
    Table { align: Vec<Align>, head: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>> },
    Rule,
    /// Raw HTML, kept as the text it is: never drawn as markup.
    Html { text: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// For a task list item, whether it is done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text { text: String },
    Strong { children: Vec<Inline> },
    Emphasis { children: Vec<Inline> },
    Strike { children: Vec<Inline> },
    Highlight { children: Vec<Inline> },
    Superscript { children: Vec<Inline> },
    Subscript { children: Vec<Inline> },
    Code { text: String },
    /// Math within text, as TeX: `$…$`, or `$$…$$` written amid the text and set on a line of its own (`display`).
    Math { tex: String, display: bool },
    Link { href: String, children: Vec<Inline> },
    Image { src: String, alt: String },
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Alert { Note, Tip, Important, Warning, Caution }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Align { None, Left, Center, Right }
