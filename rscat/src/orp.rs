//! Optimal Recognition Point: the letter an RSVP reader anchors each word on.
//!
//! The bucket table is the common Spritz-style heuristic. The underlying
//! principle (longer words have their ORP further left of center) is documented
//! in the RSVP/Spritz literature; the exact bucket boundaries here are a
//! convention that can be tuned later.

/// Returns the character index of the pivot within `word`.
pub fn pivot_index(word: &str) -> usize {
    match word.chars().count() {
        0 => 0,
        1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

/// Splits `word` into the text before the pivot, the pivot character and the
/// text after it. Byte boundaries are derived from `char_indices`, so this is
/// safe for multi-byte UTF-8.
pub fn split_word(word: &str) -> (&str, char, &str) {
    match word.chars().next() {
        None => ("", ' ', ""),
        Some(_) => {
            let pivot = pivot_index(word);
            let start = word.char_indices().nth(pivot).map_or(0, |(i, _)| i);
            let pivot_char = word[start..].chars().next().unwrap_or(' ');
            let end = start + pivot_char.len_utf8();
            (&word[..start], pivot_char, &word[end..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pivot_index_matches_length_buckets() {
        assert_eq!(pivot_index("a"), 0);
        assert_eq!(pivot_index("to"), 1);
        assert_eq!(pivot_index("hello"), 1);
        assert_eq!(pivot_index("coding"), 2);
        assert_eq!(pivot_index("terminal"), 2);
        assert_eq!(pivot_index("recognition"), 3);
        assert_eq!(pivot_index("extraordinary"), 3);
        assert_eq!(pivot_index("internationalization"), 4);
    }

    #[test]
    fn split_word_separates_pivot_character() {
        assert_eq!(split_word("coding"), ("co", 'd', "ing"));
        assert_eq!(split_word("a"), ("", 'a', ""));
    }

    #[test]
    fn split_word_handles_empty_input() {
        assert_eq!(split_word(""), ("", ' ', ""));
    }

    #[test]
    fn split_word_is_utf8_safe() {
        // "déjàvu" is 6 chars, pivot at index 2; the left side ends on the
        // multi-byte "é" and the right side starts on the multi-byte "à", so
        // both slices must land on char boundaries.
        assert_eq!(split_word("déjàvu"), ("dé", 'j', "àvu"));
        assert_eq!(split_word("grüßen"), ("gr", 'ü', "ßen"));
    }
}
