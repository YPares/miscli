use std::time::Duration;

use crate::frequency;
use crate::timing::{self, Pace};

/// Reading state: the word list, current position, speed and timing config,
/// plus the in-text occurrence counts used for the rarity factor.
pub struct Reader {
    words: Vec<String>,
    index: usize,
    wpm: u32,
    pace: Pace,
    /// Occurrences of each word in the whole text, aligned with `words`.
    counts: Vec<u32>,
    /// The largest value in `counts`.
    max_count: u32,
}

impl Reader {
    pub fn new(words: Vec<String>, wpm: u32, pace: Pace) -> Self {
        let (counts, max_count) = frequency::analyze(&words);
        Self {
            words,
            index: 0,
            wpm,
            pace,
            counts,
            max_count,
        }
    }

    pub fn current(&self) -> Option<&String> {
        self.words.get(self.index)
    }

    /// The word shown just before the current one, if any.
    pub fn previous(&self) -> Option<&String> {
        self.index.checked_sub(1).and_then(|i| self.words.get(i))
    }

    /// Advance one word. Unlike [`retreat`] and [`step_forward`], this
    /// deliberately walks off the end: playback ends when
    /// [`current`](Self::current) becomes `None`.
    pub fn advance(&mut self) {
        self.index += 1;
    }

    /// Step back one word, stopping at the first.
    pub fn retreat(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    /// Step forward one word, stopping at the last (unlike [`advance`], which
    /// walks off the end to signal the text is finished).
    pub fn step_forward(&mut self) {
        self.index = (self.index + 1).min(self.words.len().saturating_sub(1));
    }

    /// Increase the reading speed by `step` wpm, up to [`timing::MAX_WPM`].
    pub fn faster(&mut self, step: u32) {
        self.wpm = (self.wpm + step).min(timing::MAX_WPM);
    }

    /// Decrease the reading speed by `step` wpm, down to [`timing::MIN_WPM`].
    pub fn slower(&mut self, step: u32) {
        self.wpm = self.wpm.saturating_sub(step).max(timing::MIN_WPM);
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

    /// How long the current word should stay on screen, including punctuation,
    /// length and rarity scaling. Zero once the text is exhausted.
    pub fn tick(&self) -> Duration {
        self.tick_at(self.index)
    }

    /// How long the *previous* word would be shown for at the current speed;
    /// zero at the start of the text. Used to size that word's ghost.
    pub fn previous_tick(&self) -> Duration {
        match self.index.checked_sub(1) {
            Some(index) => self.tick_at(index),
            None => Duration::ZERO,
        }
    }

    /// Display time of the word at `index`, or zero if there is none.
    fn tick_at(&self, index: usize) -> Duration {
        match self.words.get(index) {
            Some(word) => timing::word_delay(
                word,
                self.wpm,
                self.pace,
                self.counts[index],
                self.max_count,
            ),
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

    /// Estimated time left at the configured speed. This ignores all pauses and
    /// scaling, so it is a lower bound on the real remaining time.
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
    fn previous_tracks_the_word_before_the_current_one() {
        let mut r = reader(&["one", "two"], 300);
        assert_eq!(r.previous(), None);
        r.advance();
        assert_eq!(r.previous().map(String::as_str), Some("one"));
        r.advance();
        assert_eq!(r.previous().map(String::as_str), Some("two"));
    }

    #[test]
    fn retreat_and_step_forward_stay_within_the_text() {
        let mut r = reader(&["a", "b", "c"], 300);
        r.retreat(); // already at the first word
        assert_eq!(r.current().map(String::as_str), Some("a"));
        r.step_forward();
        assert_eq!(r.current().map(String::as_str), Some("b"));
        r.step_forward();
        r.step_forward(); // clamps at the last word
        assert_eq!(r.current().map(String::as_str), Some("c"));
        r.retreat();
        assert_eq!(r.current().map(String::as_str), Some("b"));
    }

    #[test]
    fn faster_and_slower_clamp_within_bounds() {
        let mut r = reader(&["a"], 300);
        r.faster(10);
        assert_eq!(r.wpm(), 310);
        r.slower(20);
        assert_eq!(r.wpm(), 290);
        r.slower(1000);
        assert_eq!(r.wpm(), timing::MIN_WPM);
        r.faster(100_000);
        assert_eq!(r.wpm(), timing::MAX_WPM);
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
        // Length and rarity disabled so the multiples are exact.
        let pace = Pace {
            length: 0.0,
            rarity: 0.0,
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
            rarity: 0.0,
        };
        // 1 + 0.5*sqrt(4) = 2.0 -> 400ms at 300 wpm.
        let r = Reader::new(vec!["abcd".to_string()], 300, pace);
        assert_eq!(r.tick(), Duration::from_millis(400));
    }

    #[test]
    fn tick_slows_down_words_that_are_rare_in_the_text() {
        let pace = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 0.0,
            rarity: 1.0,
        };
        // "rare" occurs once where the max is 3, so rarity_penalty = 0.5 and the
        // multiplier is 1.5 -> 300ms at 300 wpm.
        let words: Vec<String> = ["rare", "common", "common", "common"]
            .iter()
            .map(|w| (*w).to_string())
            .collect();
        let r = Reader::new(words, 300, pace);
        assert_eq!(r.tick(), Duration::from_millis(300));
    }

    #[test]
    fn previous_tick_is_the_previous_words_display_time() {
        // Length and rarity off so only the punctuation pause matters.
        let pace = Pace {
            length: 0.0,
            rarity: 0.0,
            ..Pace::default()
        };
        let words: Vec<String> = ["dog.", "cat"].iter().map(|w| (*w).to_string()).collect();
        let mut r = Reader::new(words, 300, pace);
        assert_eq!(r.previous_tick(), Duration::ZERO); // no previous word yet
        r.advance();
        assert_eq!(r.previous_tick(), Duration::from_millis(400)); // "dog." is 2x
    }
}
