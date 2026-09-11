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

/// Splits raw text on whitespace. Paragraph and chapter structure is
/// deliberately ignored for now.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
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
    fn tokenize_of_blank_text_is_empty() {
        assert_eq!(tokenize(" \n\t "), Vec::<String>::new());
    }
}
