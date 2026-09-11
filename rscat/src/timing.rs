use std::time::Duration;

/// Timing multipliers shaping how long each word is shown, as multiples of the
/// plain per-word time (`60/wpm`).
///
/// These are heuristics, not a published standard. The punctuation and length
/// defaults follow `pasky/speedread` (MIT, 2014), a widely used terminal
/// Spritz-alike: <https://github.com/pasky/speedread>. Its `word_time` uses
/// `2.0` for a trailing `[:;,]`, `3.0` for a trailing `[.?!]` and adds
/// `0.04*sqrt(len)` for word length. The rarity default is calibrated from the
/// word-frequency effect (see [`DEFAULT_RARITY`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    /// Multiplier for a word ending in a clause separator (`,` `;` `:`).
    pub clause: f64,
    /// Multiplier for a word ending in a sentence terminator (`.` `?` `!`).
    pub sentence: f64,
    /// Added for word length, times `sqrt(word length)`.
    pub length: f64,
    /// Added for word rarity, times [`rarity_penalty`].
    pub rarity: f64,
}

/// Default clause-pause multiplier; `pasky/speedread` uses `2.0` for `[:;,]`.
pub const DEFAULT_CLAUSE_PAUSE: f64 = 1.5;

/// Default sentence-pause multiplier; `pasky/speedread` uses `3.0` for `[.?!]`.
pub const DEFAULT_SENTENCE_PAUSE: f64 = 2.0;

/// Default length factor, matching `pasky/speedread`'s `$lentime = 0.04`
/// (`$time += sqrt(length($word)) * $lentime`). Set to `0.0` to disable.
pub const DEFAULT_LENGTH_FACTOR: f64 = 0.04;

/// Default rarity factor. Calibrated so the rarest word in a text is held
/// roughly 60 ms longer at 300 WPM, matching the ~50-60 ms word-frequency
/// effect on gaze duration (Rayner 1998, summarized in
/// <https://pmc.ncbi.nlm.nih.gov/articles/PMC2715992>). Set to `0.0` to
/// disable.
pub const DEFAULT_RARITY: f64 = 0.3;

/// Multiplier for a word with no trailing sentence/clause punctuation and no
/// length or rarity term.
pub const NORMAL_PAUSE: f64 = 1.0;

impl Default for Pace {
    fn default() -> Self {
        Self {
            clause: DEFAULT_CLAUSE_PAUSE,
            sentence: DEFAULT_SENTENCE_PAUSE,
            length: DEFAULT_LENGTH_FACTOR,
            rarity: DEFAULT_RARITY,
        }
    }
}

/// Milliseconds each word stays on screen at the given speed, before any
/// pause, length or rarity scaling.
///
/// `wpm` is constrained to a positive range by the CLI, so the division cannot
/// be by zero through `Args`.
pub fn ms_per_word(wpm: u32) -> u64 {
    60_000 / u64::from(wpm)
}

/// How long `word` should stay on screen: the plain per-word time scaled by its
/// punctuation, length and rarity multipliers. `count` is how many times `word`
/// occurs in the text and `max_count` the largest such count.
pub fn word_delay(word: &str, wpm: u32, pace: Pace, count: u32, max_count: u32) -> Duration {
    Duration::from_millis(ms_per_word(wpm)).mul_f64(word_multiplier(word, pace, count, max_count))
}

/// The full display-time multiplier for `word`.
///
/// Length and rarity both estimate how hard a word is. Because word length and
/// word frequency are correlated, the two terms are combined with `max` rather
/// than added, so a rare long word is not penalised twice.
pub fn word_multiplier(word: &str, pace: Pace, count: u32, max_count: u32) -> f64 {
    let length_term = pace.length * (word.chars().count() as f64).sqrt();
    let rarity_term = pace.rarity * rarity_penalty(count, max_count);
    pause_multiplier(word, pace) + length_term.max(rarity_term)
}

/// How rare `count` occurrences are in a text whose most frequent word occurs
/// `max_count` times, in `0.0..=1.0`: the most frequent word scores `0.0` and
/// an unseen word `1.0`, on a log (Zipf-like) scale.
///
/// Texts with no repeated word (`max_count <= 1`) score `0.0` throughout, so
/// short or all-unique texts are not uniformly slowed down.
pub fn rarity_penalty(count: u32, max_count: u32) -> f64 {
    if max_count <= 1 {
        0.0
    } else {
        1.0 - (1.0 + f64::from(count)).log2() / (1.0 + f64::from(max_count)).log2()
    }
}

/// Classifies a word by its trailing punctuation, mirroring speedread's
/// `/[.?!]\W*$/` and `/[:;,]\W*$/`: any `.!?` in the characters after the last
/// word character makes it a sentence end, otherwise any `,;:` makes it a
/// clause end, otherwise it is a plain word.
pub fn pause_multiplier(word: &str, pace: Pace) -> f64 {
    let run = trailing_run(word);
    if run.contains(['.', '?', '!']) {
        pace.sentence
    } else if run.contains([',', ';', ':']) {
        pace.clause
    } else {
        NORMAL_PAUSE
    }
}

