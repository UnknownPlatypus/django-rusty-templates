use thiserror::Error;
use unicode_xid::UnicodeXID;

const START_TAG_LEN: usize = 2;
const END_TAG_LEN: usize = 2;

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
        let start = self.byte;
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
        self.byte += text.len();
        let at = (start, self.byte);
        Token::Text { text, at }
    }

    fn lex_text_to_end(&mut self) -> Token<'t> {
        let start = self.byte;
        let text = self.rest;
        self.rest = "";
        self.byte += text.len();
        let at = (start, self.byte);
        Token::Text { text, at }
    }

    fn lex_tag(&mut self, end_tag: EndTag) -> Token<'t> {
        let end_str = match end_tag {
            EndTag::Variable => "}}",
            EndTag::Tag => "%}",
            EndTag::Comment => "#}",
        };
        let start = self.byte;
        let tag = match self.rest.find(end_str) {
            None => {
                self.byte += self.rest.len();
                let text = self.rest;
                self.rest = "";
                let at = (start, self.byte);
                return Token::Text { text, at };
            }
            Some(n) => {
                let tag = &self.rest[START_TAG_LEN..n];
                self.rest = &self.rest[n + END_TAG_LEN..];
                tag
            }
        };
        self.byte += tag.len() + 4;
        let at = (start, self.byte);
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
        let start = self.byte;
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
                                self.byte += tag.len() + 4;
                                self.rest = &self.rest[tag.len() + 4..];
                                let at = (start, self.byte);
                                return Token::Tag { tag, at };
                            } else {
                                self.rest = &self.rest[index..];
                                self.byte += index;
                                let at = (start, self.byte);
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
pub enum VariableTokenType {
    Text,
    Variable,
    Filter,
    Numeric,
    TranslatedText,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VariableToken<'t> {
    token_type: VariableTokenType,
    content: &'t str,
    at: (usize, usize),
}

enum Mode {
    Variable,
    Filter,
    Argument,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum VariableLexerError {
    #[error("Variables and attributes may not begin with underscores")]
    LeadingUnderscore { at: (usize, usize) },
    #[error("Expected a complete string literal")]
    IncompleteString { at: (usize, usize) },
    #[error("Expected a complete translation string")]
    IncompleteTranslatedString { at: (usize, usize) },
    #[error("Expected a string literal within translation")]
    MissingTranslatedString { at: (usize, usize) },
    #[error("Could not parse the remainder")]
    InvalidRemainder { at: (usize, usize) },
    #[error("Expected a valid filter name")]
    InvalidFilterName { at: (usize, usize) },
    #[error("Expected a valid variable name")]
    InvalidVariableName { at: (usize, usize) },
}

pub struct VariableLexer<'t> {
    rest: &'t str,
    byte: usize,
    mode: Mode,
}

impl<'t> VariableLexer<'t> {
    pub fn new(variable: &'t str, start: usize) -> Self {
        let rest = variable.trim_start();
        Self {
            rest: rest.trim_end(),
            byte: start + START_TAG_LEN + variable.len() - rest.len(),
            mode: Mode::Variable,
        }
    }

    fn lex_text(
        &mut self,
        chars: &mut std::str::Chars,
        end: char,
    ) -> Result<VariableToken<'t>, VariableLexerError> {
        let mut count = 1;
        loop {
            let next = match chars.next() {
                None => {
                    let start = self.byte;
                    let end = self.byte + count;
                    let at = (start, end);
                    self.rest = "";
                    return Err(VariableLexerError::IncompleteString { at });
                }
                Some(c) => c,
            };
            count += 1;
            if next == '\\' {
                count += 1;
                chars.next();
            } else if next == end {
                let start = self.byte;
                let content = &self.rest[1..count - 1];
                self.byte += content.len() + 2;
                self.rest = &self.rest[count..];
                let at = (start, self.byte);
                return Ok(VariableToken {
                    token_type: VariableTokenType::Text,
                    content,
                    at,
                });
            }
        }
    }

    fn lex_translated(
        &mut self,
        chars: &mut std::str::Chars,
    ) -> Result<VariableToken<'t>, VariableLexerError> {
        let start = self.byte;
        self.byte += 2;
        self.rest = &self.rest[2..];
        let token = match chars.next() {
            None => {
                let at = (start, self.byte);
                self.rest = "";
                return Err(VariableLexerError::MissingTranslatedString { at });
            }
            Some('\'') => self.lex_text(chars, '\'')?,
            Some('"') => self.lex_text(chars, '"')?,
            _ => {
                let at = (start, self.byte + self.rest.len());
                self.rest = "";
                return Err(VariableLexerError::MissingTranslatedString { at });
            }
        };
        match chars.next() {
            Some(')') => {
                self.byte += 1;
                self.rest = &self.rest[1..];
                Ok(VariableToken {
                    token_type: VariableTokenType::TranslatedText,
                    content: token.content,
                    at: (start, self.byte),
                })
            }
            _ => {
                let at = (start, self.byte);
                self.rest = "";
                Err(VariableLexerError::IncompleteTranslatedString { at })
            }
        }
    }

    fn lex_numeric(&mut self) -> VariableToken<'t> {
        let end = self
            .rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == 'e'))
            .unwrap_or(self.rest.len());
        let start = self.byte;
        self.byte += end;
        let content = &self.rest[..end];
        self.rest = &self.rest[end..];
        let at = (start, self.byte);
        VariableToken {
            token_type: VariableTokenType::Numeric,
            content,
            at,
        }
    }

    fn lex_variable(&mut self) -> Result<VariableToken<'t>, VariableLexerError> {
        let end = self
            .rest
            .find(|c: char| !c.is_xid_continue() && c != '.')
            .unwrap_or(self.rest.len());
        let content = &self.rest[..end];

        let mut offset = 0;
        for var in content.split('.') {
            match var.chars().next() {
                Some(c) if c.is_xid_start() && c != '_' => {
                    offset += var.len() + 1;
                    continue;
                }
                _ => {
                    let at = (self.byte + offset, self.byte + offset + var.len());
                    self.rest = "";
                    return Err(VariableLexerError::InvalidVariableName { at });
                }
            }
        }
        let at = (self.byte, self.byte + end);
        self.byte += end;
        self.rest = &self.rest[end..];
        Ok(VariableToken {
            token_type: VariableTokenType::Variable,
            content,
            at,
        })
    }

    fn lex_filter(&mut self) -> Result<VariableToken<'t>, VariableLexerError> {
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
                let at = (self.byte, self.byte + end);
                self.byte += end;
                self.rest = &self.rest[end..];
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: filter,
                    at,
                })
            }
            _ => {
                let next = [self.rest.find("|"), self.rest.find(":")]
                    .iter()
                    .filter_map(|n| *n)
                    .min()
                    .unwrap_or(self.rest.len());
                let at = (self.byte, self.byte + next);
                self.rest = "";
                Err(VariableLexerError::InvalidFilterName { at })
            }
        }
    }

    fn lex_argument(&mut self) -> Result<VariableToken<'t>, VariableLexerError> {
        let mut chars = self.rest.chars();
        match chars.next().unwrap() {
            '_' => {
                if let Some('(') = chars.next() {
                    self.lex_translated(&mut chars)
                } else {
                    let start = self.byte;
                    let end = self
                        .rest
                        .find(char::is_whitespace)
                        .unwrap_or(self.rest.len());
                    let at = (start, start + end);
                    self.byte += self.rest.len();
                    self.rest = "";
                    Err(VariableLexerError::LeadingUnderscore { at })
                }
            }
            '\'' => self.lex_text(&mut chars, '\''),
            '"' => self.lex_text(&mut chars, '"'),
            '0'..='9' => Ok(self.lex_numeric()),
            _ => self.lex_variable(),
        }
    }

    fn lex_remainder(
        &mut self,
        token: VariableToken<'t>,
        remainder: &'t str,
        start_next: usize,
    ) -> Result<VariableToken<'t>, VariableLexerError> {
        match remainder.find(|c: char| !c.is_whitespace()) {
            None => {
                self.rest = &self.rest[start_next..];
                self.byte += start_next;
                Ok(token)
            }
            Some(n) => {
                let at = (self.byte + n, self.byte + remainder.trim_end().len());
                self.rest = "";
                Err(VariableLexerError::InvalidRemainder { at })
            }
        }
    }

    fn remainder_to_filter(&mut self) -> (&'t str, usize) {
        self.mode = Mode::Filter;
        match self.rest.find("|") {
            None => (self.rest, self.rest.len()),
            Some(f) => (&self.rest[..f], f + 1),
        }
    }

    fn remainder_to_filter_or_argument(&mut self) -> (&'t str, usize) {
        match (self.rest.find("|"), self.rest.find(":")) {
            (None, None) => (self.rest, self.rest.len()),
            (None, Some(a)) => {
                self.mode = Mode::Argument;
                (&self.rest[..a], a + 1)
            }
            (Some(f), Some(a)) if a < f => {
                self.mode = Mode::Argument;
                (&self.rest[..a], a + 1)
            }
            (Some(f), _) => (&self.rest[..f], f + 1),
        }
    }
}

