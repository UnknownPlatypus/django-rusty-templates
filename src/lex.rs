use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

pub const START_TAG_LEN: usize = 2;
pub const END_TAG_LEN: usize = 2;
pub const START_TRANSLATE_LEN: usize = 2;
pub const END_TRANSLATE_LEN: usize = 1;

enum EndTag {
    Variable,
    Tag,
    Comment,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'t> {
    Text {
        text: &'t str,
        at: (usize, usize),
    },
    Variable {
        variable: &'t str,
        at: (usize, usize),
    },
    Tag {
        tag: &'t str,
        at: (usize, usize),
    },
    Comment {
        comment: &'t str,
        at: (usize, usize),
    },
}

impl<'a, 't> Token<'a> {
    fn content(&self, template: &'t str) -> &'t str {
        let (start, end) = match self {
            Token::Text {
                at: (start, len), ..
            } => (*start, start + len),
            Token::Variable {
                at: (start, len), ..
            } => (start + START_TAG_LEN, start + len - END_TAG_LEN),
            Token::Tag {
                at: (start, len), ..
            } => (start + START_TAG_LEN, start + len - END_TAG_LEN),
            Token::Comment {
                at: (start, len), ..
            } => (start + START_TAG_LEN, start + len - END_TAG_LEN),
        };
        &template[start..end]
    }
}

pub struct Lexer<'t> {
    rest: &'t str,
    byte: usize,
    verbatim: Option<&'t str>,
}

impl<'t> Lexer<'t> {
    pub fn new(template: &'t str) -> Self {
        Self {
            rest: template,
            byte: 0,
            verbatim: None,
        }
    }

