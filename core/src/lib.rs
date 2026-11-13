//! docstream: Markdown, as people and models write it, read into one document tree that every client draws
//! natively: the web, Android, Windows, a blog's build. Nothing here knows who draws it or how.
//!
//! What is read: CommonMark with GitHub's tables, task lists, strikethrough and alerts; Typora's ==highlight==,
//! ^superscript^ and ~subscript~; $inline$ and $$display$$ math (kept as TeX for the drawer); fenced code with its
//! language. Raw HTML is kept as text, never as markup.
//!
//! As text streams in, [`Stream`] gives the whole tree again each time, every top-level block with a key that
//! changes only when the block does, and marked settled once a blank line and something after it have come: a client redraws only
//! what changed. The block still being written is read as if its open marks (**, `, ==, $) were closed, so that it
//! does not flicker between plain and styled while it is written.

mod tree;
mod read;
mod stream;

pub use read::read;
pub use stream::{Stream, Update, Keyed};
pub use tree::{Align, Alert, Block, Inline, Item};
