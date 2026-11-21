use memchr::{memchr, memchr2_iter};

use crate::types::{At, TemplateString};
use crate::{DelimitedToken, END_TAG_LEN, START_TAG_LEN};

#[derive(Debug, PartialEq, Eq)]
pub enum TokenType {
    Text,
    Variable,
    Tag,
    Comment,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub at: At,
}

impl Token {
    fn text(at: At) -> Self {
        Self {
            at,
            token_type: TokenType::Text,
        }
    }

    fn variable(at: At) -> Self {
        Self {
            at,
            token_type: TokenType::Variable,
        }
    }

    fn tag(at: At) -> Self {
        Self {
            at,
            token_type: TokenType::Tag,
        }
    }

    fn comment(at: At) -> Self {
        Self {
            at,
            token_type: TokenType::Comment,
        }
    }
}

impl DelimitedToken for Token {
    fn trimmed_at(&self) -> At {
        if matches!(self.token_type, TokenType::Text) {
            self.at
        } else {
            let (start, len) = self.at;
            let start = start + START_TAG_LEN;
            let len = len - START_TAG_LEN - END_TAG_LEN;
            (start, len)
        }
    }
}

impl<'t> Token {
    pub fn content(&self, template: TemplateString<'t>) -> &'t str {
        template.content(self.trimmed_at())
    }
}

pub struct Lexer<'t> {
    template: TemplateString<'t>,
    rest: &'t str,
    byte: usize,
    verbatim: Option<&'t str>,
    // The token found while looking for the end of the text before it.
    pending: Option<Token>,
    // Where the last failed search for each closer stopped: the end, or a newline if one ends it.
    // An opener of that kind before it would rescan a subset of those bytes and fail too.
    unclosed_variable_before: usize,
    unclosed_tag_before: usize,
    unclosed_comment_before: usize,
}

/// Searches `content`, the bytes following an opener, for `closer` followed by `}`.
///
/// Returns the length of `content` up to the end of the closer,
/// or the offset where the search stopped when there is none.
fn find_closer(content: &[u8], closer: u8, newline_ends_search: bool) -> Result<usize, usize> {
    for index in memchr2_iter(b'}', b'\n', content) {
        if content[index] == b'\n' {
            if newline_ends_search {
                return Err(index);
            }
        } else if index > 0 && content[index - 1] == closer {
            return Ok(index + 1);
        }
    }
    Err(content.len())
}

impl<'t> Lexer<'t> {
    pub fn new(template: TemplateString<'t>) -> Self {
        Self {
            template,
            rest: template.0,
            byte: 0,
            verbatim: None,
            pending: None,
            unclosed_variable_before: 0,
            unclosed_tag_before: 0,
            unclosed_comment_before: 0,
        }
    }

    /// Finds the first variable, tag or comment at or after `self.byte`.
    fn find_delimited(&mut self) -> Option<Token> {
        let template = self.template.0.as_bytes();
        // https://code.djangoproject.com/ticket/35899 turns this off for `{% %}`.
        let newline_ends_search = true;
        let mut from = self.byte;
        while let Some(start) = memchr(b'{', &template[from..]).map(|n| from + n) {
            from = start + 1;
            let (closer, unclosed_before, new_token): (_, _, fn(At) -> Token) =
                match template.get(from) {
                    Some(b'{') => (b'}', &mut self.unclosed_variable_before, Token::variable),
                    Some(b'%') => (b'%', &mut self.unclosed_tag_before, Token::tag),
                    Some(b'#') => (b'#', &mut self.unclosed_comment_before, Token::comment),
                    _ => continue,
                };
            if start < *unclosed_before {
                continue;
            }
            let content = start + START_TAG_LEN;
            match find_closer(&template[content..], closer, newline_ends_search) {
                Ok(len) => return Some(new_token((start, START_TAG_LEN + len))),
                Err(stopped) => *unclosed_before = content + stopped,
            }
        }
        None
    }

    fn lex_text_or_delimited(&mut self) -> Token {
        let Some(token) = self.find_delimited() else {
            return self.lex_text_to_end();
        };
        let (start, len) = token.at;
        let text_len = start - self.byte;
        let text = Token::text((self.byte, text_len));
        self.rest = &self.rest[text_len + len..];
        self.byte = start + len;
        if token.token_type == TokenType::Tag {
            let verbatim = token.content(self.template).trim();
            if verbatim == "verbatim" || verbatim.starts_with("verbatim ") {
                self.verbatim = Some(verbatim);
            }
        }
        if text_len == 0 {
            return token;
        }
        self.pending = Some(token);
        text
    }

    fn lex_text_to_end(&mut self) -> Token {
        let len = self.rest.len();
        let at = (self.byte, len);
        self.byte += len;
        self.rest = "";
        Token::text(at)
    }