    fn lex_text(&mut self) -> Token<'t> {
        let next_tag = self.rest.find("{%");
        let next_variable = self.rest.find("{{");
        let next_comment = self.rest.find("{#");
        let next = [next_tag, next_variable, next_comment]
            .iter()
            .filter_map(|n| *n)
            .min();
        let text = match next {
            None => {
                let text = self.rest;
                self.rest = "";
                text
            }
            Some(n) => {
                let text = &self.rest[..n];
                self.rest = &self.rest[n..];
                text
            }
        };
        let at = (self.byte, text.len());
        self.byte += text.len();
        Token::Text { text, at }
    }

    fn lex_text_to_end(&mut self) -> Token<'t> {
        let text = self.rest;
        let at = (self.byte, text.len());
        self.byte += text.len();
        self.rest = "";
        Token::Text { text, at }
    }

    fn lex_tag(&mut self, end_tag: EndTag) -> Token<'t> {
        let end_str = match end_tag {
            EndTag::Variable => "}}",
            EndTag::Tag => "%}",
            EndTag::Comment => "#}",
        };
        let tag = match self.rest.find(end_str) {
            None => {
                let text = self.rest;
                let at = (self.byte, text.len());
                self.byte += self.rest.len();
                self.rest = "";
                return Token::Text { text, at };
            }
            Some(n) => {
                let tag = &self.rest[START_TAG_LEN..n];
                self.rest = &self.rest[n + END_TAG_LEN..];
                tag
            }
        };
        let at = (self.byte, tag.len() + 4);
        self.byte += tag.len() + 4;
        match end_tag {
            EndTag::Variable => Token::Variable { variable: tag, at },
            EndTag::Tag => Token::Tag { tag, at },
            EndTag::Comment => Token::Comment { comment: tag, at },
        }
    }

    fn lex_verbatim(&mut self, verbatim: &'t str) -> Token<'t> {
        let verbatim = verbatim.trim();
        self.verbatim = None;

        let mut rest = self.rest;
        let mut index = 0;
        loop {
            let next_tag = rest.find("{%");
            match next_tag {
                None => return self.lex_text_to_end(),
                Some(start_tag) => {
                    rest = &rest[start_tag..];
                    let close_tag = rest.find("%}");
                    match close_tag {
                        None => return self.lex_text_to_end(),
                        Some(end_tag) => {
                            let inner = &rest[2..end_tag].trim();
                            // Check we have the right endverbatim tag
                            if inner.len() < 3 || &inner[3..] != verbatim {
                                rest = &rest[end_tag + 2..];
                                index += start_tag + end_tag + 2;
                                continue;
                            }

                            index += start_tag;
                            let text = &self.rest[..index];
                            if text.is_empty() {
                                // Return the endverbatim tag since we have no text
                                let tag = &self.rest[2..end_tag];
                                let tag_len = tag.len() + 4;
                                let at = (self.byte, tag_len);
                                self.byte += tag_len;
                                self.rest = &self.rest[tag_len..];
                                return Token::Tag { tag, at };
                            } else {
                                self.rest = &self.rest[index..];
                                let at = (self.byte, index);
                                self.byte += index;
                                return Token::Text { text, at };
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'t> Iterator for Lexer<'t> {
    type Item = Token<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        Some(match self.verbatim {
            None => match self.rest.get(..START_TAG_LEN) {
                Some("{{") => self.lex_tag(EndTag::Variable),
                Some("{%") => {
                    let tag = self.lex_tag(EndTag::Tag);
                    if let Token::Tag { tag: verbatim, .. } = tag {
                        let verbatim = verbatim.trim();
                        if verbatim == "verbatim" || verbatim.starts_with("verbatim ") {
                            self.verbatim = Some(verbatim)
                        }
                    }
                    tag
                }
                Some("{#") => self.lex_tag(EndTag::Comment),
                _ => self.lex_text(),
            },
            Some(verbatim) => self.lex_verbatim(verbatim),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArgumentType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Argument<'t> {
    pub argument_type: ArgumentType,
    pub content: &'t str,
    pub at: (usize, usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct FilterToken<'t> {
    pub content: &'t str,
    pub at: (usize, usize),
    pub argument: Option<Argument<'t>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VariableToken<'t> {
    pub content: &'t str,
    pub at: (usize, usize),
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum VariableLexerError {
    #[error("Variables and attributes may not begin with underscores")]
    LeadingUnderscore {
        #[label("here")]
        at: SourceSpan,
    },
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
    #[error("Expected a string literal within translation")]
    MissingTranslatedString {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Could not parse the remainder")]
    InvalidRemainder {
        #[label("here")]
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

fn trim_variable(variable: &str) -> &str {
    match variable.find(|c: char| !c.is_xid_continue() && c != '.') {
        Some(end) => &variable[..end],
        None => variable,
    }
}

fn check_variable_attrs(variable: &str, start: usize) -> Result<(), VariableLexerError> {
    let mut offset = 0;
    for var in variable.split('.') {
        match var.chars().next() {
            Some(c) if c.is_xid_start() && c != '_' => {
                offset += var.len() + 1;
                continue;
            }
            _ => {
                let at = (start + offset, var.len());
                return Err(VariableLexerError::InvalidVariableName { at: at.into() });
            }
        }
    }
    Ok(())
}

pub fn lex_variable(
    variable: &str,
    start: usize,
) -> Result<Option<(VariableToken, FilterLexer)>, VariableLexerError> {
    let rest = variable.trim_start();
    let start = start + variable.len() - rest.len();

    let content = trim_variable(rest);
    if content.is_empty() {
        return Ok(None);
    }
    check_variable_attrs(content, start)?;

    let end = content.len();
    let at = (start, end);
    Ok(Some((
        VariableToken { content, at },
        FilterLexer::new(&rest[end..], start + end),
    )))
}

#[derive(Debug)]
pub struct FilterLexer<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> FilterLexer<'t> {
    fn new(variable: &'t str, start: usize) -> Self {
        let offset = match variable.find('|') {
            Some(n) => n + 1,
            None => {
                return Self {
                    rest: "",
                    byte: start + variable.len(),
                }
            }
        };
        let variable = &variable[offset..];
        let rest = variable.trim_start();
        Self {
            rest: rest.trim_end(),
            byte: start + offset + variable.len() - rest.len(),
        }
    }

    fn lex_text(
        &mut self,
        chars: &mut std::str::Chars,
        end: char,
    ) -> Result<Argument<'t>, VariableLexerError> {
        let mut count = 1;
        loop {
            let next = match chars.next() {
                None => {
                    let at = (self.byte, count);
                    self.rest = "";
                    return Err(VariableLexerError::IncompleteString { at: at.into() });
                }
                Some(c) => c,
            };
            count += 1;
            if next == '\\' {
                count += 1;
                chars.next();
            } else if next == end {
                let at = (self.byte, count);
                let content = &self.rest[1..count - 1];
                self.rest = &self.rest[count..];
                self.byte += count;
                return Ok(Argument {
                    argument_type: ArgumentType::Text,
                    content,
                    at,
                });
            }
        }
    }

    fn lex_translated(
        &mut self,
        chars: &mut std::str::Chars,
    ) -> Result<Argument<'t>, VariableLexerError> {
        let start = self.byte;
        self.byte += START_TRANSLATE_LEN;
        self.rest = &self.rest[START_TRANSLATE_LEN..];
        let token = match chars.next() {
            None => {
                let at = (start, START_TRANSLATE_LEN);
                self.rest = "";
                return Err(VariableLexerError::MissingTranslatedString { at: at.into() });
            }
            Some('\'') => self.lex_text(chars, '\'')?,
            Some('"') => self.lex_text(chars, '"')?,
            _ => {
                let at = (start, self.rest.len() + START_TRANSLATE_LEN);
                self.rest = "";
                return Err(VariableLexerError::MissingTranslatedString { at: at.into() });
            }
        };
        match chars.next() {
            Some(')') => {
                self.byte += END_TRANSLATE_LEN;
                self.rest = &self.rest[END_TRANSLATE_LEN..];
                Ok(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    content: token.content,
                    at: (start, self.byte - start),
                })
            }
            _ => {
                let at = (start, self.byte - start);
                self.rest = "";
                Err(VariableLexerError::IncompleteTranslatedString { at: at.into() })
            }
        }
    }

    fn lex_numeric(&mut self) -> Argument<'t> {
        let end = self
            .rest
            .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.' || c == 'e'))
            .unwrap_or(self.rest.len());
        let content = &self.rest[..end];
        // Match django bug
        let (content, end) = match content[1..].find('-') {
            Some(n) => (&content[..n + 1], n + 1),
            None => (content, end),
        };
        // End match django bug
        self.rest = &self.rest[end..];
        let at = (self.byte, end);
        self.byte += end;
        Argument {
            argument_type: ArgumentType::Numeric,
            content,
            at,
        }
    }
    fn lex_variable_argument(&mut self) -> Result<Argument<'t>, VariableLexerError> {
        let content = trim_variable(self.rest);
        match check_variable_attrs(content, self.byte) {
            Ok(()) => {}
            Err(e) => {
                self.rest = "";
                return Err(e);
            }
        };
        let end = content.len();
        let at = (self.byte, end);
        self.byte += end;
        self.rest = &self.rest[end..];
        Ok(Argument {
            argument_type: ArgumentType::Variable,
            content,
            at,
        })
    }

    fn lex_filter(&mut self) -> Result<FilterToken<'t>, VariableLexerError> {
        let filter = self.rest.trim_start();
        let start = self.rest.len() - filter.len();
        self.byte += start;
        self.rest = &self.rest[start..];

        let end = filter
            .find(|c: char| !c.is_xid_continue())
            .unwrap_or(filter.len());
        let filter = &filter[..end];

        match filter.chars().next() {
            Some(c) if c.is_xid_start() => {
                let at = (self.byte, end);
                self.byte += end;
                self.rest = &self.rest[end..];
                let argument = self.lex_argument()?;
                Ok(FilterToken {
                    content: filter,
                    at,
                    argument,
                })
            }
            _ => {
                let next = self.rest.find("|").unwrap_or(self.rest.len());
                let at = (self.byte, next);
                self.rest = "";
                Err(VariableLexerError::InvalidFilterName { at: at.into() })
            }
        }
    }

    fn lex_argument(&mut self) -> Result<Option<Argument<'t>>, VariableLexerError> {
        let next = match (self.rest.find("|"), self.rest.find(":")) {
            (_, None) => return Ok(None),
            (Some(f), Some(a)) if f < a => return Ok(None),
            (_, Some(a)) => a + 1,
        };
        self.rest = &self.rest[next..];
        self.byte += next;

        let mut chars = self.rest.chars();
        Ok(Some(match chars.next().unwrap() {
            '_' => {
                if let Some('(') = chars.next() {
                    self.lex_translated(&mut chars)?
                } else {
                    let end = self
                        .rest
                        .find(char::is_whitespace)
                        .unwrap_or(self.rest.len());
                    let at = (self.byte, end);
                    self.byte += self.rest.len();
                    self.rest = "";
                    return Err(VariableLexerError::LeadingUnderscore { at: at.into() });
                }
            }
            '\'' => self.lex_text(&mut chars, '\'')?,
            '"' => self.lex_text(&mut chars, '"')?,
            '0'..='9' | '-' => self.lex_numeric(),
            _ => self.lex_variable_argument()?,
        }))
    }

    fn lex_remainder(
        &mut self,
        token: FilterToken<'t>,
        remainder: &'t str,
        start_next: usize,
    ) -> Result<FilterToken<'t>, VariableLexerError> {
        match remainder.find(|c: char| !c.is_whitespace()) {
            None => {
                self.rest = &self.rest[start_next..];
                self.byte += start_next;
                Ok(token)
            }
            Some(n) => {
                let at = (self.byte + n, remainder.trim().len());
                self.rest = "";
                Err(VariableLexerError::InvalidRemainder { at: at.into() })
            }
        }
    }

    fn remainder_to_filter_or_argument(&mut self) -> (&'t str, usize) {
        match (self.rest.find("|"), self.rest.find(":")) {
            (None, None) => (self.rest, self.rest.len()),
            (None, Some(a)) => (&self.rest[..a], a + 1),
            (Some(f), Some(a)) if a < f => (&self.rest[..a], a + 1),
            (Some(f), _) => (&self.rest[..f], f + 1),
        }
    }
}

impl<'t> Iterator for FilterLexer<'t> {
    type Item = Result<FilterToken<'t>, VariableLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let token = match self.lex_filter() {
            Err(e) => return Some(Err(e)),
            Ok(token) => token,
        };
        let (remainder, start_next) = self.remainder_to_filter_or_argument();
        Some(self.lex_remainder(token, remainder, start_next))
    }
}

#[cfg(test)]
mod lexer_tests {
    use super::*;

    fn contents<'t>(template: &'t str, tokens: Vec<Token>) -> Vec<&'t str> {
        tokens.iter().map(|t| t.content(template)).collect()
    }

    #[test]
    fn test_lex_empty() {
        let template = "";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_text() {
        let template = "Just some text";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Text {
                text: template,
                at: (0, 14),
            }]
        );
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_text_whitespace() {
        let template = "    ";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Text {
                text: template,
                at: (0, 4),
            }]
        );
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_comment() {
        let template = "{# comment #}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Comment {
                comment: " comment ",
                at: (0, 13),
            }]
        );
        assert_eq!(contents(template, tokens), vec![" comment "]);
    }

    #[test]
    fn test_lex_variable() {
        let template = "{{ foo.bar|title }}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Variable {
                variable: " foo.bar|title ",
                at: (0, 19),
            }]
        );
        assert_eq!(contents(template, tokens), vec![" foo.bar|title "]);
    }

    #[test]
    fn test_lex_tag() {
        let template = "{% for foo in bar %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Tag {
                tag: " for foo in bar ",
                at: (0, 20),
            }]
        );
        assert_eq!(contents(template, tokens), vec![" for foo in bar "]);
    }

    #[test]
    fn test_lex_incomplete_comment() {
        let template = "{# comment #";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Text {
                text: template,
                at: (0, 12),
            }]
        );
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_incomplete_variable() {
        let template = "{{ foo.bar|title }";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Text {
                text: template,
                at: (0, 18),
            }]
        );
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_incomplete_tag() {
        let template = "{% for foo in bar %";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Token::Text {
                text: template,
                at: (0, 19),
            }]
        );
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_django_example() {
        let template = "text\n{% if test %}{{ varvalue }}{% endif %}{#comment {{not a var}} {%not a block%} #}end text";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Text {
                    text: "text\n",
                    at: (0, 5),
                },
                Token::Tag {
                    tag: " if test ",
                    at: (5, 13),
                },
                Token::Variable {
                    variable: " varvalue ",
                    at: (18, 14),
                },
                Token::Tag {
                    tag: " endif ",
                    at: (32, 11),
                },
                Token::Comment {
                    comment: "comment {{not a var}} {%not a block%} ",
                    at: (43, 42),
                },
                Token::Text {
                    text: "end text",
                    at: (85, 8),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![
                "text\n",
                " if test ",
                " varvalue ",
                " endif ",
                "comment {{not a var}} {%not a block%} ",
                "end text",
            ]
        );
    }

    #[test]
    fn test_verbatim_with_variable() {
        let template = "{% verbatim %}{{bare   }}{% endverbatim %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "{{bare   }}",
                    at: (14, 11),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (25, 17),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "{{bare   }}", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_with_tag() {
        let template = "{% verbatim %}{% endif %}{% endverbatim %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "{% endif %}",
                    at: (14, 11),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (25, 17),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "{% endif %}", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_with_verbatim_tag() {
        let template = "{% verbatim %}It's the {% verbatim %} tag{% endverbatim %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "It's the {% verbatim %} tag",
                    at: (14, 27),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (41, 17),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "It's the {% verbatim %} tag", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_nested() {
        let template = "{% verbatim %}{% verbatim %}{% endverbatim %}{% endverbatim %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "{% verbatim %}",
                    at: (14, 14),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (28, 17),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (45, 17),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![
                " verbatim ",
                "{% verbatim %}",
                " endverbatim ",
                " endverbatim ",
            ]
        );
    }

    #[test]
    fn test_verbatim_adjacent() {
        let template = "{% verbatim %}{% endverbatim %}{% verbatim %}{% endverbatim %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (14, 17),
                },
                Token::Tag {
                    tag: " verbatim ",
                    at: (31, 14),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (45, 17),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", " endverbatim ", " verbatim ", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_special() {
        let template =
            "{% verbatim special %}Don't {% endverbatim %} just yet{% endverbatim special %}";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim special ",
                    at: (0, 22),
                },
                Token::Text {
                    text: "Don't {% endverbatim %} just yet",
                    at: (22, 32),
                },
                Token::Tag {
                    tag: " endverbatim special ",
                    at: (54, 25),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![
                " verbatim special ",
                "Don't {% endverbatim %} just yet",
                " endverbatim special ",
            ]
        );
    }

    #[test]
    fn test_verbatim_open_tag() {
        let template = "{% verbatim %}Don't {% ";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "Don't {% ",
                    at: (14, 9),
                },
            ]
        );
        assert_eq!(contents(template, tokens), vec![" verbatim ", "Don't {% "]);
    }

    #[test]
    fn test_verbatim_no_tag() {
        let template = "{% verbatim %}Don't end verbatim";
        let lexer = Lexer::new(template);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::Tag {
                    tag: " verbatim ",
                    at: (0, 14),
                },
                Token::Text {
                    text: "Don't end verbatim",
                    at: (14, 18),
                },
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "Don't end verbatim"]
        );
    }
}

