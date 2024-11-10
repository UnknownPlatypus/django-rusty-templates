use pyo3::prelude::*;

#[pymodule]
pub mod django_rusty_templates {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use encoding_rs::Encoding;
    use pyo3::import_exception_bound;
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyString};

    use crate::loaders::{AppDirsLoader, CachedLoader, FileSystemLoader, Loader};
    use crate::parse::{Parser, TokenTree};

    import_exception_bound!(django.core.exceptions, ImproperlyConfigured);
    import_exception_bound!(django.template.exceptions, TemplateDoesNotExist);

    #[pyclass]
    struct Engine {
        dirs: Vec<String>,
        app_dirs: bool,
        context_processors: Vec<String>,
        debug: bool,
        string_if_invalid: String,
        encoding: &'static Encoding,
        libraries: HashMap<String, Py<PyAny>>,
        builtins: Vec<String>,
        autoescape: bool,
        template_loaders: Vec<Loader>,
    }

    impl Engine {
        fn find_template_loader<'py>(
            py: Python<'py>,
            loader: &str,
            args: Option<Bound<'py, PyAny>>,
        ) -> PyResult<Bound<'py, PyAny>> {
            todo!()
        }
    }

    #[pymethods]
    impl Engine {
        #[new]
        #[pyo3(signature = (dirs=None, app_dirs=false, context_processors=None, debug=false, loaders=None, string_if_invalid="".to_string(), file_charset="utf-8".to_string(), libraries=None, builtins=None, autoescape=true))]
        fn new(
            py: Python<'_>,
            dirs: Option<Bound<'_, PyAny>>,
            app_dirs: bool,
            context_processors: Option<Bound<'_, PyAny>>,
            debug: bool,
            loaders: Option<Bound<'_, PyAny>>,
            string_if_invalid: String,
            file_charset: String,
            libraries: Option<Bound<'_, PyAny>>,
            builtins: Option<Bound<'_, PyAny>>,
            autoescape: bool,
        ) -> PyResult<Self> {
            let dirs = match dirs {
                Some(dirs) => dirs.extract()?,
                None => Vec::new(),
            };
            let context_processors = match context_processors {
                Some(context_processors) => context_processors.extract()?,
                None => Vec::new(),
            };
            let encoding = match Encoding::for_label(file_charset.as_bytes()) {
                Some(encoding) => encoding,
                None => todo!(),
            };
            let template_loaders = match loaders {
                Some(_) if app_dirs => {
                    let err = ImproperlyConfigured::new_err(
                        "app_dirs must not be set when loaders is defined.",
                    );
                    return Err(err);
                }
                Some(loaders) => todo!(),
                None => {
                    let filesystem_loader =
                        Loader::FileSystem(FileSystemLoader::new(dirs.clone(), encoding));
                    let appdirs_loader = Loader::AppDirs(AppDirsLoader {});
                    let loaders = if app_dirs {
                        vec![filesystem_loader, appdirs_loader]
                    } else {
                        vec![filesystem_loader]
                    };
                    let cached_loader = Loader::Cached(CachedLoader::new(loaders));
                    vec![cached_loader]
                }
            };
            let libraries = HashMap::new();
            let builtins = vec![];
            Ok(Self {
                dirs,
                app_dirs,
                context_processors,
                debug,
                template_loaders,
                string_if_invalid,
                encoding,
                libraries,
                builtins,
                autoescape,
            })
        }

        fn get_template(&mut self, py: Python<'_>, template_name: String) -> PyResult<Template> {
            let mut tried = Vec::new();
            for loader in &mut self.template_loaders {
                match loader.get_template(py, &template_name) {
                    Ok(template) => return template,
                    Err(e) => tried.push(e.tried),
                }
            }
            Err(TemplateDoesNotExist::new_err((template_name, tried)))
        }

        #[allow(clippy::wrong_self_convention)] // We're implementing a Django interface
        fn from_string(&self, template_code: Bound<'_, PyString>) -> PyResult<Template> {
            Template::from_str(template_code.extract()?)
        }
    }

    #[derive(Clone, Debug)]
    #[pyclass]
    pub struct Template {
        pub filename: Option<PathBuf>,
        pub template: String,
        pub nodes: Vec<TokenTree>,
    }

    impl Template {
        pub fn new(template: &str, filename: PathBuf) -> PyResult<Self> {
            let mut parser = Parser::new(template);
            let nodes = parser.parse().unwrap();
            Ok(Self {
                template: template.to_string(),
                filename: Some(filename),
                nodes,
            })
        }

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

        #[pyo3(signature = (context=None, request=None))]
        pub fn render(
            &self,
            py: Python<'_>,
            context: Option<Bound<'_, PyDict>>,
            request: Option<Bound<'_, PyAny>>,
        ) -> PyResult<String> {
            let context = match context {
                Some(context) => context.extract()?,
                None => HashMap::new(),
            };
            if let Some(request) = request {
                todo!()
            }
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

            assert_eq!(template.render(py, Some(context), None).unwrap(), "");
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

            assert_eq!(
                template.render(py, Some(context), None).unwrap(),
                "Hello Lily!"
            );
        })
    }

    #[test]
    fn test_render_template_unknown_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new_bound(py, "Hello {{ user }}!");
            let template = Template::from_string(template_string).unwrap();
            let context = PyDict::new_bound(py);

            assert_eq!(template.render(py, Some(context), None).unwrap(), "Hello !");
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

            assert_eq!(
                template.render(py, Some(context), None).unwrap(),
                "Hello Lily!"
            );
        })
    }
}
