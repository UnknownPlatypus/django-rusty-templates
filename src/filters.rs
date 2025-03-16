use std::sync::Arc;

use pyo3::prelude::*;

use crate::types::Argument;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct AddSlashesFilter;

#[derive(Clone, Debug)]
pub struct AddFilter {
    pub argument: Argument,
}

impl AddFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument }
    }
}

#[derive(Clone, Debug)]
pub struct CapfirstFilter;

#[derive(Clone, Debug)]
pub struct DefaultFilter {
    pub argument: Argument,
}

impl DefaultFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument }
    }
}

#[derive(Clone, Debug)]
pub struct EscapeFilter;

#[derive(Clone, Debug)]
pub struct ExternalFilter {
    pub filter: Arc<Py<PyAny>>,
    pub argument: Option<Argument>,
}

impl ExternalFilter {
    pub fn new(filter: Py<PyAny>, argument: Option<Argument>) -> Self {
        Self {
            filter: Arc::new(filter),
            argument,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LowerFilter;

#[derive(Clone, Debug)]
pub struct SafeFilter;

#[derive(Clone, Debug)]
pub struct SlugifyFilter;