/// The part of `word` up to and including its last word character (`\w`:
/// alphanumeric or `_`), or `""` when it has none.
pub fn word_core(word: &str) -> &str {
    match word
        .char_indices()
        .rfind(|(_, c)| c.is_alphanumeric() || *c == '_')
    {
        Some((i, c)) => &word[..i + c.len_utf8()],
        None => "",
    }
}

/// The characters after the last word character. For a word with no word
/// characters at all, the whole string is the run.
fn trailing_run(word: &str) -> &str {
    &word[word_core(word).len()..]
}

/// Formats a duration as `m:ss`, or `h:mm:ss` once it reaches an hour.
pub fn format_eta(duration: Duration) -> String {
    let total = duration.as_secs();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours == 0 {
        format!("{minutes}:{seconds:02}")
    } else {
        format!("{hours}:{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_per_word_at_common_speeds() {
        assert_eq!(ms_per_word(300), 200);
        assert_eq!(ms_per_word(600), 100);
        assert_eq!(ms_per_word(150), 400);
    }

    #[test]
    fn pause_multiplier_classifies_trailing_punctuation() {
        let p = Pace::default();
        assert_eq!(pause_multiplier("dog", p), NORMAL_PAUSE);
        assert_eq!(pause_multiplier("dog,", p), p.clause);
        assert_eq!(pause_multiplier("dog;", p), p.clause);
        assert_eq!(pause_multiplier("dog:", p), p.clause);
        assert_eq!(pause_multiplier("dog.", p), p.sentence);
        assert_eq!(pause_multiplier("dog?", p), p.sentence);
        assert_eq!(pause_multiplier("dog!", p), p.sentence);
        // Trailing closers/quotes do not hide the punctuation.
        assert_eq!(pause_multiplier("dog.\"", p), p.sentence);
        assert_eq!(pause_multiplier("dog,\"", p), p.clause);
        assert_eq!(pause_multiplier("(dog.)", p), p.sentence);
        // Internal punctuation is not a boundary.
        assert_eq!(pause_multiplier("foo.bar", p), NORMAL_PAUSE);
        assert_eq!(pause_multiplier("3.14", p), NORMAL_PAUSE);
        // Abbreviations also end in a period (same as speedread).
        assert_eq!(pause_multiplier("e.g.", p), p.sentence);
    }

    #[test]
    fn word_delay_applies_punctuation_multipliers() {
        // Length and rarity disabled so the punctuation multiples are exact.
        let p = Pace {
            length: 0.0,
            rarity: 0.0,
            ..Pace::default()
        };
        assert_eq!(word_delay("dog", 300, p, 1, 1), Duration::from_millis(200));
        assert_eq!(word_delay("dog,", 300, p, 1, 1), Duration::from_millis(300));
        assert_eq!(word_delay("dog.", 300, p, 1, 1), Duration::from_millis(400));
        assert_eq!(word_delay("dog", 600, p, 1, 1), Duration::from_millis(100));
    }

    #[test]
    fn word_delay_adds_sqrt_length_term() {
        // A clean 0.5 factor keeps the arithmetic exact: 1 + 0.5*sqrt(len).
        let p = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 0.5,
            rarity: 0.0,
        };
        assert_eq!(word_delay("a", 300, p, 1, 1), Duration::from_millis(300)); // 1 + .5*1
        assert_eq!(word_delay("abcd", 300, p, 1, 1), Duration::from_millis(400)); // 1 + .5*2
        assert_eq!(
            word_delay("abcdefghi", 300, p, 1, 1),
            Duration::from_millis(500)
        ); // 1 + .5*3
    }

    #[test]
    fn rarity_penalty_is_exact_at_its_bounds() {
        assert_eq!(rarity_penalty(3, 3), 0.0); // most frequent word
        assert_eq!(rarity_penalty(0, 3), 1.0); // unseen word
        assert_eq!(rarity_penalty(1, 3), 0.5); // log2(2) / log2(4)
        assert_eq!(rarity_penalty(5, 1), 0.0); // nothing is relatively rare
        assert_eq!(rarity_penalty(5, 0), 0.0);
    }

    #[test]
    fn word_multiplier_takes_the_max_of_length_and_rarity() {
        // 1-char word: length term is exactly 1.0, rarity term 0.5, so max = 1.0
        // (adding them would give 2.5).
        let both = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 1.0,
            rarity: 1.0,
        };
        assert_eq!(word_multiplier("a", both, 1, 3), 2.0);

        // Rarity only: the rare word gets the rarity term.
        let rarity_only = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 0.0,
            rarity: 1.0,
        };
        assert_eq!(word_multiplier("a", rarity_only, 1, 3), 1.5);

        // A frequent word gets neither term.
        assert_eq!(word_multiplier("a", rarity_only, 3, 3), 1.0);
    }

    #[test]
    fn format_eta_under_an_hour() {
        assert_eq!(format_eta(Duration::from_secs(0)), "0:00");
        assert_eq!(format_eta(Duration::from_secs(5)), "0:05");
        assert_eq!(format_eta(Duration::from_secs(65)), "1:05");
        assert_eq!(format_eta(Duration::from_secs(3599)), "59:59");
    }

    #[test]
    fn format_eta_over_an_hour() {
        assert_eq!(format_eta(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(format_eta(Duration::from_secs(3661)), "1:01:01");
    }
}
