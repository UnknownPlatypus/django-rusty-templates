use miette::{Diagnostic, SourceSpan};
use num_bigint::BigInt;
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::common::{LexerError, check_variable_attrs, trim_variable};
use crate::cursor::Cursor;
use crate::python_compat::{is_whitespace, trim, trim_end, trim_start};
use crate::types::{At, TemplateString};
use crate::{END_TRANSLATE_LEN, QUOTE_LEN, START_TRANSLATE_LEN, TemplateContent};

#[derive(Debug, PartialEq, Eq)]
pub enum Argument {
    Numeric(At),
    Text(At),
    TranslatedText(At),
    Variable(At),
}

impl Argument {
    pub fn content_at(&self) -> At {
        match self {
            Self::Variable(at) => *at,
            Self::Numeric(at) => *at,
            Self::Text(at) => {
                let (start, len) = at;
                let start = start + QUOTE_LEN;
                let len = len - 2 * QUOTE_LEN;
                (start, len)
            }
            Self::TranslatedText(at) => {
                let (start, len) = at;
                let start = start + START_TRANSLATE_LEN + QUOTE_LEN;
                let len = len - START_TRANSLATE_LEN - END_TRANSLATE_LEN - 2 * QUOTE_LEN;
                (start, len)
            }
        }
    }
}

