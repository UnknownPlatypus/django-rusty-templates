use std::borrow::Cow;
use std::collections::HashMap;

use num_bigint::BigInt;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::parse::{Argument, ArgumentType, Filter, FilterType, Tag, TagElement, Text, TokenTree, Url, Variable};

#[derive(Debug, IntoPyObject)]
pub enum Content<'t, 'py> {
    Py(Bound<'py, PyAny>),
    String(Cow<'t, str>),
    Float(f64),
    Int(BigInt),
}

impl<'t> Content<'t, '_> {
    fn render(self) -> PyResult<Cow<'t, str>> {
        let content = match self {
            Self::Py(content) => content.str()?.extract::<String>()?,
            Self::String(content) => return Ok(content),
            Self::Float(content) => content.to_string(),
            Self::Int(content) => content.to_string(),
        };
        Ok(Cow::Owned(content))
    }
}

pub trait Render {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>>;

    fn render<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Cow<'t, str>> {
        match self.resolve(py, template, context)? {
            Some(content) => content.render(),
            None => Ok(Cow::Borrowed("")),
        }
    }
}

impl Render for Variable {
    fn resolve<'t, 'py>(
        &self,
        _py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
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
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let left = self.left.resolve(py, template, context)?;
        Ok(match &self.filter {
            FilterType::Default(right) => match left {
                Some(left) => Some(left),
                None => right.resolve(py, template, context)?,
            },
            FilterType::External(_filter) => todo!(),
            FilterType::Lower => match left {
                Some(content) => Some(Content::String(Cow::Owned(content.render()?.to_lowercase()))),
                None => Some(Content::String(Cow::Borrowed(""))),
            }
        })
    }
}

impl Render for Url {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let view_name = match self.view_name.resolve(py, template, context)? {
            Some(view_name) => view_name,
            None => Content::String(Cow::Borrowed("")),
        };
        let urls = py.import("django.urls")?;
        let reverse = urls.getattr("reverse")?;
        let url = if self.kwargs.is_empty() {
            let py_args = PyList::empty(py);
            for arg in &self.args {
                py_args.append(arg.resolve(py, template, context)?)?;
            }
            reverse.call1((view_name, py.None(), py_args.to_tuple()))?
        } else {
            let kwargs = PyDict::new(py);
            for (key, value) in &self.kwargs {
                kwargs.set_item(key, value.resolve(py, template, context)?)?;
            }
            reverse.call1((view_name, py.None(), py.None(), kwargs))?
        };
        match &self.variable {
            None => Ok(Some(Content::Py(url))),
            Some(variable) => {
                context.insert(variable.clone(), url);
                Ok(None)
            }
        }
    }
}

impl Render for Tag {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Url(url) => url.resolve(py, template, context)
        }
    }
}

impl Render for Text {
    fn resolve<'t, 'py>(
        &self,
        _py: Python<'py>,
        template: &'t str,
        _context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        Ok(Some(Content::String(Cow::Borrowed(self.content(template)))))
    }
}

impl Render for TagElement {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Text(text) => text.resolve(py, template, context),
            Self::TranslatedText(_text) => todo!(),
            Self::Variable(variable) => variable.resolve(py, template, context),
            Self::Filter(filter) => filter.resolve(py, template, context),
            Self::Int(int) => Ok(Some(Content::Int(int.clone()))),
            Self::Float(float) => Ok(Some(Content::Float(*float))),
        }
    }
}

impl Render for TokenTree {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Text(text) => text.resolve(py, template, context),
            Self::TranslatedText(_text) => todo!(),
            Self::Tag(tag) => tag.resolve(py, template, context),
            Self::Variable(variable) => variable.resolve(py, template, context),
            Self::Filter(filter) => filter.resolve(py, template, context),
        }
    }
}

impl Render for Argument {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: &'t str,
        context: &mut HashMap<String, Bound<'py, PyAny>>,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        Ok(Some(match &self.argument_type {
            ArgumentType::Text(text) => return text.resolve(py, template, context),
            ArgumentType::TranslatedText(_text) => todo!(),
            ArgumentType::Variable(variable) => return variable.resolve(py, template, context),
            ArgumentType::Float(number) => Content::Float(*number),
            ArgumentType::Int(number) => Content::Int(number.clone()),
        }))
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
            let mut context = HashMap::from([("name".to_string(), name)]);
            let template = "{{ name }}";
            let variable = Variable::new((3, 4));

            let rendered = variable.render(py, template, &mut context).unwrap();
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
            let mut context = HashMap::from([("data".to_string(), data.into_any())]);
            let template = "{{ data.name }}";
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_list_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily");
            let names = PyList::new(py, [name]).unwrap();
            let mut context = HashMap::from([("names".to_string(), names.into_any())]);
            let template = "{{ names.0 }}";
            let variable = Variable::new((3, 7));

            let rendered = variable.render(py, template, &mut context).unwrap();
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

            let mut context = locals.extract().unwrap();
            let template = "{{ user.name }}";
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let mut context = HashMap::from([("name".to_string(), name)]);
            let template = "{{ name|default:'Bryony' }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 7),
                TagElement::Variable(variable),
                Some(Argument { at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut context = HashMap::new();
            let template = "{{ name|default:'Bryony' }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 7),
                TagElement::Variable(variable),
                Some(Argument{ at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Bryony");
        })
    }

    #[test]
    fn test_render_filter_default_integer() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut context = HashMap::new();
            let template = "{{ count|default:12}}";
            let variable = Variable::new((3, 5));
            let filter = Filter::new(
                template,
                (9, 7),
                TagElement::Variable(variable),
                Some(Argument { at: (17, 2), argument_type: ArgumentType::Int(12.into())}),
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "12");
        })
    }

    #[test]
    fn test_render_filter_default_float() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut context = HashMap::new();
            let template = "{{ count|default:3.5}}";
            let variable = Variable::new((3, 5));
            let filter = Filter::new(
                template,
                (9, 7),
                TagElement::Variable(variable),
                Some(Argument{ at: (17, 3), argument_type: ArgumentType::Float(3.5)}),
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "3.5");
        })
    }

    #[test]
    fn test_render_filter_default_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let me = PyString::new(py, "Lily").into_any();
            let mut context = HashMap::from([("me".to_string(), me)]);
            let template = "{{ name|default:me}}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 7),
                TagElement::Variable(variable),
                Some(Argument{ at: (16, 2), argument_type: ArgumentType::Variable(Variable::new((16, 2)))}),
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_lower() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let mut context = HashMap::from([("name".to_string(), name)]);
            let template = "{{ name|lower }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 5),
                TagElement::Variable(variable),
                None,
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "lily");
        })
    }

    #[test]
    fn test_render_filter_lower_missing_left() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut context = HashMap::new();
            let template = "{{ name|lower }}";
            let variable = Variable::new((3, 4));
            let filter = Filter::new(
                template,
                (8, 5),
                TagElement::Variable(variable),
                None,
            ).unwrap();

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "");
        })
    }

    #[test]
    fn test_render_chained_filters() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut context = HashMap::new();
            let template = "{{ name|default:'Bryony'|lower }}";
            let variable = Variable::new((3, 4));
            let default = Filter::new(
                template,
                (8, 7),
                TagElement::Variable(variable),
                Some(Argument { at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            ).unwrap();
            let lower = Filter::new(
                template,
                (25, 5),
                TagElement::Filter(Box::new(default)),
                None,
            ).unwrap();

            let rendered = lower.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "bryony");
        })
    }
}
