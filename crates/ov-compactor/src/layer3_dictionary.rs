//! Layer 3: Dictionary encoding — high-frequency phrase → short code mapping.

use std::collections::{HashMap, HashSet};

const MIN_FREQ: usize = 3;
const MIN_PHRASE_LEN: usize = 6;
const MAX_CODEBOOK: usize = 200;
const DOLLAR_ESCAPE: &str = "\x00DLR\x00";

/// Generate N short codes: $AA..$ZZ, then $AAA..
pub fn generate_codes(n: usize) -> Vec<String> {
    let mut codes = Vec::new();
    for i in 0..26u8 {
        for j in 0..26u8 {
            codes.push(format!("${}{}", (b'A' + i) as char, (b'A' + j) as char));
            if codes.len() >= n { return codes; }
        }
    }
    for i in 0..26u8 {
        for j in 0..26u8 {
            for k in 0..26u8 {
                codes.push(format!("${}{}{}", (b'A' + i) as char, (b'A' + j) as char, (b'A' + k) as char));
                if codes.len() >= n { return codes; }
            }
        }
    }
    codes
}

/// Extract word n-grams.
fn tokenize_ngrams(text: &str, min_n: usize, max_n: usize) -> HashMap<String, usize> {
    let mut counter: HashMap<String, usize> = HashMap::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    for n in min_n..=max_n {
        for window in words.windows(n) {
            let gram = window.join(" ");
            if gram.len() >= MIN_PHRASE_LEN {
                *counter.entry(gram).or_insert(0) += 1;
            }
        }
    }
    counter
}

/// Build a codebook from text samples.
pub fn build_codebook(texts: &[&str]) -> HashMap<String, String> {
    build_codebook_with_params(texts, MIN_FREQ, MAX_CODEBOOK)
}

/// Build codebook with custom parameters.
pub fn build_codebook_with_params(
    texts: &[&str],
    min_freq: usize,
    max_entries: usize,
) -> HashMap<String, String> {
    if texts.is_empty() {
        return HashMap::new();
    }

    let mut combined: HashMap<String, usize> = HashMap::new();
    for text in texts {
        for (gram, count) in tokenize_ngrams(text, 2, 5) {
            *combined.entry(gram).or_insert(0) += count;
        }
    }

    let mut candidates: Vec<(String, usize)> = combined
        .into_iter()
        .filter(|(phrase, count)| *count >= min_freq && phrase.len() >= MIN_PHRASE_LEN)
        .collect();
    candidates.sort_by(|a, b| (b.1 * b.0.len()).cmp(&(a.1 * a.0.len())));

    let codes = generate_codes(candidates.len().min(max_entries));
    let mut codebook = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();

    for ((phrase, _), code) in candidates.iter().zip(codes.iter()) {
        let skip = used.iter().any(|existing| phrase.contains(existing.as_str()) || existing.contains(phrase.as_str()));
        if skip { continue; }
        codebook.insert(code.clone(), phrase.clone());
        used.insert(phrase.clone());
        if codebook.len() >= max_entries { break; }
    }

    codebook
}

/// Compress text using codebook {code -> phrase}.
pub fn compress(text: &str, codebook: &HashMap<String, String>) -> String {
    if text.is_empty() || codebook.is_empty() {
        return text.to_string();
    }
    let mut result = text.replace('$', DOLLAR_ESCAPE);
    let mut sorted: Vec<_> = codebook.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (code, phrase) in sorted {
        let escaped_phrase = phrase.replace('$', DOLLAR_ESCAPE);
        result = result.replace(&escaped_phrase, code);
    }
    result
}

/// Decompress text using codebook.
pub fn decompress(text: &str, codebook: &HashMap<String, String>) -> String {
    if text.is_empty() || codebook.is_empty() {
        return text.to_string();
    }
    let mut result = text.to_string();
    let mut sorted: Vec<_> = codebook.iter().collect();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    for (code, phrase) in sorted {
        result = result.replace(code.as_str(), phrase);
    }
    result = result.replace(DOLLAR_ESCAPE, "$");
    result
}