impl<'t> TemplateContent<'t> for Argument {
    fn content(&self, template: TemplateString<'t>) -> &'t str {
        let (start, len) = self.content_at();
        &template.0[start..start + len]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FilterToken {
    pub at: At,
    pub argument: Option<Argument>,
}

impl<'t> TemplateContent<'t> for FilterToken {
    fn content(&self, template: TemplateString<'t>) -> &'t str {
        let (start, len) = self.at;
        &template.0[start..start + len]
    }
}

#[derive(Debug, PartialEq)]
pub enum VariableToken {
    Variable,
    Int(BigInt),
    Float(f64),
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum VariableLexerError {
    #[error("Variables and attributes may not begin with underscores")]
    LeadingUnderscore {
        #[label("here")]
        at: SourceSpan,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),
    #[error("Expected an argument")]
    MissingArgument {
        #[label("after this")]
        at: SourceSpan,
    },
    #[error("Expected a valid filter name")]
    InvalidFilterName {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a valid variable name")]
    InvalidVariableName {
        #[label("here")]
        at: SourceSpan,
    },
}

pub fn lex_variable_or_filter(
    variable: &str,
    start: usize,
) -> Result<Option<(VariableToken, At, FilterLexer<'_>)>, VariableLexerError> {
    let rest = trim_start(variable);
    if rest.is_empty() {
        return Ok(None);
    }

    let start = start + variable.len() - rest.len();
    let content = trim_variable(rest);

    if content.is_empty() {
        let at = (start, trim(rest).len());
        return Err(VariableLexerError::InvalidVariableName { at: at.into() });
    }

    let end = content.len();
    let at = (start, end);

    let token_type;
    if let Ok(num) = content.parse::<BigInt>() {
        token_type = VariableToken::Int(num);
    } else if let Ok(num) = content.parse::<f64>()
        && num.is_finite()
    {
        token_type = VariableToken::Float(num);
    } else {
        check_variable_attrs(content, start)?;
        token_type = VariableToken::Variable;
    }

    Ok(Some((
        token_type,
        at,
        FilterLexer::new(&rest[end..], start + end)?,
    )))
}

#[derive(Debug)]
pub struct FilterLexer<'t> {
    cursor: Cursor<'t>,
}

impl<'t> FilterLexer<'t> {
    fn new(variable: &'t str, start: usize) -> Result<Self, LexerError> {
        let mut cursor = Cursor::new(trim_end(variable), start);
        cursor.eat_whitespace();
        match cursor.first() {
            None => {}
            Some('|') => {
                cursor.bump();
                cursor.eat_whitespace();
                if cursor.is_empty() {
                    let at = (start, 1);
                    return Err(LexerError::InvalidRemainder { at: at.into() });
                }
            }
            Some(_) => {
                let rest = cursor.rest();
                let remainder = &rest[..rest.find('|').unwrap_or(rest.len())];
                let at = (cursor.pos(), trim_end(remainder).len());
                return Err(LexerError::InvalidRemainder { at: at.into() });
            }
        }
        Ok(Self { cursor })
    }

    fn lex_filter(&mut self) -> Result<FilterToken, VariableLexerError> {
        self.cursor.eat_whitespace();
        if !self.cursor.first().is_some_and(UnicodeXID::is_xid_start) {
            let rest = self.cursor.rest();
            let at = (self.cursor.pos(), rest.find('|').unwrap_or(rest.len()));
            return Err(VariableLexerError::InvalidFilterName { at: at.into() });
        }
        let at = self.cursor.eat_while(UnicodeXID::is_xid_continue);
        let (remainder, _start_next) = self.remainder_to_filter_or_argument();
        if !remainder.is_empty() {
            let at = (self.cursor.pos(), trim(remainder).len());
            return Err(LexerError::InvalidRemainder { at: at.into() }.into());
        }
        let argument = self.lex_argument()?;
        Ok(FilterToken { at, argument })
    }

    fn lex_argument(&mut self) -> Result<Option<Argument>, VariableLexerError> {
        let cursor = &mut self.cursor;
        // The caller checked that nothing separates the filter name from a `:` or `|`.
        if cursor.first() != Some(':') {
            return Ok(None);
        }
        let colon_at = cursor.bump_bytes(1);
        Ok(Some(match cursor.first() {
            None => {
                return Err(VariableLexerError::MissingArgument {
                    at: colon_at.into(),
                });
            }
            Some('_') if cursor.second() == Some('(') => {
                Argument::TranslatedText(cursor.lex_translated()?)
            }
            Some('_') => {
                let at = cursor.eat_while(|c| !is_whitespace(c));
                return Err(VariableLexerError::LeadingUnderscore { at: at.into() });
            }
            Some('\'' | '"') => Argument::Text(cursor.lex_text()?),
            Some('0'..='9' | '-') => Argument::Numeric(cursor.lex_numeric()),
            Some(_) => Argument::Variable(cursor.lex_variable_argument()?),
        }))
    }

    fn lex_remainder(&mut self, token: FilterToken) -> Result<FilterToken, VariableLexerError> {
        let (remainder, start_next) = self.remainder_to_filter_or_argument();
        match remainder.find(|c| !is_whitespace(c)) {
            None => {
                self.cursor.bump_bytes(start_next);
                Ok(token)
            }
            Some(n) => {
                let at = (self.cursor.pos() + n, trim(remainder).len());
                Err(LexerError::InvalidRemainder { at: at.into() }.into())
            }
        }
    }

    /// The source before the next `|` or `:`, and the offset after that separator.
    fn remainder_to_filter_or_argument(&self) -> (&'t str, usize) {
        let rest = self.cursor.rest();
        match rest.find(['|', ':']) {
            None => (rest, rest.len()),
            Some(n) => (&rest[..n], n + 1),
        }
    }
}

impl Iterator for FilterLexer<'_> {
    type Item = Result<FilterToken, VariableLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }
        let token = self
            .lex_filter()
            .and_then(|token| self.lex_remainder(token));
        if token.is_err() {
            self.cursor.skip_to_end();
        }
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::IntoTemplateString;
    use crate::{END_TAG_LEN, START_TAG_LEN};

    fn contents(
        template: &str,
        tokens: Vec<Result<FilterToken, VariableLexerError>>,
    ) -> Vec<(&str, Option<&str>)> {
        let template = template.into_template_string();
        tokens
            .iter()
            .map(|t| match t {
                Ok(t) => match t.argument {
                    Some(ref a) => (t.content(template), Some(a.content(template))),
                    None => (t.content(template), None),
                },
                Err(_) => unreachable!(),
            })
            .collect()
    }

    fn trim_variable(template: &str) -> &str {
        &template[START_TAG_LEN..(template.len() - END_TAG_LEN)]
    }

    #[test]
    fn test_lex_empty() {
        let variable = "  ";
        assert!(
            lex_variable_or_filter(variable, START_TAG_LEN)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn test_lex_variable() {
        let template = "{{ foo.bar }}";
        let variable = trim_variable(template);
        let (token, at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        assert_eq!(token, VariableToken::Variable);
        assert_eq!(at, (3, 7));
        assert_eq!(TemplateString(template).content(at), "foo.bar");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_index() {
        let template = "{{ 1 }}";
        let variable = trim_variable(template);
        let (token, at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        assert_eq!(token, VariableToken::Int(1.into()));
        assert_eq!(TemplateString(template).content(at), "1");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_negative_index() {
        let template = "{{ -1 }}";
        let variable = trim_variable(template);
        let (token, at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        assert_eq!(token, VariableToken::Int((-1).into()));
        assert_eq!(TemplateString(template).content(at), "-1");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_start_underscore() {
        let variable = " _foo.bar ";
        let err = lex_variable_or_filter(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (3, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_attribute_start_underscore() {
        let variable = " foo._bar ";
        let err = lex_variable_or_filter(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (7, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_attribute_index() {
        let template = "{{ foo.1 }}";
        let variable = trim_variable(template);
        let (token, at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        assert_eq!(token, VariableToken::Variable);
        assert_eq!(at, (3, 5));
        assert_eq!(TemplateString(template).content(at), "foo.1");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_attribute_negative_index() {
        let template = "{{ foo.-1 }}";
        let variable = trim_variable(template);
        let err = lex_variable_or_filter(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (7, 2).into() }.into()
        );
    }

    #[test]
    fn test_lex_filter() {
        let template = "{{ foo.bar|title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                at: (11, 5),
                argument: None,
            })]
        );
        assert_eq!(contents(template, tokens), vec![("title", None)]);
    }

    #[test]
    fn test_lex_filter_empty() {
        let template = "{{ foo.bar| }}";
        let variable = trim_variable(template);
        let err = lex_variable_or_filter(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidRemainder { at: (10, 1).into() }.into()
        );
    }

    #[test]
    fn test_lex_filter_chain() {
        let template = "{{ foo.bar|title|length }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: None,
                    at: (11, 5),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (17, 6),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("title", None), ("length", None)]
        );
    }

    #[test]
    fn test_lex_filter_remainder() {
        let template = "{{ foo.bar|title'foo' }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (16, 5).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_filter_invalid_start() {
        let template = "{{ foo.bar|'foo' }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidFilterName {
                at: (11, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_single_quote() {
        let template = "{{ foo.bar|default:'foo' }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Text((19, 5))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_text_argument_double_quote() {
        let template = "{{ foo.bar|default:\"foo\" }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Text((19, 5))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_text_argument_escaped() {
        let template = "{{ foo.bar|default:'foo\\\'' }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Text((19, 7))),
                at: (11, 7),
            })]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("foo\\\'"))]
        );
    }

    #[test]
    fn test_lex_translated_text_argument() {
        let template = "{{ foo.bar|default:_('foo') }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::TranslatedText((19, 8))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_translated_text_argument_double_quoted() {
        let template = "{{ foo.bar|default:_(\"foo\") }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::TranslatedText((19, 8))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_numeric_argument() {
        let template = "{{ foo.bar|default:500 }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Numeric((19, 3))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("500"))]);
    }

    #[test]
    fn test_lex_numeric_argument_negative() {
        let template = "{{ foo.bar|default:-0.5 }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Numeric((19, 4))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("-0.5"))]);
    }

    #[test]
    fn test_lex_numeric_argument_scientific() {
        let template = "{{ foo.bar|default:5.2e3 }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Numeric((19, 5))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("5.2e3"))]);
    }

    #[test]
    fn test_lex_numeric_argument_scientific_negative_exponent() {
        // Django mishandles this case, so we do too:
        // https://code.djangoproject.com/ticket/35816
        let template = "{{ foo.bar|default:5.2e-3 }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Err(LexerError::InvalidRemainder { at: (23, 2).into() }.into()),
                /* When fixed we can do:
                Ok(FilterToken {
                    argument: Some(Argument::Numeric((19, 6))),
                    at: (11, 7),
                })
                */
            ]
        );
        //assert_eq!(contents(template, tokens), vec![("default", Some("5.2e-3"))]);
    }

    #[test]
    fn test_lex_variable_argument() {
        let template = "{{ foo.bar|default:spam }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument::Variable((19, 4))),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("spam"))]);
    }

    #[test]
    fn test_lex_variable_argument_then_filter() {
        let template = "{{ foo.bar|default:spam|title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument::Variable((19, 4))),
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (24, 5),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("spam")), ("title", None)]
        );
    }

    #[test]
    fn test_lex_string_argument_then_filter() {
        let template = "{{ foo.bar|default:\"spam\"|title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument::Text((19, 6))),
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (26, 5),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("spam")), ("title", None)]
        );
    }

