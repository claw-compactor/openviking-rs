//! Layer 4: Dedup — near-duplicate removal via shingle hashing.

use std::collections::HashSet;

const SHINGLE_SIZE: usize = 3;
const SIMILARITY_THRESHOLD: f64 = 0.6;

/// Generate k-word shingle hashes.
pub fn shingles(text: &str, k: usize) -> HashSet<u64> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        let mut s = HashSet::new();
        s.insert(0);
        return s;
    }
    if words.len() < k {
        let mut s = HashSet::new();
        s.insert(hash_str(&words.join(" ")));
        return s;
    }
    words.windows(k)
        .map(|w| hash_str(&w.join(" ")))
        .collect()
}

fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Jaccard similarity between two shingle sets.
pub fn jaccard(a: &HashSet<u64>, b: &HashSet<u64>) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    if a.is_empty() || b.is_empty() { return 0.0; }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

/// Duplicate group.
#[derive(Debug, Clone)]
pub struct DupGroup {
    pub indices: Vec<usize>,
    pub similarity: f64,
}

/// Find near-duplicate groups.
pub fn find_duplicates(entries: &[&str]) -> Vec<DupGroup> {
    find_duplicates_with_params(entries, SIMILARITY_THRESHOLD, SHINGLE_SIZE)
}

pub fn find_duplicates_with_params(entries: &[&str], threshold: f64, k: usize) -> Vec<DupGroup> {
    if entries.len() < 2 {
        return Vec::new();
    }

    let shingle_sets: Vec<HashSet<u64>> = entries.iter().map(|e| shingles(e, k)).collect();
    let mut used: HashSet<usize> = HashSet::new();
    let mut groups = Vec::new();

    for i in 0..entries.len() {
        if used.contains(&i) { continue; }
        let mut group = vec![i];
        let mut total_sim = 0.0;
        let mut count = 0;

        for j in (i + 1)..entries.len() {
            if used.contains(&j) { continue; }
            let sim = jaccard(&shingle_sets[i], &shingle_sets[j]);
            if sim >= threshold {
                group.push(j);
                total_sim += sim;
                count += 1;
            }
        }
        if group.len() > 1 {
            let avg = if count > 0 { total_sim / count as f64 } else { threshold };
            groups.push(DupGroup { indices: group.clone(), similarity: avg });
            used.extend(group);
        }
    }
    groups
}

/// Merge duplicates, keeping the longest entry per group.
pub fn merge_duplicates(entries: &[&str], groups: &[DupGroup]) -> Vec<String> {
    if groups.is_empty() {
        return entries.iter().map(|e| e.to_string()).collect();
    }
    let mut removed: HashSet<usize> = HashSet::new();
    for g in groups {
        let best = *g.indices.iter().max_by_key(|&&idx| entries[idx].len()).unwrap();
        for &idx in &g.indices {
            if idx != best { removed.insert(idx); }
        }
    }
    entries.iter().enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(|(_, e)| e.to_string())
        .collect()
}

/// Compress: split by double-newline, dedup, rejoin.
pub fn compress(text: &str) -> String {
    let entries: Vec<&str> = text.split("\n\n").collect();
    if entries.len() < 2 {
        return text.to_string();
    }
    let groups = find_duplicates(&entries);
    merge_duplicates(&entries, &groups).join("\n\n")
}
