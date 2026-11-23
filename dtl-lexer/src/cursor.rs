use crate::python_compat::is_whitespace;
use crate::types::At;

/// A position in a piece of template source that knows its absolute offset in the template.
#[derive(Clone, Debug)]
pub struct Cursor<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> Cursor<'t> {
    /// `rest` is the source left to lex and `byte` its absolute offset in the template.
    pub fn new(rest: &'t str, byte: usize) -> Self {
        Self { rest, byte }
    }

    pub fn rest(&self) -> &'t str {
        self.rest
    }

    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// The absolute offset of the next character.
    ///
    /// Taken at the start of a token, it is the mark `at_from` measures from.
    pub fn pos(&self) -> usize {
        self.byte
    }

    /// The location of everything consumed since `start`, a mark taken with `pos`.
    pub fn at_from(&self, start: usize) -> At {
        (start, self.byte - start)
    }

    pub fn first(&self) -> Option<char> {
        self.rest.chars().next()
    }

    pub fn second(&self) -> Option<char> {
        self.rest.chars().nth(1)
    }

    pub fn bump(&mut self) -> Option<char> {
        let mut chars = self.rest.chars();
        let c = chars.next()?;
        let rest = chars.as_str();
        self.byte += self.rest.len() - rest.len();
        self.rest = rest;
        Some(c)
    }

    /// Consumes `len` bytes, which must end on a character boundary, and returns their location.
    pub fn bump_bytes(&mut self, len: usize) -> At {
        let at = (self.byte, len);
        self.rest = &self.rest[len..];
        self.byte += len;
        at
    }

    /// Consumes characters while `predicate` holds and returns their location.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) -> At {
        let len = self.rest.find(|c| !predicate(c)).unwrap_or(self.rest.len());
        self.bump_bytes(len)
    }

    pub fn eat_whitespace(&mut self) {
        self.eat_while(is_whitespace);
    }

    /// Consumes everything left and returns its location.
    pub fn skip_to_end(&mut self) -> At {
        self.bump_bytes(self.rest.len())
    }
}
