use crate::cursor::Cursor;
use crate::python_compat::is_whitespace;
use crate::tag::TagParts;
use crate::types::{At, TemplateString};

#[derive(Debug, PartialEq, Eq)]
pub struct LoadToken {
    pub at: At,
}

pub struct LoadLexer<'t> {
    cursor: Cursor<'t>,
}

impl<'t> LoadLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            cursor: Cursor::new(template.content(parts.at), parts.at.0),
        }
    }
}

impl Iterator for LoadLexer<'_> {
    type Item = LoadToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }
        let at = self.cursor.eat_while(|c| !is_whitespace(c));
        self.cursor.eat_whitespace();
        Some(LoadToken { at })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_library() {
        let template = "{% load foo %}";
        let parts = TagParts { at: (8, 3) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        assert_eq!(tokens, [foo]);
    }

    #[test]
    fn test_lex_libraries() {
        let template = "{% load foo bar.eggs %}";
        let parts = TagParts { at: (8, 12) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        let bar_eggs = LoadToken { at: (12, 8) };
        assert_eq!(tokens, [foo, bar_eggs]);
    }

    #[test]
    fn test_lex_individual() {
        let template = "{% load foo bar from library %}";
        let parts = TagParts { at: (8, 20) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        let bar = LoadToken { at: (12, 3) };
        let from = LoadToken { at: (16, 4) };
        let library = LoadToken { at: (21, 7) };
        assert_eq!(tokens, [foo, bar, from, library]);
    }
}
