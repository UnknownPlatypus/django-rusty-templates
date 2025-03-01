use std::borrow::Cow;

use html_escape::encode_quoted_attribute_to_string;
use pyo3::prelude::*;

use crate::filters::{
    AddFilter, AddSlashesFilter, CapfirstFilter, DefaultFilter, EscapeFilter, ExternalFilter,
    FilterType, LowerFilter, SafeFilter, SlugifyFilter,
};
use crate::parse::Filter;
use crate::render::types::{Content, Context};
use crate::render::{Resolve, ResolveResult};
use crate::types::TemplateString;

trait IntoOwnedContent<'t, 'py> {
    fn into_content(self) -> Option<Content<'t, 'py>>;
}

trait AsBorrowedContent<'a, 't, 'py>
where
    'a: 't,
{
    fn as_content(&'a self) -> Option<Content<'t, 'py>>;
}

impl<'a, 't, 'py> AsBorrowedContent<'a, 't, 'py> for str
where
    'a: 't,
{
    fn as_content(&'a self) -> Option<Content<'t, 'py>> {
        Some(Content::String(Cow::Borrowed(self)))
    }
}

impl<'t, 'py> IntoOwnedContent<'t, 'py> for String {
    fn into_content(self) -> Option<Content<'t, 'py>> {
        Some(Content::String(Cow::Owned(self)))
    }
}

impl Resolve for Filter {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let left = self.left.resolve(py, template, context)?;
        let result = match &self.filter {
            FilterType::Add(filter) => filter.resolve(left, py, template, context),
            FilterType::AddSlashes(filter) => filter.resolve(left, py, template, context),
            FilterType::Capfirst(filter) => filter.resolve(left, py, template, context),
            FilterType::Default(filter) => filter.resolve(left, py, template, context),
            FilterType::Escape(filter) => filter.resolve(left, py, template, context),
            FilterType::External(filter) => filter.resolve(left, py, template, context),
            FilterType::Lower(filter) => filter.resolve(left, py, template, context),
            FilterType::Safe(filter) => filter.resolve(left, py, template, context),
            FilterType::Slugify(filter) => filter.resolve(left, py, template, context),
        };
        result
    }
}

pub trait ResolveFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py>;
}

impl ResolveFilter for AddSlashesFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let content = match variable {
            Some(content) => content
                .render(context)?
                .replace(r"\", r"\\")
                .replace("\"", "\\\"")
                .replace("'", r"\'")
                .into_content(),
            None => "".as_content(),
        };
        Ok(content)
    }
}

impl ResolveFilter for AddFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let variable = match variable {
            Some(left) => left,
            None => return Ok(None),
        };
        let right = self
            .argument
            .resolve(py, template, context)?
            .expect("missing argument in context should already have raised");
        match (variable.to_bigint(), right.to_bigint()) {
            (Some(variable), Some(right)) => Ok(Some(Content::Int(variable + right))),
            _ => {
                let variable = variable.to_py(py)?;
                let right = right.to_py(py)?;
                match variable.add(right) {
                    Ok(sum) => Ok(Some(Content::Py(sum))),
                    Err(_) => Ok(None),
                }
            }
        }
    }
}

impl ResolveFilter for CapfirstFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let content = match variable {
            Some(content) => {
                let content_string = content.render(context)?.into_owned();
                let mut chars = content_string.chars();
                let first_char = match chars.next() {
                    Some(c) => c.to_uppercase(),
                    None => return Ok("".as_content()),
                };
                let string: String = first_char.chain(chars).collect();
                string.into_content()
            }
            None => "".as_content(),
        };
        Ok(content)
    }
}

impl ResolveFilter for DefaultFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let content = match variable {
            Some(left) => Some(left),
            None => self.argument.resolve(py, template, context)?,
        };
        Ok(content)
    }
}

impl ResolveFilter for EscapeFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        _context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        Ok(match variable {
            Some(content) => match content {
                Content::HtmlSafe(content) => Some(Content::HtmlSafe(content)),
                Content::String(content) => {
                    let mut encoded = String::new();
                    encode_quoted_attribute_to_string(&content, &mut encoded);
                    Some(Content::HtmlSafe(Cow::Owned(encoded)))
                }
                Content::Int(n) => Some(Content::HtmlSafe(Cow::Owned(n.to_string()))),
                Content::Float(n) => Some(Content::HtmlSafe(Cow::Owned(n.to_string()))),
                Content::Py(object) => {
                    let content = object.str()?.extract::<String>()?;
                    let mut encoded = String::new();
                    encode_quoted_attribute_to_string(&content, &mut encoded);
                    Some(Content::HtmlSafe(Cow::Owned(encoded)))
                }
            },
            None => Some(Content::HtmlSafe(Cow::Borrowed(""))),
        })
    }
}

