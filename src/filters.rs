use pyo3::prelude::*;

use crate::types::Argument;

#[derive(Debug)]
pub enum FilterType {
    Add(AddFilter),
    AddSlashes(AddSlashesFilter),
    Capfirst(CapfirstFilter),
    Default(DefaultFilter),
    Escape(EscapeFilter),
    External(ExternalFilter),
    Lower(LowerFilter),
    Safe(SafeFilter),
    Slugify(SlugifyFilter),
}

#[derive(Debug)]
pub struct AddSlashesFilter;

#[derive(Debug)]
pub struct AddFilter {
    pub argument: Argument,
}

impl AddFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument }
    }
}

#[derive(Debug)]
pub struct CapfirstFilter;

#[derive(Debug)]
pub struct DefaultFilter {
    pub argument: Argument,
}

impl DefaultFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument }
    }
}

#[derive(Debug)]
pub struct EscapeFilter;

#[derive(Debug)]
pub struct ExternalFilter {
    pub filter: Py<PyAny>,
    pub argument: Option<Argument>,
}

impl ExternalFilter {
    pub fn new(filter: Py<PyAny>, argument: Option<Argument>) -> Self {
        Self { filter, argument }
    }
}

#[derive(Debug)]
pub struct LowerFilter;

#[derive(Debug)]
pub struct SafeFilter;

#[derive(Debug)]
pub struct SlugifyFilter;