    #[test]
    fn test_lex_argument_with_leading_underscore() {
        let template = "{{ foo.bar|default:_spam }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::LeadingUnderscore {
                at: (19, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_argument_with_attribute_underscore() {
        let template = "{{ foo.bar|default:spam._eggs }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidVariableName { at: (24, 5).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_argument_with_only_underscore() {
        let template = "{{ foo.bar|default:_ }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::LeadingUnderscore {
                at: (19, 1).into()
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_incomplete() {
        let template = "{{ foo.bar|default:'foo }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (19, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete() {
        let template = "{{ foo.bar|default:_('foo' }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteTranslatedString { at: (19, 7).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string() {
        let template = "{{ foo.bar|default:_('foo }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (21, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string_double_quotes() {
        let template = "{{ foo.bar|default:_(\"foo }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (21, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string() {
        let template = "{{ foo.bar|default:_( }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::MissingTranslatedString { at: (19, 2).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string_trailing_chars() {
        let template = "{{ foo.bar|default:_(foo) }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::MissingTranslatedString { at: (19, 6).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_filter_remainder_before_argument() {
        let template = "{{ foo.bar|default'spam':title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (18, 6).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_filter_remainder_before_filter() {
        let template = "{{ foo.bar|title'spam'|title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (16, 6).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder() {
        let template = "{{ foo.bar|default:\"spam\"title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (25, 5).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder_before_filter() {
        let template = "{{ foo.bar|default:\"spam\"title|title }}";
        let variable = trim_variable(template);
        let (_token, _at, lexer) = lex_variable_or_filter(variable, START_TAG_LEN)
            .unwrap()
            .unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (25, 5).into() }.into()
            )]
        );
    }
}
