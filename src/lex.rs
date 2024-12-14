mod core;
mod variable;

pub use core::{Lexer, TokenType};
pub use variable::{lex_variable, Argument, ArgumentType, VariableLexerError};

pub const START_TAG_LEN: usize = 2;
const END_TAG_LEN: usize = 2;
