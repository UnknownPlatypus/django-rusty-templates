use miette::{Diagnostic, SourceSpan};
use num_bigint::BigInt;
use thiserror::Error;

use crate::lex::{
    lex_variable, Argument, ArgumentType, Lexer, TokenType, VariableLexerError, START_TAG_LEN,
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
            nodes.push(match token.token_type {
                TokenType::Text => TokenTree::Text(token.content(self.template)),
                TokenType::Comment => continue,
                TokenType::Variable => {
                    self.parse_variable(token.content(self.template), token.at)?
                }
                TokenType::Tag => self.parse_tag(token.content(self.template), token.at)?,
            })
        }
        Ok(nodes)
    }

    fn parse_variable(
        &self,
        variable: &'t str,
        at: (usize, usize),
    ) -> Result<TokenTree<'t>, ParseError> {
        let (variable_token, filter_lexer) = match lex_variable(variable, at.0 + START_TAG_LEN)? {
            None => return Err(ParseError::EmptyVariable { at: at.into() }),
            Some(t) => t,
        };
        let mut var = TokenTree::Variable(Variable::new(
            variable_token.content(self.template),
            variable_token.at,
        ));
        for filter_token in filter_lexer {
            let filter_token = filter_token?;
            let argument = match filter_token.argument {
                None => None,
                Some(ref a) => Some(a.parse(self.template)?),
            };
            let filter = Filter::new(filter_token.content(self.template), var, argument);
            var = TokenTree::Filter(Box::new(filter));
        }
        Ok(var)
    }

    fn parse_tag(&mut self, tag: &'t str, at: (usize, usize)) -> Result<TokenTree<'t>, ParseError> {
        todo!()
    }
}

impl<'t> Argument {
    fn parse(&self, template: &'t str) -> Result<TokenTree<'t>, ParseError> {
        Ok(match self.argument_type {
            ArgumentType::Variable => {
                TokenTree::Variable(Variable::new(self.content(template), self.at))
            }
            ArgumentType::Text => TokenTree::Text(self.content(template)),
            ArgumentType::Numeric => match self.content(template).parse::<BigInt>() {
                Ok(n) => TokenTree::Int(n),
                Err(_) => match self.content(template).parse::<f64>() {
                    Ok(f) => TokenTree::Float(f),
                    Err(_) => return Err(ParseError::InvalidNumber { at: self.at.into() }),
                },
            },
            ArgumentType::TranslatedText => TokenTree::TranslatedText(self.content(template)),
        })
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