impl ResolveFilter for ExternalFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let arg = match &self.argument {
            Some(arg) => arg.resolve(py, template, context)?,
            None => None,
        };
        let filter = self.filter.bind(py);
        let value = match arg {
            Some(arg) => filter.call1((variable, arg))?,
            None => filter.call1((variable,))?,
        };
        Ok(Some(Content::Py(value)))
    }
}

impl ResolveFilter for LowerFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let content = match variable {
            Some(content) => Some(
                content
                    .resolve_string(context)?
                    .map_content(|content| Cow::Owned(content.to_lowercase())),
            ),
            None => "".as_content(),
        };
        Ok(content)
    }
}

impl ResolveFilter for SafeFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        _context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        let content = match variable {
            Some(content) => match content {
                Content::HtmlSafe(content) => Some(Content::HtmlSafe(content)),
                Content::String(content) => Some(Content::HtmlSafe(content)),
                Content::Int(n) => Some(Content::HtmlSafe(Cow::Owned(n.to_string()))),
                Content::Float(n) => Some(Content::HtmlSafe(Cow::Owned(n.to_string()))),
                Content::Py(object) => {
                    let content = object.str()?.extract::<String>()?;
                    Some(Content::HtmlSafe(Cow::Owned(content)))
                }
            },
            None => Some(Content::HtmlSafe(Cow::Borrowed(""))),
        };
        Ok(content)
    }
}

impl ResolveFilter for SlugifyFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py> {
        Ok("".as_content())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{AddSlashesFilter, DefaultFilter, LowerFilter};
    use crate::parse::TagElement;
    use crate::render::Render;
    use crate::template::django_rusty_templates::{EngineData, Template};
    use crate::types::{Argument, ArgumentType, Text, Variable};

    use pyo3::types::{PyDict, PyString};

    use std::collections::HashMap;

    #[test]
    fn test_render_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_addslashes_single() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "'hello'").into_any();
            let context = HashMap::from([("quotes".to_string(), name.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ quotes|addslashes }}");
            let variable = Variable::new((3, 6));
            let filter = Filter {
                at: (10, 10),
                left: TagElement::Variable(variable),
                filter: FilterType::AddSlashes(AddSlashesFilter),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, r"\'hello\'");
        })
    }

    #[test]
    fn test_render_filter_capfirst() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let template_string = "{{ var|capfirst }}".to_string();
            let context = PyDict::new(py);
            context.set_item("var", "hello world").unwrap();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "Hello world");

            let context = PyDict::new(py);
            context.set_item("var", "").unwrap();
            let template_string = "{{ var|capfirst }}".to_string();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "");

            let context = PyDict::new(py);
            context.set_item("bar", "").unwrap();
            let template_string = "{{ var|capfirst }}".to_string();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "");

            let template_string = "{{ var|capfirst:invalid }}".to_string();
            let error = Template::new_from_string(py, template_string, &engine).unwrap_err();

            let error_string = format!("{error}");
            assert!(error_string.contains("capfirst filter does not take an argument"));
        })
    }

    #[test]
    fn test_render_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Bryony");
        })
    }

    #[test]
    fn test_render_filter_default_integer() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ count|default:12}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (17, 2),
                    argument_type: ArgumentType::Int(12.into()),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "12");
        })
    }

    #[test]
    fn test_render_filter_default_float() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ count|default:3.5}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (17, 3),
                    argument_type: ArgumentType::Float(3.5),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "3.5");
        })
    }

    #[test]
    fn test_render_filter_default_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let me = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("me".to_string(), me.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:me}}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 2),
                    argument_type: ArgumentType::Variable(Variable::new((16, 2))),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_lower() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower(LowerFilter),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "lily");
        })
    }

    #[test]
    fn test_render_filter_lower_missing_left() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower(LowerFilter),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "");
        })
    }

    #[test]
    fn test_render_chained_filters() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony'|lower }}");
            let variable = Variable::new((3, 4));
            let default = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
            };
            let lower = Filter {
                at: (25, 5),
                left: TagElement::Filter(Box::new(default)),
                filter: FilterType::Lower(LowerFilter),
            };

            let rendered = lower.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "bryony");
        })
    }
}
