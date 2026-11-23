use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::cursor::Cursor;
use crate::python_compat::is_whitespace;
use crate::types::At;
use crate::{END_TRANSLATE_LEN, QUOTE_LEN, START_TRANSLATE_LEN};

#[derive(Clone, Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum LexerError {
    #[error("Expected a complete string literal")]
    IncompleteString {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a complete translation string")]
    IncompleteTranslatedString {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a valid variable name")]
    InvalidVariableName {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Could not parse the remainder")]
    InvalidRemainder {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a string literal within translation")]
    MissingTranslatedString {
        #[label("here")]
        at: SourceSpan,
    },
}

impl Cursor<'_> {
    /// Lexes a variable with its filters.
    /// Stops at the first character outside a string literal that can be part of neither.
    pub(crate) fn lex_variable(&mut self) -> At {
        let mut in_text = None;
        self.eat_while(|c| {
            match c {
                '"' | '\'' => match in_text {
                    None => in_text = Some(c),
                    Some(quote) if quote == c => in_text = None,
                    Some(_) => {}
                },
                _ if in_text.is_some() => {}
                _ => return c.is_xid_continue() || matches!(c, '.' | '|' | ':' | '-'),
            }
            true
        })
    }

    /// Lexes a string literal, starting at its opening quote.
    pub(crate) fn lex_text(&mut self) -> Result<At, LexerError> {
        let start = self.pos();
        let quote = self.bump();
        loop {
            match self.bump() {
                None => {
                    let at = self.at_from(start);
                    return Err(LexerError::IncompleteString { at: at.into() });
                }
                Some('\\') => {
                    self.bump();
                }
                next if next == quote => return Ok(self.at_from(start)),
                Some(_) => {}
            }
        }
    }

    /// Lexes a translated string literal, starting at its `_(`.
    pub(crate) fn lex_translated(&mut self) -> Result<At, LexerError> {
        let start = self.pos();
        self.bump_bytes(START_TRANSLATE_LEN);
        match self.first() {
            Some('\'' | '"') => {
                self.lex_text()?;
            }
            _ => {
                self.skip_to_end();
                let at = self.at_from(start);
                return Err(LexerError::MissingTranslatedString { at: at.into() });
            }
        }
        if self.first() == Some(')') {
            self.bump_bytes(END_TRANSLATE_LEN);
            Ok(self.at_from(start))
        } else {
            let at = self.at_from(start);
            Err(LexerError::IncompleteTranslatedString { at: at.into() })
        }
    }

    /// Lexes a number, starting at its first digit or sign.
    pub(crate) fn lex_numeric(&mut self) -> At {
        let start = self.pos();
        self.bump();
        // Match django bug: a `-` after the first character ends the number.
        self.eat_while(|c| c.is_ascii_digit() || c == '.' || c == 'e');
        self.at_from(start)
    }

    pub(crate) fn lex_variable_argument(&mut self) -> Result<At, LexerError> {
        let content = trim_variable(self.rest());
        check_variable_attrs(content, self.pos())?;
        Ok(self.bump_bytes(content.len()))
    }

    /// Skips the whitespace after a token.
    ///
    /// Errors without consuming anything when anything else directly follows the token.
    pub(crate) fn lex_remainder(&mut self) -> Result<(), LexerError> {
        match self.clone().eat_while(|c| !is_whitespace(c)) {
            (_, 0) => {
                self.eat_whitespace();
                Ok(())
            }
            at => Err(LexerError::InvalidRemainder { at: at.into() }),
        }
    }
}

// Shims for the lexers not yet ported to `Cursor`.

pub(crate) fn lex_variable(byte: usize, rest: &str) -> (At, usize, &str) {
    let mut cursor = Cursor::new(rest, byte);
    let at = cursor.lex_variable();
    (at, cursor.pos(), cursor.rest())
}

pub(crate) fn lex_text<'t>(
    byte: usize,
    rest: &'t str,
    _chars: &mut std::str::Chars,
    _end: char,
) -> Result<(At, usize, &'t str), LexerError> {
    let mut cursor = Cursor::new(rest, byte);
    let at = cursor.lex_text()?;
    Ok((at, cursor.pos(), cursor.rest()))
}

pub(crate) fn lex_translated<'t>(
    byte: usize,
    rest: &'t str,
    _chars: &mut std::str::Chars,
) -> Result<(At, usize, &'t str), LexerError> {
    let mut cursor = Cursor::new(rest, byte);
    let at = cursor.lex_translated()?;
    Ok((at, cursor.pos(), cursor.rest()))
}

