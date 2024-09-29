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
        Token::Text {
            text,
            at: (start, self.byte),
        }
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
        todo!()
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
                Some("{%") => self.lex_tag(EndTag::Tag),
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
}
