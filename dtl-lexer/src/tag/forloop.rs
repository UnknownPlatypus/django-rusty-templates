use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::TemplateContent;
use crate::common::{LexerError, text_content_at, translated_text_content_at};
use crate::cursor::Cursor;
use crate::python_compat::is_whitespace;
use crate::tag::TagParts;
use crate::types::{At, TemplateString};

#[derive(Clone, Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum ForLexerError {
    #[error(transparent)]
    LexerError(#[from] LexerError),
    #[error("Invalid variable name {name} in for loop:")]
    InvalidName {
        name: String,
        #[label("invalid variable name")]
        at: SourceSpan,
    },
    #[error("Expected an expression after the 'in' keyword:")]
    MissingExpression {
        #[label("after this keyword")]
        at: SourceSpan,
    },
    #[error("Unexpected expression in for loop:")]
    UnexpectedExpression {
        #[label("unexpected expression")]
        at: SourceSpan,
    },
    #[error("Unexpected comma in for loop:")]
    #[diagnostic(help("Try removing the comma, or adding a variable name before it"))]
    UnexpectedComma {
        #[label("here")]
        at: SourceSpan,
    },
}

#[derive(Clone, Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum ForLexerInError {
    #[error("Unexpected expression in for loop. Did you miss a comma when unpacking?")]
    MissingComma {
        #[label("unexpected expression")]
        at: SourceSpan,
    },
    #[error("Expected the 'in' keyword or a variable name:")]
    MissingIn {
        #[label("after this name")]
        at: SourceSpan,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ForTokenType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ForVariableNameToken {
    pub at: At,
}

impl<'t> TemplateContent<'t> for ForVariableNameToken {
    fn content(&self, template: TemplateString<'t>) -> &'t str {
        template.content(self.at)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ForVariableToken {
    pub at: At,
    pub token_type: ForTokenType,
}

impl<'t> TemplateContent<'t> for ForVariableToken {
    fn content(&self, template: TemplateString<'t>) -> &'t str {
        match self.token_type {
            ForTokenType::Numeric => template.content(self.at),
            ForTokenType::Text => template.content(text_content_at(self.at)),
            ForTokenType::TranslatedText => template.content(translated_text_content_at(self.at)),
            ForTokenType::Variable => template.content(self.at),
        }
    }
}

enum State {
    VariableName,
    Done,
}

pub struct ForLexer<'t> {
    template: TemplateString<'t>,
    cursor: Cursor<'t>,
    state: State,
    previous_at: Option<At>,
}

impl<'t> ForLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            template,
            cursor: Cursor::new(template.content(parts.at), parts.at.0),
            state: State::VariableName,
            previous_at: None,
        }
    }

    pub fn lex_expression(&mut self) -> Result<ForVariableToken, ForLexerError> {
        let mut cursor = self.cursor.clone();
        let (token_type, at) = match cursor.first() {
            None => {
                return Err(ForLexerError::MissingExpression {
                    at: self.previous_at.expect("previous_at is set").into(),
                });
            }
            Some('_') if cursor.second() == Some('(') => {
                (ForTokenType::TranslatedText, cursor.lex_translated()?)
            }
            Some('"' | '\'') => (ForTokenType::Text, cursor.lex_text()?),
            Some('0'..='9' | '-') => (ForTokenType::Numeric, cursor.lex_numeric()),
            Some(_) => (ForTokenType::Variable, cursor.lex_variable()),
        };
        self.cursor = cursor;
        self.cursor.lex_remainder()?;
        Ok(ForVariableToken { at, token_type })
    }

    pub fn lex_in(&mut self) -> Result<(), ForLexerInError> {
        if self.cursor.is_empty() {
            return Err(ForLexerInError::MissingIn {
                at: self.previous_at.expect("previous_at is set").into(),
            });
        }
        let mut cursor = self.cursor.clone();
        let at = cursor.eat_while(|c| !is_whitespace(c));
        if self.template.content(at) != "in" {
            return Err(ForLexerInError::MissingComma { at: at.into() });
        }
        cursor.eat_whitespace();
        self.cursor = cursor;
        self.previous_at = Some(at);
        Ok(())
    }

    pub fn lex_reversed(&mut self) -> Result<bool, ForLexerError> {
        let mut cursor = self.cursor.clone();
        if cursor.is_empty() {
            return Ok(false);
        }
        let mut at = cursor.eat_while(|c| !is_whitespace(c));
        if self.template.content(at) == "reversed" {
            cursor.eat_whitespace();
            if cursor.is_empty() {
                return Ok(true);
            }
            at = cursor.skip_to_end();
        }
        Err(ForLexerError::UnexpectedExpression { at: at.into() })
    }

    fn check_invalid_variable_name(name: &str, at: At) -> Result<(), ForLexerError> {
        let mut cursor = Cursor::new(name, at.0);
        cursor.lex_variable();
        if !cursor.is_empty() || name == "." || name.contains(['"', '\'', '|']) {
            return Err(ForLexerError::InvalidName {
                name: name.to_string(),
                at: at.into(),
            });
        }
        Ok(())
    }

    pub fn lex_variable_name(&mut self) -> Option<Result<ForVariableNameToken, ForLexerError>> {
        match self.state {
            State::VariableName if !self.cursor.is_empty() => {}
            State::VariableName => {
                self.state = State::Done;
                return None;
            }
            State::Done => return None,
        }

        if self.cursor.first() == Some(',') {
            let at = (self.cursor.pos(), 1);
            return Some(Err(ForLexerError::UnexpectedComma { at: at.into() }));
        }
        let at = self.cursor.eat_while(|c| c != ',' && !is_whitespace(c));
        self.previous_at = Some(at);

        if let Err(err) = Self::check_invalid_variable_name(self.template.content(at), at) {
            self.cursor.skip_to_end();
            self.state = State::Done;
            return Some(Err(err));
        }

        self.cursor.eat_whitespace();
        if self.cursor.first() == Some(',') {
            self.cursor.bump();
            self.cursor.eat_whitespace();
        } else {
            self.state = State::Done;
        }
        Some(Ok(ForVariableNameToken { at }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple() {
        let template = "{% for foo in bar %}";
        let parts = TagParts { at: (7, 10) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 3),
            token_type: ForTokenType::Variable,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_text() {
        let template = "{% for foo in 'bar' %}";
        let parts = TagParts { at: (7, 12) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 5),
            token_type: ForTokenType::Text,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_text_double_quotes() {
        let template = "{% for foo in \"bar\" %}";
        let parts = TagParts { at: (7, 12) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 5),
            token_type: ForTokenType::Text,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_translated_text() {
        let template = "{% for foo in _('bar') %}";
        let parts = TagParts { at: (7, 15) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 8),
            token_type: ForTokenType::TranslatedText,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_underscore_expression() {
        let template = "{% for foo in _bar %}";
        let parts = TagParts { at: (7, 11) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 4),
            token_type: ForTokenType::Variable,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_int() {
        let template = "{% for foo in 123 %}";
        let parts = TagParts { at: (7, 10) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 3),
            token_type: ForTokenType::Numeric,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_variable_names() {
        let template = "{% for foo, bar in spam %}";
        let parts = TagParts { at: (7, 16) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableNameToken { at: (12, 3) };
        let spam = ForVariableToken {
            at: (19, 4),
            token_type: ForTokenType::Variable,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), bar);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), spam);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_variable_names_no_whitespace_after_comma() {
        let template = "{% for foo,bar in spam %}";
        let parts = TagParts { at: (7, 15) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableNameToken { at: (11, 3) };
        let spam = ForVariableToken {
            at: (18, 4),
            token_type: ForTokenType::Variable,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), bar);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), spam);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_comma_in_text() {
        let template = "{% for foo in 'spam,' %}";
        let parts = TagParts { at: (7, 14) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let spam = ForVariableToken {
            at: (14, 7),
            token_type: ForTokenType::Text,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), spam);
        assert!(!lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_lex_reversed() {
        let template = "{% for foo in bar reversed %}";
        let parts = TagParts { at: (7, 19) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 3),
            token_type: ForTokenType::Variable,
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert!(lexer.lex_reversed().unwrap());
    }

    #[test]
    fn test_unexpected_before_in() {
        let template = "{% for foo bar in bar reversed %}";
        let parts = TagParts { at: (7, 23) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let unexpected = ForLexerInError::MissingComma { at: (11, 3).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        assert_eq!(lexer.lex_in().unwrap_err(), unexpected);
    }

    #[test]
    fn test_unexpected_after_iterable() {
        let template = "{% for foo in bar invalid %}";
        let parts = TagParts { at: (7, 18) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 3),
            token_type: ForTokenType::Variable,
        };
        let unexpected = ForLexerError::UnexpectedExpression { at: (18, 7).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert_eq!(lexer.lex_reversed().unwrap_err(), unexpected);
    }

    #[test]
    fn test_unexpected_after_reversed() {
        let template = "{% for foo in bar reversed invalid %}";
        let parts = TagParts { at: (7, 27) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let bar = ForVariableToken {
            at: (14, 3),
            token_type: ForTokenType::Variable,
        };
        let unexpected = ForLexerError::UnexpectedExpression { at: (27, 7).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap(), bar);
        assert_eq!(lexer.lex_reversed().unwrap_err(), unexpected);
    }

    #[test]
    fn test_incomplete_string() {
        let template = "{% for foo in 'bar %}";
        let parts = TagParts { at: (7, 11) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let incomplete = LexerError::IncompleteString { at: (14, 4).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap_err(), incomplete.into());
    }

    #[test]
    fn test_incomplete_translated_string() {
        let template = "{% for foo in _('bar' %}";
        let parts = TagParts { at: (7, 14) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let incomplete = LexerError::IncompleteTranslatedString { at: (14, 7).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap_err(), incomplete.into());
    }

    #[test]
    fn test_invalid_remainder() {
        let template = "{% for foo in 'bar'baz %}";
        let parts = TagParts { at: (7, 15) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let foo = ForVariableNameToken { at: (7, 3) };
        let incomplete = LexerError::InvalidRemainder { at: (19, 3).into() };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap(), foo);
        lexer.lex_in().unwrap();
        assert_eq!(lexer.lex_expression().unwrap_err(), incomplete.into());
    }

    #[test]
    fn test_invalid_name() {
        let template = "{% for '2' in 'bar' %}";
        let parts = TagParts { at: (7, 12) };
        let mut lexer = ForLexer::new(template.into(), parts);

        let invalid = ForLexerError::InvalidName {
            name: "'2'".to_string(),
            at: (7, 3).into(),
        };
        assert_eq!(lexer.lex_variable_name().unwrap().unwrap_err(), invalid);
    }
}