pub(crate) fn lex_numeric(byte: usize, rest: &str) -> (At, usize, &str) {
    let mut cursor = Cursor::new(rest, byte);
    let at = cursor.lex_numeric();
    (at, cursor.pos(), cursor.rest())
}

pub fn trim_variable(variable: &str) -> &str {
    match variable.find(|c: char| !c.is_xid_continue() && c != '.' && c != '-') {
        Some(end) => &variable[..end],
        None => variable,
    }
}

pub fn check_variable_attrs(variable: &str, start: usize) -> Result<(), LexerError> {
    let mut offset = 0;
    for (i, var) in variable.split('.').enumerate() {
        if i == 0 {
            let mut chars = var.chars();
            chars.next();
            if chars.any(|c| c == '-') {
                let at = (start + offset, var.len());
                return Err(LexerError::InvalidVariableName { at: at.into() });
            }
        } else if var.find('-').is_some() {
            let at = (start + offset, var.len());
            return Err(LexerError::InvalidVariableName { at: at.into() });
        }

        match var.chars().next() {
            Some(c) if c != '_' => {
                offset += var.len() + 1;
            }
            _ => {
                let at = (start + offset, var.len());
                return Err(LexerError::InvalidVariableName { at: at.into() });
            }
        }
    }
    Ok(())
}

pub fn text_content_at(at: At) -> At {
    let (start, len) = at;
    let start = start + QUOTE_LEN;
    let len = len - 2 * QUOTE_LEN;
    (start, len)
}

pub fn translated_text_content_at(at: At) -> At {
    let (start, len) = at;
    let start = start + START_TRANSLATE_LEN + QUOTE_LEN;
    let len = len - START_TRANSLATE_LEN - END_TRANSLATE_LEN - 2 * QUOTE_LEN;
    (start, len)
}

/// Returns a new `At` representing the span from the beginning of `start`
/// to the end of `end`.
pub fn get_all_at(start: At, end: At) -> At {
    assert!(
        end.0 >= start.0,
        "End position must be greater than or equal to start position"
    );
    (start.0, end.0 - start.0 + end.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_text_non_ascii() {
        let template = "'N\u{ec655}'";
        let mut cursor = Cursor::new(template, 1);
        let at = cursor.lex_text().unwrap();
        assert_eq!(at, (1, 7));
        assert_eq!(cursor.pos(), 8);
        assert_eq!(cursor.rest(), "");
    }

    #[test]
    fn test_lex_argument_non_ascii() {
        let template = "ZJ5G4YXZJUH6|default:\"#`´କ¯\"";
        let mut cursor = Cursor::new(template, 0);
        let at = cursor.lex_variable();
        assert_eq!(at, (0, 32));
        assert_eq!(cursor.pos(), 32);
        assert_eq!(cursor.rest(), "");
    }

    #[test]
    fn test_get_all_at_valid() {
        let start = (10, 5);
        let end = (20, 3);
        assert_eq!(get_all_at(start, end), (10, 13));
    }

    #[test]
    #[should_panic(expected = "End position must be greater than or equal to start position")]
    fn test_get_all_at_invalid_range() {
        let start = (20, 5);
        let end = (10, 3);
        get_all_at(start, end);
    }
}
