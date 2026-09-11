use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;

/// Reads a file, or stdin when `path` is `None`, and splits it into words.
///
/// Refuses to read stdin when it is an interactive terminal, so `rscat` fails
/// loudly instead of hanging on a no-input invocation.
pub fn load_words(path: Option<&Path>) -> io::Result<Vec<String>> {
    match path {
        Some(path) => Ok(tokenize(&fs::read_to_string(path)?)),
        None if io::stdin().is_terminal() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no input: pass a file path or pipe text on stdin",
        )),
        None => Ok(tokenize(&io::read_to_string(io::stdin())?)),
    }
}

/// Punctuation that opens a group. A lone token made only of these binds to the
/// following word (as in French `« mot »`), not the preceding one.
const OPENING_PUNCTUATION: &str = "«‹“‘„‚([{";

/// Splits raw text into display words. Paragraph and chapter structure is
/// deliberately ignored for now.
///
/// A whitespace-separated token with no alphanumeric character is not a word of
/// its own: it is attached to the adjacent word, keeping the separating space
/// that was in the text. This keeps spaced punctuation attached where it
/// belongs — French puts a space before `!` `?` `;` `:`, so `Bonjour !` is one
/// word `"Bonjour !"` rather than `"Bonjour"` followed by a lone `"!"`.
///
/// The separator is normalized to a regular space because `split_whitespace`
/// discards the original character (which may be a non-breaking space).
pub fn tokenize(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut pending_prefix = String::new();

    for token in text.split_whitespace() {
        if is_opening_only(token) {
            extend_prefix(&mut pending_prefix, token);
        } else if is_punctuation_only(token) {
            match words.last_mut() {
                Some(word) => {
                    word.push(' ');
                    word.push_str(token);
                }
                None => extend_prefix(&mut pending_prefix, token),
            }
        } else {
            if pending_prefix.is_empty() {
                words.push(token.to_string());
            } else {
                words.push(format!("{pending_prefix} {token}"));
                pending_prefix.clear();
            }
        }
    }

    // Input made only of punctuation: keep it as one word rather than dropping
    // it entirely.
    if !pending_prefix.is_empty() && words.is_empty() {
        words.push(pending_prefix);
    }

    words
}

/// Appends `token` to an accumulating prefix, separated from it by a space.
fn extend_prefix(prefix: &mut String, token: &str) {
    if !prefix.is_empty() {
        prefix.push(' ');
    }
    prefix.push_str(token);
}

/// A token with no alphanumeric character, i.e. only punctuation/symbols.
///
/// std has no Unicode punctuation test, so this uses the inverse of
/// `is_alphanumeric`; that also attaches things like a lone `+` or `€`.
fn is_punctuation_only(token: &str) -> bool {
    !token.chars().any(|c| c.is_alphanumeric())
}

/// A punctuation-only token made exclusively of opening marks.
fn is_opening_only(token: &str) -> bool {
    token.chars().all(|c| OPENING_PUNCTUATION.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_on_any_whitespace() {
        assert_eq!(
            tokenize("  hello\nworld\t foo "),
            vec!["hello", "world", "foo"]
        );
    }

    #[test]
    fn tokenize_keeps_punctuation_attached() {
        assert_eq!(tokenize("end. Next,"), vec!["end.", "Next,"]);
    }

    #[test]
    fn tokenize_attaches_spaced_french_punctuation() {
        assert_eq!(tokenize("Bonjour !"), vec!["Bonjour !"]);
        assert_eq!(tokenize("Ça va ?"), vec!["Ça", "va ?"]);
        assert_eq!(tokenize("Oui ; non :"), vec!["Oui ;", "non :"]);
    }

    #[test]
    fn tokenize_attaches_guillemets_to_the_quoted_words() {
        assert_eq!(tokenize("« Bonjour ! »"), vec!["« Bonjour ! »"]);
        assert_eq!(
            tokenize("Il dit : « Bonjour »."),
            vec!["Il", "dit :", "« Bonjour »."]
        );
    }

    #[test]
    fn tokenize_of_blank_text_is_empty() {
        assert_eq!(tokenize(" \n\t "), Vec::<String>::new());
    }

    #[test]
    fn tokenize_of_only_punctuation_is_one_word() {
        assert_eq!(tokenize("! ?"), vec!["! ?"]);
    }

    #[test]
    fn tokenize_attaches_spaced_symbols_to_the_preceding_word() {
        // Deliberate trade-off of the simple rule: a lone `+` (or `€`, …) also
        // binds to the preceding word, space included.
        assert_eq!(tokenize("3 + 4"), vec!["3 +", "4"]);
    }
}
