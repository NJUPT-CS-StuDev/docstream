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

## Layout

| Part | What |
| --- | --- |
| `core` | The reader and the tree (`docstream-core`), with golden fixtures in `core/tests/fixtures` |
| `wasm` | WebAssembly binding; `js` wraps it as the `docstream` package with TypeScript types |
| `android` | UniFFI binding for Android and other JVM clients (`top.hicancan.docstream`) |

## Build

```powershell
pwsh ./build.ps1          # Rust tests, the JS package and its tests, then the Android libraries
```

`UPDATE_GOLDEN=1 cargo test -p docstream-core` rewrites the kept trees after a deliberate change.

Licensed under AGPL-3.0-only.
