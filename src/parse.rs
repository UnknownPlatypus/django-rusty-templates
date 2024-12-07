use miette::{Diagnostic, SourceSpan};
use num_bigint::BigInt;
use thiserror::Error;

use crate::lex::{
    lex_variable, Argument, ArgumentType, Lexer, TokenType, VariableLexerError, START_TAG_LEN,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable {
    at: (usize, usize),
}

impl<'t> Variable {
    fn new(at: (usize, usize)) -> Self {
        Self { at }
    }

    fn content(&self, template: &'t str) -> &'t str {
        let (start, len) = self.at;
        &template[start..start + len]
    }

    pub fn parts(&self, template: &'t str) -> impl Iterator<Item = &'t str> {
        let variable = self.content(template);
        variable.split(".")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    at: (usize, usize),
}

impl<'t> Text {
    fn new(at: (usize, usize)) -> Self {
        Self { at }
    }

    pub fn content(&self, template: &'t str) -> &'t str {
        let (start, len) = self.at;
        &template[start..start + len]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    External {
        at: (usize, usize),
        left: TokenTree,
        right: Option<TokenTree>,
    },
}

impl Filter {
    fn new(_template: &str, at: (usize, usize), left: TokenTree, right: Option<TokenTree>) -> Self {
        Self::External { at, left, right }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenTree {
    Text(Text),
    TranslatedText(Text),
    Tag(Tag),
    Variable(Variable),
    Filter(Box<Filter>),
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

    pub fn parse(&mut self) -> Result<Vec<TokenTree>, ParseError> {
        let mut nodes = Vec::new();
        while let Some(token) = self.lexer.next() {
            nodes.push(match token.token_type {
                TokenType::Text => TokenTree::Text(Text::new(token.at)),
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
    ) -> Result<TokenTree, ParseError> {
        let (variable_token, filter_lexer) = match lex_variable(variable, at.0 + START_TAG_LEN)? {
            None => return Err(ParseError::EmptyVariable { at: at.into() }),
            Some(t) => t,
        };
        let mut var = TokenTree::Variable(Variable::new(variable_token.at));
        for filter_token in filter_lexer {
            let filter_token = filter_token?;
            let argument = match filter_token.argument {
                None => None,
                Some(ref a) => Some(a.parse(self.template)?),
            };
            let filter = Filter::new(self.template, filter_token.at, var, argument);
            var = TokenTree::Filter(Box::new(filter));
        }
        Ok(var)
    }

    fn parse_tag(&mut self, _tag: &'t str, _at: (usize, usize)) -> Result<TokenTree, ParseError> {
        todo!()
    }
}

impl Argument {
    fn parse(&self, template: &'_ str) -> Result<TokenTree, ParseError> {
        Ok(match self.argument_type {
            ArgumentType::Variable => TokenTree::Variable(Variable::new(self.at)),
            ArgumentType::Text => TokenTree::Text(Text::new(self.content_at())),
            ArgumentType::Numeric => match self.content(template).parse::<BigInt>() {
                Ok(n) => TokenTree::Int(n),
                Err(_) => match self.content(template).parse::<f64>() {
                    Ok(f) => TokenTree::Float(f),
                    Err(_) => return Err(ParseError::InvalidNumber { at: self.at.into() }),
                },
            },
            ArgumentType::TranslatedText => TokenTree::TranslatedText(Text::new(self.content_at())),
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
        let text = Text::new((0, template.len()));
        assert_eq!(nodes, vec![TokenTree::Text(text)]);
        assert_eq!(text.content(template), template);
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
        let variable = Variable { at: (3, 3) };
        assert_eq!(nodes, vec![TokenTree::Variable(variable)]);
        assert_eq!(variable.parts(template).collect::<Vec<_>>(), vec!["foo"]);
    }

    #[test]
    fn test_variable_attribute() {
        let template = "{{ foo.bar.baz }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();
        let variable = Variable { at: (3, 11) };
        assert_eq!(nodes, vec![TokenTree::Variable(variable)]);
        assert_eq!(
            variable.parts(template).collect::<Vec<_>>(),
            vec!["foo", "bar", "baz"]
        );
    }

    #[test]
    fn test_filter() {
        let template = "{{ foo|bar }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = Variable { at: (3, 3) };
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
            left: TokenTree::Variable(foo),
            right: None,
        }));
        assert_eq!(nodes, vec![bar]);
        assert_eq!(foo.parts(template).collect::<Vec<_>>(), vec!["foo"]);
    }

    #[test]
    fn test_filter_multiple() {
        let template = "{{ foo|bar|baz }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
            left: foo,
            right: None,
        }));
        let baz = TokenTree::Filter(Box::new(Filter::External {
            at: (11, 3),
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

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let baz = Variable { at: (11, 3) };
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
            left: foo,
            right: Some(TokenTree::Variable(baz)),
        }));
        assert_eq!(nodes, vec![bar]);
        assert_eq!(baz.parts(template).collect::<Vec<_>>(), vec!["baz"]);
    }

    #[test]
    fn test_filter_argument_text() {
        let template = "{{ foo|bar:'baz' }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let baz = Text::new((12, 3));
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
            left: foo,
            right: Some(TokenTree::Text(baz)),
        }));
        assert_eq!(nodes, vec![bar]);
        assert_eq!(baz.content(template), "baz");
    }

    #[test]
    fn test_filter_argument_translated_text() {
        let template = "{{ foo|bar:_('baz') }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let baz = Text::new((14, 3));
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
            left: foo,
            right: Some(TokenTree::TranslatedText(baz)),
        }));
        assert_eq!(nodes, vec![bar]);
        assert_eq!(baz.content(template), "baz");
    }

    #[test]
    fn test_filter_argument_float() {
        let template = "{{ foo|bar:5.2e3 }}";
        let mut parser = Parser::new(template);
        let nodes = parser.parse().unwrap();

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let num = TokenTree::Float(5.2e3);
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
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

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let num = TokenTree::Int(99.into());
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
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

        let foo = TokenTree::Variable(Variable { at: (3, 3) });
        let num = TokenTree::Int("99999999999999999".parse::<BigInt>().unwrap());
        let bar = TokenTree::Filter(Box::new(Filter::External {
            at: (7, 3),
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
