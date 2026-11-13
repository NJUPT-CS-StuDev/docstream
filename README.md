# docstream

A document kernel. Markdown, as people and models write it, is read into one document tree that every client draws
natively: a web page, an Android app, a Windows app, a blog's build. The kernel knows nothing of who draws the tree
or how; there is one reading of every document, everywhere.

**What is read.** CommonMark; GitHub's tables, task lists, strikethrough and alerts (`> [!TIP]`); Typora's
`==highlight==`, `^superscript^` and `~subscript~`; `$inline$` and `$$display$$` math, kept as TeX; fenced code with its
language. Emphasis follows the CJK-friendly rule, so `**加粗**，` is bold. Raw HTML is kept as text, never as markup.

**As it streams.** `Stream.push(text)` gives the whole tree as it now stands, every top-level block with a key that
changes only when the block does, and marked settled once a blank line and something after it have come, when
nothing more can join it: a client redraws only what changed. The block still being written is read as if its open
marks (`**`, `` ` ``, `==`, `$`) were closed, so it does not flicker between plain and styled. Fed a character at a
time, a settled block never changes and the end equals reading the whole at once; the tests hold both.

**Math.** TeX is typeset on the device by TeX's own rules (The TeXbook, appendix G), every measure read from the
OpenType MATH table of New Computer Modern Math: fractions, radicals, scripts and limits, stretchy delimiters built
from the font's variants and assemblies, accents, matrices, cases, aligned equations. A formula comes back as glyph
outlines, rules and words (CJK in `\text{}`, drawn by the client's own font), or as SVG in the colour of the text
around it. What is not understood is shown as written; an unfinished formula, streamed a character at a time, still
typesets.

**Diagrams.** Mermaid flowcharts, state diagrams and sequence diagrams are laid out on the device: a flowchart in
layers by Sugiyama's method (cycles broken, ranks by longest path, crossings reduced by barycentres, positions
balanced), each edge's label given its own place in the rank between, clusters drawn around what they hold; a
sequence diagram with its notes and frames (loop, alt, opt, par). A diagram comes back as paths and words, or as SVG
in the colour of the text around it. A kind not laid out is left for the client to show as written.

## Layout

| Part | What |
| --- | --- |
| `core` | The reader and the tree (`docstream-core`), with golden fixtures in `core/tests/fixtures` |
| `math` | The typesetter (`docstream-math`): `tex` reads TeX, `layout` sets it; cases in `math/tests/cases.tex` |
| `diagram` | The diagram layout (`docstream-diagram`); cases in `diagram/tests/cases.md` |
| `wasm` | WebAssembly binding; `js` wraps it as the `docstream` package with TypeScript types |
| `android` | UniFFI binding for Android and other JVM clients (`top.hicancan.docstream`) |

## Build

```powershell
pwsh ./build.ps1          # Rust tests, the JS package and its tests, then the Android libraries
```

`UPDATE_GOLDEN=1 cargo test` rewrites the kept trees and layouts after a deliberate change;
`cargo run -p docstream-math --example gallery -- gallery.html` (or `docstream-diagram`) draws every case on one page
to look at.

Licensed under AGPL-3.0-only.
