//! Word frequencies derived from the text being read.
//!
//! A word like a character's name appears in no dictionary, yet is frequent
//! within its book; counting occurrences in the input handles such words and
//! needs no language data. This mirrors the within-text repetition effect,
//! where repeated words are read faster (Raney & Rayner, 1995; and e.g.
//! <https://link.springer.com/article/10.3758/s13423-021-02054-0>).

use std::collections::HashMap;

use crate::timing;

/// Total number of times each word occurs in `words`, aligned with the input,
/// plus the largest count. Words are normalized first (case-folded, surrounding
/// punctuation stripped), so `Aragorn` and `aragorn.` count as one word.
pub fn analyze(words: &[String]) -> (Vec<u32>, u32) {
    let keys: Vec<String> = words.iter().map(|word| normalize(word)).collect();

    let mut counts: HashMap<&str, u32> = HashMap::new();
    for key in &keys {
        *counts.entry(key.as_str()).or_insert(0) += 1;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    let per_word = keys.iter().map(|key| counts[key.as_str()]).collect();

    (per_word, max_count)
}

/// Case-folds `word` and drops the non-alphanumeric characters around its core.
fn normalize(word: &str) -> String {
    let core = timing::word_core(word);
    let start = core
        .find(|c: char| c.is_alphanumeric())
        .unwrap_or(core.len());
    core[start..].to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn analyze_counts_case_and_punctuation_insensitively() {
        let (counts, max) = analyze(&words(&["Aragorn", "the", "aragorn.", "Aragorn!", "the"]));
        assert_eq!(counts, vec![3, 2, 3, 3, 2]);
        assert_eq!(max, 3);
    }

    #[test]
    fn analyze_ignores_surrounding_punctuation_like_guillemets() {
        let (counts, max) = analyze(&words(&["« Bonjour", "bonjour"]));
        assert_eq!(counts, vec![2, 2]);
        assert_eq!(max, 2);
    }

    #[test]
    fn analyze_of_a_text_with_no_repeats() {
        let (counts, max) = analyze(&words(&["one", "two", "three"]));
        assert_eq!(counts, vec![1, 1, 1]);
        assert_eq!(max, 1);
    }
}
