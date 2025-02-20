use crate::types::TemplateString;
use num_bigint::BigInt;
use pyo3::prelude::*;

struct PartsIterator<'t> {
    variable: &'t str,
    start: usize,
}

impl<'t> Iterator for PartsIterator<'t> {
    type Item = (&'t str, (usize, usize));

    fn next(&mut self) -> Option<Self::Item> {
        if self.variable.is_empty() {
            return None;
        }

        match self.variable.find('.') {
            Some(index) => {
                let part = &self.variable[..index];
                let at = (self.start, index);
                self.start += index + 1;
                self.variable = &self.variable[index + 1..];
                Some((part, at))
            }
            None => {
                let part = self.variable;
                self.variable = "";
                Some((part, (self.start, part.len())))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub at: (usize, usize),
}

impl Text {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable {
    pub at: (usize, usize),
}

impl<'t> Variable {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }

    pub fn parts(
        &self,
        template: TemplateString<'t>,
    ) -> impl Iterator<Item = (&'t str, (usize, usize))> {
        let start = self.at.0;
        let variable = template.content(self.at);
        PartsIterator { variable, start }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentType {
    Variable(Variable),
    Text(Text),
    TranslatedText(Text),
    Int(BigInt),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub at: (usize, usize),
    pub argument_type: ArgumentType,
}

#[derive(Debug)]
pub enum FilterType {
    Add(Argument, AddFilter),
    AddSlashes(AddSlashesFilter),
    Capfirst(CapfirstFilter),
    Default(Argument, DefaultFilter),
    External(Py<PyAny>, Option<Argument>, ExternalFilter),
    Lower(LowerFilter),
}

#[derive(Debug)]
pub struct AddSlashesFilter;

#[derive(Debug)]
pub struct AddFilter;

#[derive(Debug)]
pub struct CapfirstFilter;

#[derive(Debug)]
pub struct DefaultFilter;

#[derive(Debug)]
pub struct ExternalFilter;

#[derive(Debug)]
pub struct LowerFilter;