    fn lex_verbatim(&mut self, verbatim: &'t str) -> Token {
        let verbatim = verbatim.trim();
        self.verbatim = None;

        let mut index = 0;
        loop {
            let Some(start) = self.rest[index..].find("{%").map(|n| index + n) else {
                return self.lex_text_to_end();
            };
            let inner_start = start + START_TAG_LEN;
            let Some(end) = self.rest[inner_start..].find("%}").map(|n| inner_start + n) else {
                return self.lex_text_to_end();
            };
            let inner = &self.rest[inner_start..end];
            if inner.contains('\n') {
                index = inner_start;
                continue;
            }
            if inner.trim().strip_prefix("end") != Some(verbatim) {
                index = end + END_TAG_LEN;
                continue;
            }
            if start == 0 {
                // Return the endverbatim tag since we have no text
                let tag_len = end + END_TAG_LEN;
                let at = (self.byte, tag_len);
                self.byte += tag_len;
                self.rest = &self.rest[tag_len..];
                return Token::tag(at);
            }
            self.rest = &self.rest[start..];
            let at = (self.byte, start);
            self.byte += start;
            return Token::text(at);
        }
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.pending.take() {
            return Some(token);
        }
        if self.rest.is_empty() {
            return None;
        }
        Some(match self.verbatim {
            None => self.lex_text_or_delimited(),
            Some(verbatim) => self.lex_verbatim(verbatim),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn contents<'t>(template: impl Into<TemplateString<'t>>, tokens: Vec<Token>) -> Vec<&'t str> {
        let template = template.into();
        tokens.iter().map(|t| t.content(template)).collect()
    }

    #[test]
    fn test_lex_empty() {
        let template = "";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_text() {
        let template = "Just some text";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::text((0, 14))]);
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_text_whitespace() {
        let template = "    ";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::text((0, 4))]);
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_comment() {
        let template = "{# comment #}";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::comment((0, 13))]);
        assert_eq!(contents(template, tokens), vec![" comment "]);
    }

    #[test]
    fn test_lex_variable() {
        let template = "{{ foo.bar|title }}";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::variable((0, 19))]);
        assert_eq!(contents(template, tokens), vec![" foo.bar|title "]);
    }

    #[test]
    fn test_lex_tag() {
        let template = "{% for foo in bar %}";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::tag((0, 20))]);
        assert_eq!(contents(template, tokens), vec![" for foo in bar "]);
    }

    #[test]
    fn test_lex_incomplete_comment() {
        let template = "{# comment #";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::text((0, 12))]);
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_incomplete_variable() {
        let template = "{{ foo.bar|title }";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::text((0, 18))]);
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_incomplete_tag() {
        let template = "{% for foo in bar %";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::text((0, 19))]);
        assert_eq!(contents(template, tokens), vec![template]);
    }

    #[test]
    fn test_lex_unclosed_opener_is_text() {
        let cases = [
            ("x{%", vec![Token::text((0, 3))]),
            ("{{#", vec![Token::text((0, 3))]),
            ("{%}", vec![Token::text((0, 3))]),
            ("{{ a\n}}", vec![Token::text((0, 7))]),
            ("{% a\r\n b %}", vec![Token::text((0, 11))]),
            ("{{% a %}", vec![Token::text((0, 1)), Token::tag((1, 7))]),
            (
                "{{ a {{ b {% c %}\n{{ d }}",
                vec![
                    Token::text((0, 10)),
                    Token::tag((10, 7)),
                    Token::text((17, 1)),
                    Token::variable((18, 7)),
                ],
            ),
        ];
        for (template, expected) in cases {
            let tokens: Vec<_> = Lexer::new(template.into()).collect();
            assert_eq!(tokens, expected, "{template:?}");
        }
    }

    #[test]
    fn test_lex_first_closer_after_opener() {
        let cases = [
            ("{%%}", vec![Token::tag((0, 4))], vec![""]),
            (
                "{{}}}",
                vec![Token::variable((0, 4)), Token::text((4, 1))],
                vec!["", "}"],
            ),
            ("{{{ a }}", vec![Token::variable((0, 8))], vec!["{ a "]),
        ];
        for (template, expected, expected_contents) in cases {
            let tokens: Vec<_> = Lexer::new(template.into()).collect();
            assert_eq!(tokens, expected, "{template:?}");
            assert_eq!(contents(template, tokens), expected_contents);
        }
    }

    #[test]
    fn test_find_closer_newline() {
        assert_eq!(find_closer(b" a\n%}", b'%', true), Err(2));
        assert_eq!(find_closer(b" a\n%}", b'%', false), Ok(5));
    }

    #[test]
    fn test_django_example() {
        let template = "text\n{% if test %}{{ varvalue }}{% endif %}{#comment {{not a var}} {%not a block%} #}end text";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::text((0, 5)),
                Token::tag((5, 13)),
                Token::variable((18, 14)),
                Token::tag((32, 11)),
                Token::comment((43, 42)),
                Token::text((85, 8)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 14)),
                Token::text((14, 11)),
                Token::tag((25, 17)),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "{{bare   }}", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_after_text() {
        let template = "x{% verbatim %}{{ a }}{% endverbatim %}";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::text((0, 1)),
                Token::tag((1, 14)),
                Token::text((15, 7)),
                Token::tag((22, 17)),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec!["x", " verbatim ", "{{ a }}", " endverbatim "]
        );
    }

    #[test]
    fn test_verbatim_with_tag() {
        let template = "{% verbatim %}{% endif %}{% endverbatim %}";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 14)),
                Token::text((14, 11)),
                Token::tag((25, 17)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 14)),
                Token::text((14, 27)),
                Token::tag((41, 17)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 14)),
                Token::text((14, 14)),
                Token::tag((28, 17)),
                Token::tag((45, 17)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 14)),
                Token::tag((14, 17)),
                Token::tag((31, 14)),
                Token::tag((45, 17)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Token::tag((0, 22)),
                Token::text((22, 32)),
                Token::tag((54, 25)),
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
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::tag((0, 14)), Token::text((14, 9))]);
        assert_eq!(contents(template, tokens), vec![" verbatim ", "Don't {% "]);
    }

    #[test]
    fn test_verbatim_no_tag() {
        let template = "{% verbatim %}Don't end verbatim";
        let lexer = Lexer::new(template.into());
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![Token::tag((0, 14)), Token::text((14, 18))]);
        assert_eq!(
            contents(template, tokens),
            vec![" verbatim ", "Don't end verbatim"]
        );
    }

    #[test]
    fn test_trimmed_at_doesnt_panic() {
        assert_eq!(Token::text((34, 1)).trimmed_at(), (34, 1));
        assert_eq!(Token::text((34, 20)).trimmed_at(), (34, 20));
        assert_eq!(Token::tag((34, 5)).trimmed_at(), (36, 1));
        assert_eq!(Token::variable((34, 5)).trimmed_at(), (36, 1));
        assert_eq!(Token::comment((34, 5)).trimmed_at(), (36, 1));
    }

    #[test]
    fn test_lex_token_invariants() {
        use proptest::prelude::*;

        // Uniformly random strings almost never contain a closed tag, fragments do.
        let fragment = prop::sample::select(vec![
            "{%",
            "%}",
            "{{",
            "}}",
            "{#",
            "#}",
            "{",
            "}",
            "%",
            "#",
            " ",
            "\t",
            "\n",
            "\r",
            "\x0b",
            "\x0c",
            "\u{85}",
            "\u{a0}",
            "\u{2028}",
            "\u{1c}",
            "\u{1f}",
            "verbatim",
            "endverbatim",
            "comment",
            "endcomment",
            "x",
            "a b",
            "é",
            "\"",
            "'",
            "{% verbatim %}",
            "{% endverbatim %}",
            "{% verbatim x %}",
            "{% endverbatim x %}",
            "{{ v }}",
            "{# c #}",
            "{% t %}",
        ]);
        let template_strategy =
            prop::collection::vec(fragment, 0..=40).prop_map(|fragments| fragments.concat());
        // The default 256 cases miss the non-ASCII panic fixed in #588 every other run.
        proptest!(ProptestConfig::with_cases(4096), |(template in template_strategy)| {
            let template = template.as_str();
            let mut end = 0;
            let mut after_text = false;
            for token in Lexer::new(template.into()) {
                let (start, len) = token.at;
                prop_assert_eq!(start, end, "{:?} does not follow the previous token", token);
                let is_text = token.token_type == TokenType::Text;
                prop_assert!(!(after_text && is_text), "{:?} follows another text token", token);
                after_text = is_text;
                prop_assert!(len > 0, "{:?} is empty", token);
                end = start + len;
                prop_assert!(template.is_char_boundary(end), "{:?} ends inside a character", token);

                let (open, close) = match token.token_type {
                    TokenType::Text => ("", ""),
                    TokenType::Variable => ("{{", "}}"),
                    TokenType::Tag => ("{%", "%}"),
                    TokenType::Comment => ("{#", "#}"),
                };
                let inner = template[start..end]
                    .strip_prefix(open)
                    .and_then(|rest| rest.strip_suffix(close));
                let content = token.content(template.into());
                prop_assert_eq!(inner, Some(content), "{:?} lacks its delimiters", token);
            }
            prop_assert_eq!(end, template.len(), "the tokens stop before the end");
        });
    }
}
