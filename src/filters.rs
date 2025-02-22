use crate::render::{Context, IntoBorrowedContent, IntoOwnedContent, Render, TemplateResult};
use crate::types::Argument;
use crate::{render::Content, types::TemplateString};
use pyo3::prelude::*;

#[derive(Debug)]
pub enum FilterType {
    Add(Argument, AddFilter),
    AddSlashes(AddSlashesFilter),
    Capfirst(CapfirstFilter),
    Default(Argument, DefaultFilter),
    External(Py<PyAny>, Option<Argument>, ExternalFilter),
    Lower(LowerFilter),
}

pub trait Applicable<'t, 'py> {
    fn apply(variable: Option<Content<'t, 'py>>, context: &mut Context) -> TemplateResult<'t, 'py>;
}

pub trait ApplicableArg<'t, 'py> {
    fn apply(
        variable: Option<Content<'t, 'py>>,
        arg: &Argument,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py>;
}

pub trait ApplicableFilter<'t, 'py> {
    fn apply(
        filter: &Py<PyAny>,
        variable: Option<Content<'t, 'py>>,
        arg: Option<&Argument>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let arg = match arg {
            Some(arg) => arg.resolve(py, template, context)?,
            None => None,
        };
        let filter = filter.bind(py);
        let value = match arg {
            Some(arg) => filter.call1((variable, arg))?,
            None => filter.call1((variable,))?,
        };
        Ok(Some(Content::Py(value)))
    }
}

#[derive(Debug)]
pub struct AddSlashesFilter;

impl<'t, 'py> Applicable<'t, 'py> for AddSlashesFilter {
    fn apply(variable: Option<Content<'t, 'py>>, context: &mut Context) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => content
                .render(context)?
                .replace(r"\", r"\\")
                .replace("\"", "\\\"")
                .replace("'", r"\'")
                .into_content(),
            None => "".into_content(),
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct AddFilter;

impl<'t, 'py> ApplicableArg<'t, 'py> for AddFilter {
    fn apply(
        variable: Option<Content<'t, 'py>>,
        arg: &Argument,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let variable = match variable {
            Some(left) => left,
            None => return Ok(None),
        };
        let right = arg
            .resolve(py, template, context)?
            .expect("missing argument in context should already have raised");
        match (variable.to_bigint(), right.to_bigint()) {
            (Some(variable), Some(right)) => return Ok(Some(Content::Int(variable + right))),
            _ => {
                let variable = variable.to_py(py);
                let right = right.to_py(py);
                match variable.add(right) {
                    Ok(sum) => return Ok(Some(Content::Py(sum))),
                    Err(_) => return Ok(None),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CapfirstFilter;

impl<'t, 'py> Applicable<'t, 'py> for CapfirstFilter {
    fn apply(variable: Option<Content<'t, 'py>>, context: &mut Context) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => {
                let content_string = content.render(context)?.into_owned();
                let mut chars = content_string.chars();
                let first_char = match chars.next() {
                    Some(c) => c.to_uppercase(),
                    None => return Ok("".into_content()),
                };
                let string: String = first_char.chain(chars).collect();
                string.into_content()
            }
            None => "".into_content(),
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct DefaultFilter;

impl<'t, 'py> ApplicableArg<'t, 'py> for DefaultFilter {
    fn apply(
        variable: Option<Content<'t, 'py>>,
        arg: &Argument,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(left) => Some(left),
            None => arg.resolve(py, template, context)?,
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct ExternalFilter;
impl<'t, 'py> ApplicableFilter<'t, 'py> for ExternalFilter {}

#[derive(Debug)]
pub struct LowerFilter;

impl<'t, 'py> Applicable<'t, 'py> for LowerFilter {
    fn apply(variable: Option<Content<'t, 'py>>, context: &mut Context) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => content.render(context)?.to_lowercase().into_content(),
            None => "".into_content(),
        };
        Ok(content)
    }
}
