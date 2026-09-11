use std::time::Duration;

/// Timing multipliers shaping how long each word is shown, as multiples of the
/// plain per-word time (`60/wpm`).
///
/// These are heuristics, not a published standard. The defaults follow
/// `pasky/speedread` (MIT, 2014), a widely used terminal Spritz-alike:
/// <https://github.com/pasky/speedread>. Its `word_time` uses `2.0` for a
/// trailing `[:;,]`, `3.0` for a trailing `[.?!]` and adds `0.04*sqrt(len)` for
/// word length. We take the same shape but gentler punctuation defaults,
/// exposed as CLI flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    /// Multiplier for a word ending in a clause separator (`,` `;` `:`).
    pub clause: f64,
    /// Multiplier for a word ending in a sentence terminator (`.` `?` `!`).
    pub sentence: f64,
    /// Added to every word's multiplier, times `sqrt(word length)`.
    pub length: f64,
}

/// Default clause-pause multiplier; `pasky/speedread` uses `2.0` for `[:;,]`.
pub const DEFAULT_CLAUSE_PAUSE: f64 = 1.5;

/// Default sentence-pause multiplier; `pasky/speedread` uses `3.0` for `[.?!]`.
pub const DEFAULT_SENTENCE_PAUSE: f64 = 2.0;

/// Default length factor, matching `pasky/speedread`'s `$lentime = 0.04`
/// (`$time += sqrt(length($word)) * $lentime`). Set to `0.0` to disable.
pub const DEFAULT_LENGTH_FACTOR: f64 = 0.04;

/// Multiplier for a word with no trailing sentence/clause punctuation and no
/// length term.
pub const NORMAL_PAUSE: f64 = 1.0;

impl Default for Pace {
    fn default() -> Self {
        Self {
            clause: DEFAULT_CLAUSE_PAUSE,
            sentence: DEFAULT_SENTENCE_PAUSE,
            length: DEFAULT_LENGTH_FACTOR,
        }
    }
}

/// Milliseconds each word stays on screen at the given speed, before any
/// pause or length scaling.
///
/// `wpm` is constrained to a positive range by the CLI, so the division cannot
/// be by zero through `Args`.
pub fn ms_per_word(wpm: u32) -> u64 {
    60_000 / u64::from(wpm)
}

/// How long `word` should stay on screen: the plain per-word time scaled by its
/// punctuation and length multipliers.
pub fn word_delay(word: &str, wpm: u32, pace: Pace) -> Duration {
    Duration::from_millis(ms_per_word(wpm)).mul_f64(word_multiplier(word, pace))
}

/// The full display-time multiplier for `word`: punctuation pause plus the
/// `sqrt(length)` term.
pub fn word_multiplier(word: &str, pace: Pace) -> f64 {
    pause_multiplier(word, pace) + pace.length * (word.chars().count() as f64).sqrt()
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

/// The characters after the last word character (`\w`: alphanumeric or `_`).
/// For a word with no word characters at all, the whole string is the run.
fn trailing_run(word: &str) -> &str {
    match word
        .char_indices()
        .rfind(|(_, c)| c.is_alphanumeric() || *c == '_')
    {
        Some((i, c)) => &word[i + c.len_utf8()..],
        None => word,
    }
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
        // Length term disabled so the punctuation multiples are exact.
        let p = Pace {
            length: 0.0,
            ..Pace::default()
        };
        assert_eq!(word_delay("dog", 300, p), Duration::from_millis(200));
        assert_eq!(word_delay("dog,", 300, p), Duration::from_millis(300));
        assert_eq!(word_delay("dog.", 300, p), Duration::from_millis(400));
        assert_eq!(word_delay("dog", 600, p), Duration::from_millis(100));
    }

    #[test]
    fn word_delay_adds_sqrt_length_term() {
        // A clean 0.5 factor keeps the arithmetic exact: 1 + 0.5*sqrt(len).
        let p = Pace {
            clause: 1.0,
            sentence: 1.0,
            length: 0.5,
        };
        assert_eq!(word_delay("a", 300, p), Duration::from_millis(300)); // 1 + .5*1
        assert_eq!(word_delay("abcd", 300, p), Duration::from_millis(400)); // 1 + .5*2
        assert_eq!(word_delay("abcdefghi", 300, p), Duration::from_millis(500)); // 1 + .5*3
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