impl<'t> Iterator for VariableLexer<'t> {
    type Item = Result<VariableToken<'t>, VariableLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let token = match self.mode {
            Mode::Variable => self.lex_variable(),
            Mode::Filter => self.lex_filter(),
            Mode::Argument => self.lex_argument(),
        };
        let token = match token {
            Err(e) => return Some(Err(e)),
            Ok(token) => token,
        };
        let (remainder, start_next) = match self.mode {
            Mode::Filter => self.remainder_to_filter_or_argument(),
            Mode::Variable | Mode::Argument => self.remainder_to_filter(),
        };
        Some(self.lex_remainder(token, remainder, start_next))
    }
}

#[cfg(test)]
mod lexer_tests {
    use super::*;

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
                    at: (5, 18),
                },
                Token::Variable {
                    variable: " varvalue ",
                    at: (18, 32),
                },
                Token::Tag {
                    tag: " endif ",
                    at: (32, 43),
                },
                Token::Comment {
                    comment: "comment {{not a var}} {%not a block%} ",
                    at: (43, 85),
                },
                Token::Text {
                    text: "end text",
                    at: (85, 93),
                },
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
                    at: (14, 25),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (25, 42),
                },
            ]
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
                    at: (14, 25),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (25, 42),
                },
            ]
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
                    at: (14, 41),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (41, 58),
                },
            ]
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
                    at: (14, 28),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (28, 45),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (45, 62),
                },
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
                    at: (14, 31),
                },
                Token::Tag {
                    tag: " verbatim ",
                    at: (31, 45),
                },
                Token::Tag {
                    tag: " endverbatim ",
                    at: (45, 62),
                },
            ]
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
                    at: (22, 54),
                },
                Token::Tag {
                    tag: " endverbatim special ",
                    at: (54, 79),
                },
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
                    at: (14, 23),
                },
            ]
        );
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
                    at: (14, 32),
                },
            ]
        );
    }
}

