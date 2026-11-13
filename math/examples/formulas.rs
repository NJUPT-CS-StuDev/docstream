//! Every case the golden tests hold, drawn on one page to look at: `cargo run -p docstream-math --example formulas -- out.html`.
fn main() {
    let out = std::env::args().nth(1).expect("where to write the page");
    let cases = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/cases.tex")).expect("the cases");
    let mut page = String::from("<!doctype html><meta charset=utf-8><style>body{font:20px/1.6 'Latin Modern Roman',serif;max-width:900px;margin:2em auto;color:#222}div{margin:.6em 0;padding:.4em;border-bottom:1px solid #eee}code{font-size:12px;color:#888;display:block}</style>");
    for line in cases.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('%')) {
        page += &format!("<div><code>{}</code>文字 {} 文字<br><center>{}</center></div>", line.replace('<', "&lt;"), docstream_math::svg(line, false), docstream_math::svg(line, true));
    }
    std::fs::write(out, page).expect("the page");
}
