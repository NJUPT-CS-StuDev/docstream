//! How each case in cases.tex is typeset, kept beside it as layout without outlines; `UPDATE_GOLDEN=1 cargo test`
//! writes it anew after a deliberate change, and the gallery example draws them to look at.
use std::fs;
use std::path::Path;

fn cases() -> Vec<String> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.tex")).unwrap();
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('%')).map(String::from).collect()
}

#[test]
fn every_case_typesets_as_kept() {
    let kept = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.json");
    let got: Vec<serde_json::Value> = cases().iter().flat_map(|tex| [false, true].map(|display| {
        let f = docstream_math::typeset(tex, display);
        serde_json::json!({ "tex": tex, "display": display, "width": f.width, "height": f.height, "depth": f.depth, "items": f.items })
    })).collect();
    let got = serde_json::to_string_pretty(&got).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() { fs::write(&kept, &got).unwrap(); return; }
    assert_eq!(got, fs::read_to_string(&kept).unwrap().replace("\r\n", "\n"));
}

/// A model writes TeX a character at a time; every unfinished piece of it still typesets to something finite.
#[test]
fn every_prefix_of_every_case_typesets() {
    for tex in cases() {
        for end in tex.char_indices().map(|(i, _)| i).chain([tex.len()]) {
            for display in [false, true] {
                let f = docstream_math::typeset(&tex[..end], display);
                assert!(f.width.is_finite() && f.height.is_finite() && f.depth.is_finite(), "{:?}", &tex[..end]);
                for item in &f.items { if let docstream_math::Item::Glyph { id, .. } = item { assert!(f.glyphs.contains_key(id)); } }
            }
        }
    }
}

#[test]
fn what_is_not_understood_is_shown_as_written() {
    let f = docstream_math::typeset(r"\notacommand{x}", false);
    assert!(!f.items.is_empty());
    let svg = docstream_math::svg(r"a<b", true);
    assert!(svg.starts_with("<svg") && svg.contains("<title>a&lt;b</title>"));
}
