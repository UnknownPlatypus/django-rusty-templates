use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::cursor::Cursor;
use crate::python_compat::is_whitespace;
use crate::tag::TagParts;
use crate::types::{At, TemplateString};

#[derive(Debug, PartialEq, Eq)]
pub struct BlockToken {
    pub at: At,
}

#[derive(Debug, Error, Diagnostic, PartialEq, Eq)]
pub enum BlockLexerError {
    #[error("'{name}' tag takes only one argument")]
    UnexpectedArguments {
        name: &'static str,
        #[label("unexpected argument(s)")]
        at: SourceSpan,
    },
}

pub enum BlockType {
    Start,
    End,
}

impl BlockType {
    fn name(&self) -> &'static str {
        match self {
            Self::Start => "block",
            Self::End => "endblock",
        }
    }
}

pub fn lex_block(
    template: TemplateString,
    parts: TagParts,
    block_type: BlockType,
) -> Result<Option<BlockToken>, BlockLexerError> {
    let mut cursor = Cursor::new(template.content(parts.at), parts.at.0);
    if cursor.is_empty() {
        return Ok(None);
    }

    let at = cursor.eat_while(|c| !is_whitespace(c));
    cursor.eat_whitespace();
    if cursor.is_empty() {
        Ok(Some(BlockToken { at }))
    } else {
        let at = cursor.skip_to_end();
        Err(BlockLexerError::UnexpectedArguments {
            at: at.into(),
            name: block_type.name(),
        })
    }
}
