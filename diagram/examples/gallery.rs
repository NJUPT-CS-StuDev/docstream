//! Every case the golden tests hold, drawn on one page to look at: `cargo run -p docstream-diagram --example gallery -- out.html`.
fn main() {
    let out = std::env::args().nth(1).expect("where to write the page");
    let cases = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/cases.md")).expect("the cases");
    let mut page = String::from("<!doctype html><meta charset=utf-8><style>body{font:14px system-ui,'Microsoft YaHei',sans-serif;max-width:1100px;margin:2em auto;color:#1f2328}section{margin:1em 0;padding:1em;border-bottom:1px solid #eee;display:flex;gap:2em;align-items:flex-start}pre{font-size:11px;color:#888;width:300px;flex:none;white-space:pre-wrap}.dark{background:#16181d;color:#e6e6e6}</style>");
    for case in cases.split("```mermaid").skip(1) {
        let source = case.split("```").next().unwrap_or("").trim();
        let drawn = docstream_diagram::svg(source).unwrap_or_else(|| "<b>not laid out</b>".into());
        page += &format!("<section><pre>{}</pre><div>{drawn}</div></section>", source.replace('<', "&lt;"));
    }
    std::fs::write(out, page).expect("the page");
}
