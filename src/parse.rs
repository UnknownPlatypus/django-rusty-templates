use miette::{Diagnostic, SourceSpan};
use num_bigint::BigInt;
use thiserror::Error;

use crate::lex::{
    Lexer, Token, VariableLexer, VariableLexerError, VariableToken, VariableTokenType,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Tag {}

#[derive(Debug, PartialEq, Eq)]
pub struct Variable<'t> {
    parts: Vec<&'t str>,
    at: (usize, usize),
}

impl<'t> Variable<'t> {
    fn new(variable: &'t str, at: (usize, usize)) -> Self {
        Self {
            parts: variable.split(".").collect(),
            at,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Filter<'t> {
    External {
        name: &'t str,
        left: TokenTree<'t>,
        right: Option<TokenTree<'t>>,
    },
}

impl<'t> Filter<'t> {
    fn new(name: &'t str, left: TokenTree<'t>, right: Option<TokenTree<'t>>) -> Self {
        Self::External { name, left, right }
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenTree<'t> {
    Text(&'t str),
    TranslatedText(&'t str),
    Tag(Tag),
    Variable(Variable<'t>),
    Filter(Box<Filter<'t>>),
    Float(f64),
    Int(BigInt),
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum ParseError {
    #[error("Empty variable tag")]
    EmptyVariable {
        #[label("here")]
        at: SourceSpan,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] VariableLexerError),
    #[error("Invalid numeric literal")]
    InvalidNumber {
        #[label("here")]
        at: SourceSpan,
    },
}

pub struct Parser<'t> {
    template: &'t str,
    lexer: Lexer<'t>,
}

impl<'t> Parser<'t> {
    pub fn new(template: &'t str) -> Self {
        Self {
            template,
            lexer: Lexer::new(template),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<TokenTree<'t>>, ParseError> {
        let mut nodes = Vec::new();
        while let Some(token) = self.lexer.next() {
            nodes.push(match token {
                Token::Text { text, .. } => TokenTree::Text(text),
                Token::Comment { .. } => continue,
                Token::Variable { variable, at } => self.parse_variable(variable, at)?,
                Token::Tag { tag, at } => self.parse_tag(tag, at)?,
            })
        }
        Ok(nodes)
    }

    fn parse_variable(
        &self,
        variable: &'t str,
        at: (usize, usize),
    ) -> Result<TokenTree<'t>, ParseError> {
        let mut variable_lexer = VariableLexer::new(variable, at.0).peekable();
        let token = match variable_lexer.next() {
            None => return Err(ParseError::EmptyVariable { at: at.into() }),
            Some(token) => token?,
        };
        let mut var = token
            .parse_variable()
            .expect("The first token from VariableLexer is always a VariableTokenType::Variable");
        while let Some(filter) = variable_lexer.next() {
            let filter = filter?;
            let argument = match variable_lexer.peek() {
                Some(token) => match token {
                    Ok(VariableToken {
                        token_type: VariableTokenType::Filter,
                        ..
                    }) => None,
                    _ => variable_lexer
                        .next()
                        .expect("peek is Some")?
                        .parse_argument()?,
                },
                None => None,
            };
            let filter = match filter.token_type {
                VariableTokenType::Filter => Filter::new(filter.content, var, argument),
                _ => unreachable!("Expected a VariableTokenType::Filter"),
            };
            var = TokenTree::Filter(Box::new(filter));
        }
        Ok(var)
    }

    fn parse_tag(&mut self, tag: &'t str, at: (usize, usize)) -> Result<TokenTree<'t>, ParseError> {
        todo!()
    }
}

impl<'t> VariableToken<'t> {
    fn parse_variable(self) -> Option<TokenTree<'t>> {
        match self.token_type {
            VariableTokenType::Variable => {
                Some(TokenTree::Variable(Variable::new(self.content, self.at)))
            }
            _ => None,
        }
    }

    fn parse_argument(self) -> Result<Option<TokenTree<'t>>, ParseError> {
        Ok(Some(match self.token_type {
            VariableTokenType::Filter => return Ok(None),
            VariableTokenType::Variable => {
                TokenTree::Variable(Variable::new(self.content, self.at))
            }
            VariableTokenType::Text => TokenTree::Text(self.content),
            VariableTokenType::Numeric => match self.content.parse::<BigInt>() {
                Ok(n) => TokenTree::Int(n),
                Err(_) => match self.content.parse::<f64>() {
                    Ok(f) => TokenTree::Float(f),
                    Err(_) => return Err(ParseError::InvalidNumber { at: self.at.into() }),
                },
            },
            VariableTokenType::TranslatedText => TokenTree::TranslatedText(self.content),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_template() {
        let template = "";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        assert_eq!(nodes, vec![]);
    }

    #[test]
    fn test_text() {
        let template = "Some text";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        assert_eq!(nodes, vec![TokenTree::Text(template)]);
    }

    #[test]
    fn test_comment() {
        let template = "{# A commment #}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        assert_eq!(nodes, vec![]);
    }

    #[test]
    fn test_empty_variable() {
        let template = "{{ }}";
        let mut parser = Parser::new(template);
        let error = parser.parse().unwrap_err();
        assert_eq!(error, ParseError::EmptyVariable { at: (0, 5).into() });
    }

    #[test]
    fn test_variable() {
        let template = "{{ foo }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        assert_eq!(
            nodes,
            vec![TokenTree::Variable(Variable {
                parts: vec!["foo"],
                at: (3, 3)
            })]
        );
    }

    #[test]
    fn test_variable_attribute() {
        let template = "{{ foo.bar.baz }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        assert_eq!(
            nodes,
            vec![TokenTree::Variable(Variable {
                parts: vec!["foo", "bar", "baz"],
                at: (3, 11),
            })]
        );
    }

    #[test]
    fn test_filter() {
        let template = "{{ foo|bar }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: None,
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_multiple() {
        let template = "{{ foo|bar|baz }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: None,
        }));
        let baz = TokenTree::Filter(Box::new(Filter::External {
            name: "baz",
            left: bar,
            right: None,
        }));
        assert_eq!(nodes, vec![baz]);
    }

    #[test]
    fn test_filter_argument() {
        let template = "{{ foo|bar:baz }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let baz = TokenTree::Variable(Variable {
            parts: vec!["baz"],
            at: (11, 3),
        });
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(baz),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_argument_text() {
        let template = "{{ foo|bar:'baz' }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let baz = TokenTree::Text("baz");
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(baz),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_argument_translated_text() {
        let template = "{{ foo|bar:_('baz') }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let baz = TokenTree::TranslatedText("baz");
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(baz),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_argument_float() {
        let template = "{{ foo|bar:5.2e3 }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let num = TokenTree::Float(5.2e3);
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(num),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_argument_int() {
        let template = "{{ foo|bar:99 }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let num = TokenTree::Int(99.into());
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(num),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_filter_argument_bigint() {
        let template = "{{ foo|bar:99999999999999999 }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable {
            parts: vec!["foo"],
            at: (3, 3),
        });
        let num = TokenTree::Int("99999999999999999".parse::<BigInt>().unwrap());
        let bar = TokenTree::Filter(Box::new(Filter::External {
            name: "bar",
            left: foo,
            right: Some(num),
        }));
        assert_eq!(nodes, vec![bar]);
    }

    #[test]
    fn test_variable_lexer_error() {
        let template = "{{ _foo }}";
        let mut parser = Parser::new(template);
        let error = parser.parse().unwrap_err();
        assert_eq!(
            error,
            ParseError::LexerError(VariableLexerError::InvalidVariableName { at: (3, 4).into() })
        );
    }
}