#[cfg(test)]
mod variable_lexer_tests {
    use super::*;

    #[test]
    fn test_lex_empty() {
        let variable = "  ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable() {
        let variable = " foo.bar ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(VariableToken {
                token_type: VariableTokenType::Variable,
                content: "foo.bar",
                at: (3, 10)
            })]
        );
    }

    #[test]
    fn test_lex_variable_start_underscore() {
        let variable = " _foo.bar ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidVariableName { at: (3, 7) })]
        );
    }

    #[test]
    fn test_lex_attribute_start_underscore() {
        let variable = " foo._bar ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidVariableName { at: (7, 11) })]
        );
    }

    #[test]
    fn test_lex_filter() {
        let variable = " foo.bar|title ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "title",
                    at: (11, 16),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_filter_remainder() {
        let variable = " foo.bar|title'foo' ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Err(VariableLexerError::InvalidRemainder { at: (16, 21) }),
            ]
        );
    }

    #[test]
    fn test_lex_filter_invalid_start() {
        let variable = " foo.bar|'foo' ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Err(VariableLexerError::InvalidFilterName { at: (11, 16) }),
            ]
        );
    }

    #[test]
    fn test_lex_text_argument_single_quote() {
        let variable = " foo.bar|default:'foo' ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Text,
                    content: "foo",
                    at: (19, 24),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_text_argument_double_quote() {
        let variable = " foo.bar|default:\"foo\" ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Text,
                    content: "foo",
                    at: (19, 24),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_text_argument_escaped() {
        let variable = " foo.bar|default:'foo\\\'' ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Text,
                    content: "foo\\\'",
                    at: (19, 26),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_translated_text_argument() {
        let variable = " foo.bar|default:_('foo') ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::TranslatedText,
                    content: "foo",
                    at: (19, 27),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_numeric_argument() {
        let variable = " foo.bar|default:500 ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Numeric,
                    content: "500",
                    at: (19, 22),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_variable_argument() {
        let variable = " foo.bar|default:spam ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "spam",
                    at: (19, 23),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_variable_argument_then_filter() {
        let variable = " foo.bar|default:spam|title ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "spam",
                    at: (19, 23),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "title",
                    at: (24, 29),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_string_argument_then_filter() {
        let variable = " foo.bar|default:\"spam\"|title ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Text,
                    content: "spam",
                    at: (19, 25),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "title",
                    at: (26, 31),
                }),
            ]
        );
    }

    #[test]
    fn test_lex_argument_with_leading_underscore() {
        let variable = " foo.bar|default:_spam ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::LeadingUnderscore { at: (19, 24) }),
            ]
        );
    }

    #[test]
    fn test_lex_argument_with_only_underscore() {
        let variable = " foo.bar|default:_ ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::LeadingUnderscore { at: (19, 20) }),
            ]
        );
    }

    #[test]
    fn test_lex_text_argument_incomplete() {
        let variable = " foo.bar|default:'foo ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::IncompleteString { at: (19, 23) }),
            ]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete() {
        let variable = " foo.bar|default:_('foo' ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::IncompleteTranslatedString { at: (19, 26) }),
            ]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string() {
        let variable = " foo.bar|default:_('foo ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::IncompleteString { at: (21, 25) }),
            ]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string() {
        let variable = " foo.bar|default:_( ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::MissingTranslatedString { at: (19, 21) }),
            ]
        );
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string_trailing_chars() {
        let variable = " foo.bar|default:_(foo) ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::MissingTranslatedString { at: (19, 25) }),
            ]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder() {
        let variable = " foo.bar|default:\"spam\"title ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::InvalidRemainder { at: (25, 30) }),
            ]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder_before_filter() {
        let variable = " foo.bar|default:\"spam\"title|title ";
        let lexer = VariableLexer::new(variable, 0);
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(VariableToken {
                    token_type: VariableTokenType::Variable,
                    content: "foo.bar",
                    at: (3, 10),
                }),
                Ok(VariableToken {
                    token_type: VariableTokenType::Filter,
                    content: "default",
                    at: (11, 18),
                }),
                Err(VariableLexerError::InvalidRemainder { at: (25, 30) }),
            ]
        );
    }
}
