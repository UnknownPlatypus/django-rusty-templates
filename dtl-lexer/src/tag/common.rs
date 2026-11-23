use crate::common::{LexerError, text_content_at, translated_text_content_at};
use crate::cursor::Cursor;
use crate::tag::TagParts;
use crate::types::{At, TemplateString};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagElementTokenType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagElementToken {
    pub at: At,
    pub token_type: TagElementTokenType,
}

impl TagElementToken {
    pub fn content_at(&self) -> At {
        match self.token_type {
            TagElementTokenType::Variable => self.at,
            TagElementTokenType::Numeric => self.at,
            TagElementTokenType::Text => text_content_at(self.at),
            TagElementTokenType::TranslatedText => translated_text_content_at(self.at),
        }
    }
}

pub struct TagElementLexer<'t> {
    cursor: Cursor<'t>,
}

impl<'t> TagElementLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            cursor: Cursor::new(template.content(parts.at), parts.at.0),
        }
    }

    fn lex_element(&mut self) -> Result<TagElementToken, LexerError> {
        let cursor = &mut self.cursor;
        let (token_type, at) = match cursor.first() {
            Some('_') if cursor.second() == Some('(') => (
                TagElementTokenType::TranslatedText,
                cursor.lex_translated()?,
            ),
            Some('"' | '\'') => (TagElementTokenType::Text, cursor.lex_text()?),
            Some('0'..='9' | '-') => (TagElementTokenType::Numeric, cursor.lex_numeric()),
            _ => (TagElementTokenType::Variable, cursor.lex_variable()),
        };
        cursor.lex_remainder()?;
        Ok(TagElementToken { at, token_type })
    }
}

impl Iterator for TagElementLexer<'_> {
    type Item = Result<TagElementToken, LexerError>;

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
    fn test_lex_text() {
        let template = "{% url 'foo' %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 5),
            token_type: TagElementTokenType::Text,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_text_double_quotes() {
        let template = "{% url \"foo\" %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 5),
            token_type: TagElementTokenType::Text,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_text_incomplete() {
        let template = "{% url 'foo %}";
        let parts = TagParts { at: (7, 4) };
        let mut lexer = TagElementLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error, LexerError::IncompleteString { at: (7, 4).into() });
    }

    #[test]
    fn test_lex_variable() {
        let template = "{% url foo %}";
        let parts = TagParts { at: (7, 3) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 3),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_filter() {
        let template = "{% url foo|default:'home' %}";
        let parts = TagParts { at: (7, 18) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 18),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_filter_inner_double_quote() {
        let template = "{% url foo|default:'home\"' %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 19),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_filter_inner_single_quote() {
        let template = "{% url foo|default:\"home'\" %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 19),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_filter_inner_whitespace() {
        let template = "{% url foo|default:'home url' %}";
        let parts = TagParts { at: (7, 22) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 22),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_leading_underscore() {
        let template = "{% url _foo %}";
        let parts = TagParts { at: (7, 4) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 4),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_translated() {
        let template = "{% url _('foo') %}";
        let parts = TagParts { at: (7, 8) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 8),
            token_type: TagElementTokenType::TranslatedText,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_translated_incomplete() {
        let template = "{% url _('foo' %}";
        let parts = TagParts { at: (7, 7) };
        let mut lexer = TagElementLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::IncompleteTranslatedString { at: (7, 7).into() }
        );
    }

    #[test]
    fn test_lex_numeric() {
        let template = "{% url 5 %}";
        let parts = TagParts { at: (7, 1) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = TagElementToken {
            at: (7, 1),
            token_type: TagElementTokenType::Numeric,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_multiple_tokens() {
        let template = "{% url 'home' next %}";
        let parts = TagParts { at: (7, 11) };
        let lexer = TagElementLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let home = TagElementToken {
            at: (7, 6),
            token_type: TagElementTokenType::Text,
        };
        let next = TagElementToken {
            at: (14, 4),
            token_type: TagElementTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(home), Ok(next)]);
    }

    #[test]
    fn test_lex_incomplete_kwarg() {
        let template = "{% url name= %}";
        let parts = TagParts { at: (7, 5) };
        let mut lexer = TagElementLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error, LexerError::InvalidRemainder { at: (11, 1).into() });
    }

    #[test]
    fn test_lex_incomplete_kwarg_args() {
        let template = "{% url name= foo %}";
        let parts = TagParts { at: (7, 9) };
        let mut lexer = TagElementLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error, LexerError::InvalidRemainder { at: (11, 1).into() });
    }

    #[test]
    fn test_lex_invalid_remainder() {
        let template = "{% url 'foo'remainder %}";
        let parts = TagParts { at: (7, 14) };
        let mut lexer = TagElementLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error, LexerError::InvalidRemainder { at: (12, 9).into() });
    }
}
