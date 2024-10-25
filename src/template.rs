use pyo3::prelude::*;

#[pymodule]
mod django_rusty_templates {
    use std::collections::HashMap;

    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyString};

    use crate::parse::{Parser, TokenTree};

    #[pyclass]
    pub struct Template {
        filename: Option<String>,
        template: String,
        nodes: Vec<TokenTree>,
    }

    impl Template {
        fn from_str(template: &str) -> PyResult<Self> {
            let mut parser = Parser::new(template);
            let nodes = parser.parse().unwrap();
            Ok(Self {
                template: template.to_string(),
                filename: None,
                nodes,
            })
        }

        fn _render<'py>(
            &self,
            py: Python<'py>,
            context: &HashMap<String, Bound<'py, PyAny>>,
        ) -> PyResult<String> {
            let mut rendered = String::with_capacity(self.template.len());
            for node in &self.nodes {
                rendered.push_str(&node.render(py, &self.template, context)?)
            }
            Ok(rendered)
        }
    }

    #[pymethods]
    impl Template {
        #[staticmethod]
        pub fn from_string(template: Bound<'_, PyString>) -> PyResult<Self> {
            let template = template.extract::<&str>()?;
            Self::from_str(template)
        }

        pub fn render(&self, context: Bound<'_, PyDict>) -> PyResult<String> {
            let py = context.py();
            let context = context.extract()?;
            self._render(py, &context)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::django_rusty_templates::*;

    use pyo3::types::{PyDict, PyDictMethods, PyString};
    use pyo3::Python;

    #[test]
    fn test_render_empty_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new_bound(py, "");
            let template = Template::from_string(template_string).unwrap();
            let context = PyDict::new_bound(py);

            assert_eq!(template.render(context).unwrap(), "");
        })
    }

    #[test]
    fn test_render_template_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new_bound(py, "Hello {{ user }}!");
            let template = Template::from_string(template_string).unwrap();
            let context = PyDict::new_bound(py);
            context.set_item("user", "Lily").unwrap();

            assert_eq!(template.render(context).unwrap(), "Hello Lily!");
        })
    }

    #[test]
    fn test_render_template_unknown_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new_bound(py, "Hello {{ user }}!");
            let template = Template::from_string(template_string).unwrap();
            let context = PyDict::new_bound(py);

            assert_eq!(template.render(context).unwrap(), "Hello !");
        })
    }

    #[test]
    fn test_render_template_variable_nested() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new_bound(py, "Hello {{ user.profile.names.0 }}!");
            let template = Template::from_string(template_string).unwrap();
            let locals = PyDict::new_bound(py);
            py.run_bound(
                r#"
class User:
    def __init__(self, names):
        self.profile = {"names": names}

user = User(["Lily"])
"#,
                None,
                Some(&locals),
            )
            .unwrap();
            let user = locals.get_item("user").unwrap().unwrap();
            let context = PyDict::new_bound(py);
            context.set_item("user", user.into_any()).unwrap();

            assert_eq!(template.render(context).unwrap(), "Hello Lily!");
        })
    }
}
