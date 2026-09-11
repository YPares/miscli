use std::time::Duration;

/// Milliseconds each word stays on screen at the given speed.
///
/// `wpm` is constrained to a positive range by the CLI, so the division cannot
/// be by zero through `Args`.
pub fn ms_per_word(wpm: u32) -> u64 {
    60_000 / u64::from(wpm)
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
