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

#[cfg(test)]
mod tests {
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
}
