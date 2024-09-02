use std::{
    // fs::File,
    io::{Cursor, Read},
};

use lz4_flex::frame::FrameDecoder;
use once_cell::sync::Lazy;
use regex::Regex;
use vibrato::{Dictionary, Tokenizer};

const DICT: &[u8] = include_bytes!("../dictionary/system.dic.lz4");

pub struct FuriganaGenerator {
    tokenizer: Tokenizer,
}

impl FuriganaGenerator {
    pub fn new() -> Self {
        let dict = {
            // Note: we could just pass the decoder straight to `Dictionary::read()`
            // below, and it would work.  However, that ends up being slower than
            // first decompressing the whole thing ahead of time.
            let mut decoder = FrameDecoder::new(Cursor::new(DICT));
            let mut data = Vec::new();
            decoder.read_to_end(&mut data).unwrap();

            Dictionary::read(Cursor::new(&data)).unwrap()
        };
        Self {
            tokenizer: Tokenizer::new(dict),
        }
    }

    pub fn add_html_furigana(&self, text: &str) -> String {
        add_html_furigana_skip_already_ruby(&text, &self.tokenizer)
    }
}

/// Like `add_html_furigana()`, but skips text that already has ruby on it, to it doesn't get double-ruby.
fn add_html_furigana_skip_already_ruby(text: &str, tokenizer: &Tokenizer) -> String {
    static ALREADY_RUBY: Lazy<Regex> = Lazy::new(|| Regex::new(r"<ruby.*?>.*?</ruby>").unwrap());

    let mut new_text = String::new();
    let mut last_byte_index = 0;
    for hit in ALREADY_RUBY.find_iter(text) {
        new_text.push_str(&add_html_furigana(
            &text[last_byte_index..hit.start()],
            tokenizer,
        ));
        new_text.push_str(hit.as_str());
        last_byte_index = hit.end();
    }

    new_text.push_str(&add_html_furigana(&text[last_byte_index..], tokenizer));

    new_text
}

/// Adds furigana to Japanese text, using html ruby tags.
fn add_html_furigana(text: &str, tokenizer: &Tokenizer) -> String {
    let mut worker = tokenizer.new_worker();

    worker.reset_sentence(text);
    worker.tokenize();

    let mut new_text = String::new();
    for i in 0..worker.num_tokens() {
        let t = worker.token(i);
        let surface = t.surface();
        let kana = t.feature().split(",").nth(1).unwrap();

        let (start_bytes, end_bytes) = matching_kana_ends(surface, kana);

        if kana.is_empty()
            || start_bytes == surface.len()
            || surface
                .chars()
                .map(|c| c.is_ascii() || c.is_numeric())
                .all(|n| n)
        {
            new_text.push_str(surface);
        } else {
            let start = &surface[..start_bytes];
            let mid = &surface[start_bytes..(surface.len() - end_bytes)];
            let mid_kana = &kana[start_bytes..(kana.len() - end_bytes)];
            let end = &surface[(surface.len() - end_bytes)..];
            new_text.push_str(start);
            new_text.push_str("<ruby>");
            new_text.push_str(mid);
            new_text.push_str("<rt>");
            new_text.push_str(mid_kana);
            new_text.push_str("</rt></ruby>");
            new_text.push_str(end);
        }
    }

    new_text
}

/// Returns (matching_start_bytes, matching_end_bytes).
///
/// Note that the bytes are in terms of `a`'s bytes.
///
/// If `matching_start_bytes == a.len()` you can assume that strings are kana
/// equivalents, and thus no ruby is needed.
fn matching_kana_ends(a: &str, b: &str) -> (usize, usize) {
    let mut start_bytes = 0;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca == cb || is_equivalent_kana(ca, cb) {
            start_bytes += ca.len_utf8();
        } else {
            break;
        }
    }

    let mut end_bytes = 0;
    for (ca, cb) in a.chars().rev().zip(b.chars().rev()) {
        if ca == cb || is_equivalent_kana(ca, cb) {
            end_bytes += ca.len_utf8();
        } else {
            break;
        }
    }

    if (start_bytes + end_bytes) >= a.len() || (start_bytes + end_bytes) >= b.len() {
        (a.len(), 0)
    } else {
        (start_bytes, end_bytes)
    }
}

