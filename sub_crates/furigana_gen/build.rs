// Generate table for traversal order of quad BVHs.

use std::{env, fs::File, io::Write, path::Path};

const KANJI: &str = include_str!("data/kanji_frequency.txt");

fn main() {
    // Write traversal table to Rust file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("kanji_freq_inc.rs");
    let mut f = File::create(&dest_path).unwrap();

    f.write_all("const KANJI_FREQ: &[char] = &[".as_bytes())
        .unwrap();

    for c in KANJI.chars() {
        if c.is_whitespace() {
            continue;
        }

        f.write_all(format!("\n'{}',", c).as_bytes()).unwrap();
    }

    f.write_all("\n];".as_bytes()).unwrap();
}
