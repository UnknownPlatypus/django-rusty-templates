use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::common::{LexerError, get_all_at, text_content_at, translated_text_content_at};
use crate::cursor::Cursor;
use crate::python_compat::is_whitespace;
use crate::tag::TagParts;
use crate::tag::common::TagElementTokenType;
use crate::types::{At, TemplateString};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagElementKwargToken {
    pub at: At,
    pub token_type: TagElementTokenType,
    pub kwarg: Option<At>,
}

impl TagElementKwargToken {
    pub fn content_at(&self) -> At {
        match self.token_type {
            TagElementTokenType::Variable => self.at,
            TagElementTokenType::Numeric => self.at,
            TagElementTokenType::Text => text_content_at(self.at),
            TagElementTokenType::TranslatedText => translated_text_content_at(self.at),
        }
    }

    /// The location of the full keyword argument:
    ///     keyword=variable
    ///             ────┬───
    ///                 ╰── `self.at`
    ///     ───┬───
    ///        ╰── `self.kwarg`
    ///     ────────┬───────
    ///             ╰── `self.all_at()`
    pub fn all_at(&self) -> At {
        match self.kwarg {
            None => self.at,
            Some(kwarg_at) => get_all_at(kwarg_at, self.at),
        }
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum TagElementKwargLexerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),
    #[error("Incomplete keyword argument")]
    IncompleteKeywordArgument {
        #[label("here")]
        at: SourceSpan,
    },
}

pub struct TagElementKwargLexer<'t> {
    cursor: Cursor<'t>,
}

impl<'t> TagElementKwargLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            cursor: Cursor::new(template.content(parts.at), parts.at.0),
        }
    }

    fn lex_kwarg(&mut self) -> Option<At> {
        let mut cursor = self.cursor.clone();
        let at = cursor.eat_while(UnicodeXID::is_xid_continue);
        if cursor.bump() != Some('=') {
            return None;
        }
        self.cursor = cursor;
        Some(at)
    }

    fn lex_element(&mut self) -> Result<TagElementKwargToken, TagElementKwargLexerError> {
        let kwarg = self.lex_kwarg();
        let cursor = &mut self.cursor;
        let (token_type, at) = match cursor.first() {
            Some('_') if cursor.second() == Some('(') => (
                TagElementTokenType::TranslatedText,
                cursor.lex_translated()?,
            ),
            Some('"' | '\'') => (TagElementTokenType::Text, cursor.lex_text()?),
            Some('0'..='9' | '-') => (TagElementTokenType::Numeric, cursor.lex_numeric()),
            Some(c) if !is_whitespace(c) => (TagElementTokenType::Variable, cursor.lex_variable()),
            _ => {
                let at = kwarg.expect("kwarg is Some or we'd already have exited");
                let at = (at.0, at.1 + 1).into();
                return Err(TagElementKwargLexerError::IncompleteKeywordArgument { at });
            }
        };
        cursor.lex_remainder()?;
        Ok(TagElementKwargToken {
            at,
            token_type,
            kwarg,
        })
    }
}

impl Iterator for TagElementKwargLexer<'_> {
    type Item = Result<TagElementKwargToken, TagElementKwargLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }
        let token = self.lex_element();
        if token.is_err() {
            self.cursor.skip_to_end();
        }
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_url_name_text() {
        let template = "{% url 'foo' %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 5),
            token_type: TagElementTokenType::Text,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_double_quotes() {
        let template = "{% url \"foo\" %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 5),
            token_type: TagElementTokenType::Text,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_incomplete() {
        let template = "{% url 'foo %}";
        let parts = TagParts { at: (7, 4) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::IncompleteString { at: (7, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_name_variable() {
        let template = "{% url foo %}";
        let parts = TagParts { at: (7, 3) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 3),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter() {
        let template = "{% url foo|default:'home' %}";
        let parts = TagParts { at: (7, 18) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 18),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_double_quote() {
        let template = "{% url foo|default:'home\"' %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 19),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_single_quote() {
        let template = "{% url foo|default:\"home'\" %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 19),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_whitespace() {
        let template = "{% url foo|default:'home url' %}";
        let parts = TagParts { at: (7, 22) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 22),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_leading_underscore() {
        let template = "{% url _foo %}";
        let parts = TagParts { at: (7, 4) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 4),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated() {
        let template = "{% url _('foo') %}";
        let parts = TagParts { at: (7, 8) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 8),
            token_type: TagElementTokenType::TranslatedText,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated_incomplete() {
        let template = "{% url _('foo' %}";
        let parts = TagParts { at: (7, 7) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::IncompleteTranslatedString { at: (7, 7).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_name_numeric() {
        let template = "{% url 5 %}";
        let parts = TagParts { at: (7, 1) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (7, 1),
            token_type: TagElementTokenType::Numeric,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_kwarg() {
        let template = "{% url name='foo' %}";
        let parts = TagParts { at: (7, 10) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 5),
            token_type: TagElementTokenType::Text,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_kwarg_double_quotes() {
        let template = "{% url name=\"foo\" %}";
        let parts = TagParts { at: (7, 10) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 5),
            token_type: TagElementTokenType::Text,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_variable_kwarg() {
        let template = "{% url name=foo %}";
        let parts = TagParts { at: (7, 8) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 3),
            token_type: TagElementTokenType::Variable,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_leading_underscore_kwarg() {
        let template = "{% url name=_foo %}";
        let parts = TagParts { at: (7, 9) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 4),
            token_type: TagElementTokenType::Variable,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated_kwarg() {
        let template = "{% url name=_('foo') %}";
        let parts = TagParts { at: (7, 13) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 8),
            token_type: TagElementTokenType::TranslatedText,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_numeric_kwarg() {
        let template = "{% url name=5 %}";
        let parts = TagParts { at: (7, 6) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementKwargToken {
            at: (12, 1),
            token_type: TagElementTokenType::Numeric,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url() {
        let template = "{% url 'home' next %}";
        let parts = TagParts { at: (7, 11) };
        let lexer = TagElementKwargLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let home = TagElementKwargToken {
            at: (7, 6),
            token_type: TagElementTokenType::Text,
            kwarg: None,
        };
        let next = TagElementKwargToken {
            at: (14, 4),
            token_type: TagElementTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(home), Ok(next)]);
    }

    #[test]
    fn test_lex_url_incomplete_kwarg() {
        let template = "{% url name= %}";
        let parts = TagParts { at: (7, 5) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            TagElementKwargLexerError::IncompleteKeywordArgument { at: (7, 5).into() }
        );
    }

    #[test]
    fn test_lex_url_incomplete_kwarg_args() {
        let template = "{% url name= foo %}";
        let parts = TagParts { at: (7, 9) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            TagElementKwargLexerError::IncompleteKeywordArgument { at: (7, 5).into() }
        );
    }

    #[test]
    fn test_lex_url_invalid_remainder() {
        let template = "{% url 'foo'remainder %}";
        let parts = TagParts { at: (7, 14) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::InvalidRemainder { at: (12, 9).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_kwarg_invalid_remainder() {
        let template = "{% url name='foo'=remainder %}";
        let parts = TagParts { at: (7, 20) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::InvalidRemainder {
                at: (17, 10).into()
            }
            .into()
        );
    }

    #[test]
    fn test_lex_url_incomplete_kwarg_message() {
        let template = "{% url name= %}";
        let parts = TagParts { at: (7, 5) };
        let mut lexer = TagElementKwargLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error.to_string(), "Incomplete keyword argument");
    }
}
