use std::borrow::Cow;
use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::parse::{TokenTree, Variable};

impl Variable {
    fn resolve<'py>(
        &self,
        py: Python<'py>,
        template: &str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut parts = self.parts(template);
        let first = parts.next().expect("Variable names cannot be empty");
        let mut variable = match context.get(first) {
            Some(variable) => variable.clone(),
            None => PyString::new(py, "").into_any(),
        };
        for part in parts {
            variable = match variable.get_item(part) {
                Ok(variable) => variable,
                Err(_) => match variable.getattr(part) {
                    Ok(variable) => variable,
                    Err(e) => {
                        let int = match part.parse::<usize>() {
                            Ok(int) => int,
                            Err(_) => return Err(e),
                        };
                        match variable.get_item(int) {
                            Ok(variable) => variable,
                            Err(_) => todo!(),
                        }
                    }
                },
            }
        }
        Ok(variable)
    }

    fn render<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Cow<'t, str>> {
        let variable = match self.resolve(py, template, context) {
            Ok(variable) => variable.str()?.extract::<String>()?,
            Err(_) => "".to_string(),
        };
        Ok(Cow::Owned(variable))
    }
}

impl TokenTree {
    pub fn render<'t>(
        &self,
        py: Python<'_>,
        template: &'t str,
        context: &HashMap<String, Bound<'_, PyAny>>,
    ) -> PyResult<Cow<'t, str>> {
        match self {
            TokenTree::Text(text) => Ok(Cow::Borrowed(text.content(template))),
            TokenTree::TranslatedText(text) => todo!(),
            TokenTree::Tag(tag) => todo!(),
            TokenTree::Variable(variable) => variable.render(py, template, context),
            TokenTree::Filter(filter) => todo!(),
            TokenTree::Float(number) => Ok(Cow::Owned(number.to_string())),
            TokenTree::Int(number) => Ok(Cow::Owned(number.to_string())),
        }
    }
}