#[cfg(test)]
mod variable_lexer_tests {
    use super::*;

    #[test]
    fn test_lex_empty() {
        let variable = "  ";
        assert!(lex_variable(variable, START_TAG_LEN).unwrap().is_none());
    }

    #[test]
    fn test_lex_variable() {
        let variable = " foo.bar ";
        let (token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        assert_eq!(
            token,
            VariableToken {
                content: "foo.bar",
                at: (3, 7)
            }
        );
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_start_underscore() {
        let variable = " _foo.bar ";
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            VariableLexerError::InvalidVariableName { at: (3, 4).into() }
        );
    }

    #[test]
    fn test_lex_attribute_start_underscore() {
        let variable = " foo._bar ";
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            VariableLexerError::InvalidVariableName { at: (7, 4).into() }
        );
    }

    #[test]
    fn test_lex_filter() {
        let variable = " foo.bar|title ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                content: "title",
                at: (11, 5),
                argument: None,
            })]
        );
    }

    #[test]
    fn test_lex_filter_chain() {
        let variable = " foo.bar|title|length ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: None,
                    content: "title",
                    at: (11, 5),
                }),
                Ok(FilterToken {
                    argument: None,
                    content: "length",
                    at: (17, 6),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_filter_remainder() {
        let variable = " foo.bar|title'foo' ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidRemainder {
                at: (16, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_filter_invalid_start() {
        let variable = " foo.bar|'foo' ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
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
        let variable = " foo.bar|default:'foo' ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    content: "foo",
                    at: (19, 5),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_double_quote() {
        let variable = " foo.bar|default:\"foo\" ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    content: "foo",
                    at: (19, 5),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_escaped() {
        let variable = " foo.bar|default:'foo\\\'' ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    content: "foo\\\'",
                    at: (19, 7),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument() {
        let variable = " foo.bar|default:_('foo') ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    content: "foo",
                    at: (19, 8),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_double_quoted() {
        let variable = " foo.bar|default:_(\"foo\") ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    content: "foo",
                    at: (19, 8),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_numeric_argument() {
        let variable = " foo.bar|default:500 ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    content: "500",
                    at: (19, 3),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_numeric_argument_negative() {
        let variable = " foo.bar|default:-0.5 ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    content: "-0.5",
                    at: (19, 4),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_numeric_argument_scientific() {
        let variable = " foo.bar|default:5.2e3 ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    content: "5.2e3",
                    at: (19, 5),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_numeric_argument_scientific_negative_exponent() {
        // Django mishandles this case, so we do too:
        // https://code.djangoproject.com/ticket/35816
        let variable = " foo.bar|default:5.2e-3 ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Err(VariableLexerError::InvalidRemainder { at: (23, 2).into() }),
                /* When fixed we can do:
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Numeric,
                            content: "5.2e-3",
                            at: (19, 6),
                    }),
                    content: "default",
                    at: (11, 7),
                })
                */
            ]
        );
    }

    #[test]
    fn test_lex_variable_argument() {
        let variable = " foo.bar|default:spam ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Variable,
                    content: "spam",
                    at: (19, 4),
                }),
                content: "default",
                at: (11, 7),
            })]
        );
    }

    #[test]
    fn test_lex_variable_argument_then_filter() {
        let variable = " foo.bar|default:spam|title ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Variable,
                        content: "spam",
                        at: (19, 4),
                    }),
                    content: "default",
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    content: "title",
                    at: (24, 5),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_string_argument_then_filter() {
        let variable = " foo.bar|default:\"spam\"|title ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Text,
                        content: "spam",
                        at: (19, 6),
                    }),
                    content: "default",
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    content: "title",
                    at: (26, 5),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_argument_with_leading_underscore() {
        let variable = " foo.bar|default:_spam ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::LeadingUnderscore {
                at: (19, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_argument_with_only_underscore() {
        let variable = " foo.bar|default:_ ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
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
        let variable = " foo.bar|default:'foo ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::IncompleteString {
                at: (19, 4).into()
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete() {
        let variable = " foo.bar|default:_('foo' ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::IncompleteTranslatedString {
                at: (19, 7).into()
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string() {
        let variable = " foo.bar|default:_('foo ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::IncompleteString {
                at: (21, 4).into()
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string_double_quotes() {
        let variable = " foo.bar|default:_(\"foo ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::IncompleteString {
                at: (21, 4).into()
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string() {
        let variable = " foo.bar|default:_( ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::MissingTranslatedString {
                at: (19, 2).into()
            })]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string_trailing_chars() {
        let variable = " foo.bar|default:_(foo) ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::MissingTranslatedString {
                at: (19, 6).into()
            })]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder() {
        let variable = " foo.bar|default:\"spam\"title ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidRemainder {
                at: (25, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder_before_filter() {
        let variable = " foo.bar|default:\"spam\"title|title ";
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidRemainder {
                at: (25, 5).into()
            })]
        );
    }
}
