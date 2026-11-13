//! The tree each fixture is read into, kept beside it; `UPDATE_GOLDEN=1 cargo test` writes it anew after a deliberate change.
use docstream_core::{read, Stream};
use std::fs;
use std::path::Path;

fn fixtures() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut all: Vec<_> = fs::read_dir(&dir).unwrap().filter_map(|entry| {
        let path = entry.ok()?.path();
        (path.extension()? == "md").then(|| (path.file_stem().unwrap().to_string_lossy().into_owned(), fs::read_to_string(&path).unwrap().replace("\r\n", "\n")))
    }).collect();
    all.sort();
    all
}

#[test]
fn every_fixture_reads_into_its_kept_tree() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (name, text) in fixtures() {
        let got = serde_json::to_string_pretty(&read(&text)).unwrap() + "\n";
        let kept = dir.join(format!("{name}.json"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() { fs::write(&kept, &got).unwrap(); continue; }
        assert_eq!(got, fs::read_to_string(&kept).unwrap().replace("\r\n", "\n"), "{name}");
    }
}

#[test]
fn streamed_a_character_at_a_time_a_settled_block_never_changes_and_the_end_is_the_whole() {
    for (name, text) in fixtures() {
        let mut stream = Stream::new();
        let mut settled: Vec<String> = Vec::new();
        let mut before: Vec<docstream_core::Keyed> = Vec::new();
        for ch in text.chars() {
            let update = stream.push(&ch.to_string());
            let now: Vec<&str> = update.blocks.iter().filter(|block| block.settled).map(|block| block.key.as_str()).collect();
            if !(now.len() >= settled.len() && now[..settled.len()] == settled.iter().map(String::as_str).collect::<Vec<_>>()[..]) {
                let at = (0..settled.len()).find(|&i| now.get(i) != Some(&settled[i].as_str())).unwrap_or(0);
                panic!("{name}: settled block {at} changed after {:?}\nwas: {}\nnow: {}", stream.text().chars().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect::<String>(), serde_json::to_string(&before[at]).unwrap(), serde_json::to_string(&update.blocks.get(at)).unwrap());
            }
            settled = now.into_iter().map(String::from).collect();
            before = update.blocks.clone();
        }
        let last = stream.finish();
        assert!(last.done && last.blocks.iter().all(|block| block.settled), "{name}");
        assert_eq!(last.blocks.into_iter().map(|block| block.block).collect::<Vec<_>>(), read(&text), "{name}: the end is not the whole");
    }
}
