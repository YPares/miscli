use std::time::Duration;

use crate::timing::{self, Pace};

/// Reading state: the full word list, the current position, the speed and the
/// timing/pause configuration.
pub struct Reader {
    words: Vec<String>,
    index: usize,
    wpm: u32,
    pace: Pace,
}

impl Reader {
    pub fn new(words: Vec<String>, wpm: u32, pace: Pace) -> Self {
        Self {
            words,
            index: 0,
            wpm,
            pace,
        }
    }

    pub fn current(&self) -> Option<&String> {
        self.words.get(self.index)
    }

    pub fn advance(&mut self) {
        self.index += 1;
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn wpm(&self) -> u32 {
        self.wpm
    }

    /// How long the current word should stay on screen, including any
    /// punctuation pause and length scaling. Zero once the text is exhausted.
    pub fn tick(&self) -> Duration {
        match self.current() {
            Some(word) => timing::word_delay(word, self.wpm, self.pace),
            None => Duration::ZERO,
        }
    }

    /// Fraction of the text already streamed, in `0.0..=1.0`.
    pub fn progress_ratio(&self) -> f64 {
        if self.words.is_empty() {
            0.0
        } else {
            self.index as f64 / self.words.len() as f64
        }
    }

    /// Estimated time left at the configured speed. This ignores punctuation
    /// and length pauses, so it is a lower bound on the real remaining time.
    pub fn remaining(&self) -> Duration {
        let remaining_words = self.words.len().saturating_sub(self.index);
        Duration::from_secs_f64(remaining_words as f64 * 60.0 / f64::from(self.wpm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(words: &[&str], wpm: u32) -> Reader {
        Reader::new(
            words.iter().map(|w| (*w).to_string()).collect(),
            wpm,
            Pace::default(),
        )
    }

    #[test]
    fn advances_through_words() {
        let mut r = reader(&["one", "two"], 300);
        assert_eq!(r.current().map(String::as_str), Some("one"));
        r.advance();
        assert_eq!(r.current().map(String::as_str), Some("two"));
        r.advance();
        assert_eq!(r.current(), None);
    }

    #[test]
    fn progress_ratio_is_exact_at_boundaries() {
        let mut r = reader(&["a", "b", "c", "d"], 300);
        assert_eq!(r.progress_ratio(), 0.0);
        for _ in 0..4 {
            r.advance();
        }
        assert_eq!(r.progress_ratio(), 1.0);
    }

    #[test]
    fn progress_ratio_of_empty_reader_is_zero() {
        let r = reader(&[], 300);
        assert_eq!(r.progress_ratio(), 0.0);
    }

    #[test]
    fn remaining_scales_with_speed() {
        // 600 words left at 300 wpm = 2 minutes.
        let r = reader(&vec!["w"; 600], 300);
        assert_eq!(r.remaining(), Duration::from_secs(120));
    }

    #[test]
    fn tick_includes_punctuation_pause() {
        // Length term disabled so the multiples are exact.
        let pace = Pace {
            length: 0.0,
            ..Pace::default()
        };
        let reader = |words: &[&str]| -> Reader {
            Reader::new(words.iter().map(|w| (*w).to_string()).collect(), 300, pace)
        };
        assert_eq!(reader(&["dog"]).tick(), Duration::from_millis(200));
        assert_eq!(reader(&["dog,"]).tick(), Duration::from_millis(300));
        assert_eq!(reader(&["dog."]).tick(), Duration::from_millis(400));
        assert_eq!(reader(&[]).tick(), Duration::ZERO);
    }

    #[test]
    fn tick_scales_with_word_length() {
        let pace = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 0.5,
        };
        // 1 + 0.5*sqrt(4) = 2.0 -> 400ms at 300 wpm.
        let r = Reader::new(vec!["abcd".to_string()], 300, pace);
        assert_eq!(r.tick(), Duration::from_millis(400));
    }
}
