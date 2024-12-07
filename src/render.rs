use std::borrow::Cow;
use std::collections::HashMap;

use num_bigint::BigInt;
use pyo3::prelude::*;

use crate::parse::{Filter, FilterType, TokenTree, Variable};

pub enum Content<'t, 'py> {
    Py(Bound<'py, PyAny>),
    String(Cow<'t, str>),
    Float(f64),
    Int(BigInt),
}

pub trait Render {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>>;

    fn render<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Cow<'t, str>> {
        let content = match self.resolve(py, template, context) {
            Ok(Some(content)) => match content {
                Content::Py(content) => content.str()?.extract::<String>()?,
                Content::String(content) => return Ok(content),
                Content::Float(content) => return Ok(Cow::Owned(content.to_string())),
                Content::Int(content) => return Ok(Cow::Owned(content.to_string())),
            },
            Ok(None) => "".to_string(),
            Err(_) => "".to_string(),
        };
        Ok(Cow::Owned(content))
    }
}

impl Render for Variable {
    fn resolve<'t, 'py>(
        &self,
        _py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let mut parts = self.parts(template);
        let first = parts.next().expect("Variable names cannot be empty");
        let mut variable = match context.get(first) {
            Some(variable) => variable.clone(),
            None => return Ok(None),
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
        Ok(Some(Content::Py(variable)))
    }
}

impl Render for Filter {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let left = self.left.resolve(py, template, context)?;
        match &self.filter {
            FilterType::Default(right) => match left {
                Some(left) => Ok(Some(left)),
                None => Ok(right.resolve(py, template, context)?),
            },
            FilterType::External(_filter) => todo!(),
        }
    }
}

impl Render for TokenTree {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            TokenTree::Text(text) => {
                Ok(Some(Content::String(Cow::Borrowed(text.content(template)))))
            }
            TokenTree::TranslatedText(_text) => todo!(),
            TokenTree::Tag(_tag) => todo!(),
            TokenTree::Variable(variable) => variable.resolve(py, template, context),
            TokenTree::Filter(filter) => filter.resolve(py, template, context),
            TokenTree::Float(number) => Ok(Some(Content::Float(*number))),
            TokenTree::Int(number) => Ok(Some(Content::Int(number.clone()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pyo3::types::{PyDict, PyList, PyString};

    use crate::parse::Text;

    #[test]
    fn test_render_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name)]);
            let template = "{{ name }}";
            let variable = Variable::new((3, 4));

            let rendered = variable.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_dict_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let data = PyDict::new(py);
            let name = PyString::new(py, "Lily");
            data.set_item("name", name).unwrap();
            let context = HashMap::from([("data".to_string(), data.into_any())]);
            let template = "{{ data.name }}";
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_list_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily");
            let names = PyList::new(py, [name]).unwrap();
            let context = HashMap::from([("names".to_string(), names.into_any())]);
            let template = "{{ names.0 }}";
            let variable = Variable::new((3, 7));

            let rendered = variable.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_attribute_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class User:
    def __init__(self, name):
        self.name = name

user = User('Lily')
",
                None,
                Some(&locals),
            ).unwrap();

            let context = locals.extract().unwrap();
            let template = "{{ user.name }}";
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name)]);
            let template = "{{ name|default:'Bryony' }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 7),
                TokenTree::Variable(variable),
                Some(TokenTree::Text(Text::new((17, 6)))),
            ).unwrap();

            let rendered = filter.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let template = "{{ name|default:'Bryony' }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 7),
                TokenTree::Variable(variable),
                Some(TokenTree::Text(Text::new((17, 6)))),
            ).unwrap();

            let rendered = filter.render(py, template, &context).unwrap();
            assert_eq!(rendered, "Bryony");
        })
    }

    #[test]
    fn test_render_filter_default_integer() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let template = "{{ count|default:12}}";
            let variable = Variable::new((3, 5));
            let filter = Filter::new(
                template,
                (9, 7),
                TokenTree::Variable(variable),
                Some(TokenTree::Int(12.into())),
            ).unwrap();

            let rendered = filter.render(py, template, &context).unwrap();
            assert_eq!(rendered, "12");
        })
    }

    #[test]
    fn test_render_filter_default_float() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let template = "{{ count|default:3.5}}";
            let variable = Variable::new((3, 5));
            let filter = Filter::new(
                template,
                (9, 7),
                TokenTree::Variable(variable),
                Some(TokenTree::Float(3.5)),
            ).unwrap();

            let rendered = filter.render(py, template, &context).unwrap();
            assert_eq!(rendered, "3.5");
        })
    }
}