/// Due to the way this is used, this isn't meant to be exact, but instead
/// liberal in what it considers equivalent.
fn is_equivalent_kana(a: char, b: char) -> bool {
    const PAIRS: &[[char; 2]] = &[['は', 'わ'], ['を', 'お'], ['づ', 'ず'], ['へ', 'え']];
    const VOWELS: &[char] = &['あ', 'い', 'う', 'え', 'お', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ'];

    let (a, b) = match (normalize_kana(a), normalize_kana(b)) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };

    if a == b {
        return true;
    }

    if a == 'ー' && VOWELS.contains(&b) {
        return true;
    }

    if b == 'ー' && VOWELS.contains(&a) {
        return true;
    }

    for &[c, d] in PAIRS {
        if (a == c && b == d) || (a == d && b == c) {
            return true;
        }
    }

    false
}

const HIRAGANA: u32 = 0x3041;
const KATAKANA: u32 = 0x30A1;
const KANA_COUNT: u32 = 0x3097 - HIRAGANA;

pub fn is_kana(c: char) -> bool {
    if c == 'ー' {
        return true;
    }

    let c = c as u32;

    if c >= HIRAGANA && c < (HIRAGANA + KANA_COUNT) {
        return true;
    }

    if c >= KATAKANA && c < (KATAKANA + KANA_COUNT) {
        return true;
    }

    return false;
}

pub fn normalize_kana(c: char) -> Option<char> {
    if !is_kana(c) {
        return None;
    }

    Some(katakana_to_hiragana(c).unwrap_or(c))
}

pub fn hiragana_to_katakana(c: char) -> Option<char> {
    let c = c as u32;
    if c >= HIRAGANA && c < (HIRAGANA + KANA_COUNT) {
        char::try_from(c + KATAKANA - HIRAGANA).ok()
    } else {
        None
    }
}

pub fn katakana_to_hiragana(c: char) -> Option<char> {
    let c = c as u32;
    if c >= KATAKANA && c < (KATAKANA + KANA_COUNT) {
        char::try_from(c - KATAKANA + HIRAGANA).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_kana_ends_01() {
        let surface = "へぇ";
        let kana = "ヘー";
        let (start_bytes, end_bytes) = matching_kana_ends(surface, kana);

        assert_eq!(6, start_bytes);
        assert_eq!(0, end_bytes);
    }

    #[test]
    fn matching_kana_ends_02() {
        let surface = "へぇー";
        let kana = "ヘー";
        let (start_bytes, end_bytes) = matching_kana_ends(surface, kana);

        assert_eq!(9, start_bytes);
        assert_eq!(0, end_bytes);
    }

    #[test]
    fn is_equivalent_kana_01() {
        assert!(is_equivalent_kana('か', 'カ'));
        assert!(is_equivalent_kana('カ', 'か'));
        assert!(is_equivalent_kana('ぁ', 'ァ'));
        assert!(is_equivalent_kana('ァ', 'ぁ'));
        assert!(is_equivalent_kana('は', 'わ'));
        assert!(is_equivalent_kana('わ', 'は'));
        assert!(is_equivalent_kana('を', 'お'));
        assert!(is_equivalent_kana('お', 'を'));
        assert!(is_equivalent_kana('づ', 'ず'));
        assert!(is_equivalent_kana('ず', 'づ'));
        assert!(is_equivalent_kana('ー', 'あ'));
        assert!(is_equivalent_kana('あ', 'ー'));
        assert!(is_equivalent_kana('ー', 'ぁ'));
        assert!(is_equivalent_kana('ぁ', 'ー'));

        assert!(!is_equivalent_kana('は', 'ば'));
        assert!(!is_equivalent_kana('ー', 'か'));
        assert!(!is_equivalent_kana('た', '食'));
    }

    #[test]
    fn tokenize_01() {
        let gen = FuriganaGenerator::new();

        let mut worker = gen.tokenizer.new_worker();
        worker.reset_sentence("食べている");
        worker.tokenize();

        assert_eq!(3, worker.num_tokens());
        assert_eq!("食べ", worker.token(0).surface());
        assert_eq!("動詞-一般,タベ", worker.token(0).feature());
        assert_eq!("て", worker.token(1).surface());
        assert_eq!("助詞-接続助詞,テ", worker.token(1).feature());
        assert_eq!("いる", worker.token(2).surface());
        assert_eq!("動詞-非自立可能,イル", worker.token(2).feature());
    }
}
