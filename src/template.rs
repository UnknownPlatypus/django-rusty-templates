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
    use crate::render::{Context, Render};

    import_exception_bound!(django.core.exceptions, ImproperlyConfigured);
    import_exception_bound!(django.template.exceptions, TemplateDoesNotExist);
    import_exception_bound!(django.template.exceptions, TemplateSyntaxError);
    import_exception_bound!(django.urls, NoReverseMatch);

    impl TemplateSyntaxError {
        fn with_source_code(
            err: miette::Report,
            source: impl miette::SourceCode + 'static,
        ) -> PyErr {
            let miette_err = err.with_source_code(source);
            TemplateSyntaxError::new_err(format!("{miette_err:?}"))
        }
    }

    #[pyclass]
    pub struct Engine {
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
            _py: Python<'py>,
            _loader: &str,
            _args: Option<Bound<'py, PyAny>>,
        ) -> PyResult<Bound<'py, PyAny>> {
            todo!()
        }
    }

    #[pymethods]
    impl Engine {
        #[new]
        #[pyo3(signature = (dirs=None, app_dirs=false, context_processors=None, debug=false, loaders=None, string_if_invalid="".to_string(), file_charset="utf-8".to_string(), libraries=None, builtins=None, autoescape=true))]
        #[allow(clippy::too_many_arguments)] // We're matching Django's Engine __init__ signature
        pub fn new(
            _py: Python<'_>,
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
                Some(_loaders) => todo!(),
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
                match loader.get_template(py, &template_name, self.autoescape) {
                    Ok(template) => return template,
                    Err(e) => tried.push(e.tried),
                }
            }
            Err(TemplateDoesNotExist::new_err((template_name, tried)))
        }

        #[allow(clippy::wrong_self_convention)] // We're implementing a Django interface
        pub fn from_string(&self, template_code: Bound<'_, PyString>) -> PyResult<Template> {
            Template::new_from_string(template_code.extract()?, self.autoescape)
        }
    }

    #[derive(Clone, Debug)]
    #[pyclass]
    pub struct Template {
        pub filename: Option<PathBuf>,
        pub template: String,
        pub nodes: Vec<TokenTree>,
        pub autoescape: bool,
    }

    impl Template {
        pub fn new(template: &str, filename: PathBuf, autoescape: bool) -> PyResult<Self> {
            let mut parser = Parser::new(template);
            let nodes = match parser.parse() {
                Ok(nodes) => nodes,
                Err(err) => {
                    let source =
                        miette::NamedSource::new(filename.to_string_lossy(), template.to_string());
                    return Err(TemplateSyntaxError::with_source_code(err.into(), source));
                }
            };
            Ok(Self {
                template: template.to_string(),
                filename: Some(filename),
                nodes,
                autoescape,
            })
        }

        pub fn new_from_string(template: String, autoescape: bool) -> PyResult<Self> {
            let mut parser = Parser::new(&template);
            let nodes = match parser.parse() {
                Ok(nodes) => nodes,
                Err(err) => {
                    return Err(TemplateSyntaxError::with_source_code(err.into(), template));
                }
            };
            Ok(Self {
                template,
                filename: None,
                nodes,
                autoescape,
            })
        }

        fn _render(
            &self,
            py: Python<'_>,
            context: &mut Context,
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
        #[pyo3(signature = (template, autoescape=true))]
        pub fn from_string(template: Bound<'_, PyString>, autoescape: bool) -> PyResult<Self> {
            Self::new_from_string(template.extract()?, autoescape)
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
            let request = request.map(|request| request.unbind());
            let mut context = Context {request, context, autoescape: self.autoescape};
            self._render(py, &mut context)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::django_rusty_templates::*;

    use pyo3::types::{PyDict, PyDictMethods, PyString};
    use pyo3::Python;

    #[test]
    fn test_syntax_error() {
        pyo3::prepare_freethreaded_python();

        let mut filename = std::env::current_dir().unwrap();
        filename.push("tests");
        filename.push("templates");
        filename.push("parse_error.txt");

        let expected = format!(
            "TemplateSyntaxError: \n  × Empty variable tag
   ╭─[{}:1:28]
 1 │ This is an empty variable: {{{{ }}}}
   ·                            ──┬──
   ·                              ╰── here
   ╰────
",
            filename.display(),
        );

        let template_string = std::fs::read_to_string(&filename).unwrap();
        let error = temp_env::with_var("NO_COLOR", Some("1"), || {
            Template::new(&template_string, filename, true).unwrap_err()
        });

        let error_string = format!("{error}");
        assert_eq!(error_string, expected);
    }

    #[test]
    fn test_syntax_error_from_string() {
        pyo3::prepare_freethreaded_python();

        let template_string = "{{ foo.bar|title'foo' }}".to_string();
        let error = temp_env::with_var("NO_COLOR", Some("1"), || {
            Template::new_from_string(template_string, true).unwrap_err()
        });

        let expected = "TemplateSyntaxError: \n  × Could not parse the remainder
   ╭────
 1 │ {{ foo.bar|title'foo' }}
   ·                 ──┬──
   ·                   ╰── here
   ╰────
";

        let error_string = format!("{error}");
        assert_eq!(error_string, expected);
    }

    #[test]
    fn test_render_empty_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new(py, "");
            let template = Template::from_string(template_string, true).unwrap();
            let context = PyDict::new(py);

            assert_eq!(template.render(py, Some(context), None).unwrap(), "");
        })
    }

    #[test]
    fn test_render_template_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new(py, "Hello {{ user }}!");
            let template = Template::from_string(template_string, true).unwrap();
            let context = PyDict::new(py);
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
            let template_string = PyString::new(py, "Hello {{ user }}!");
            let template = Template::from_string(template_string, true).unwrap();
            let context = PyDict::new(py);

            assert_eq!(template.render(py, Some(context), None).unwrap(), "Hello !");
        })
    }

    #[test]
    fn test_render_template_variable_nested() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template_string = PyString::new(py, "Hello {{ user.profile.names.0 }}!");
            let template = Template::from_string(template_string, true).unwrap();
            let locals = PyDict::new(py);
            py.run(
                cr#"
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
            let context = PyDict::new(py);
            context.set_item("user", user.into_any()).unwrap();

            assert_eq!(
                template.render(py, Some(context), None).unwrap(),
                "Hello Lily!"
            );
        })
    }

    #[test]
    fn test_engine_from_string() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = Engine::new(
                py,
                None,
                false,
                None,
                false,
                None,
                "".to_string(),
                "utf-8".to_string(),
                None,
                None,
                false,
            )
            .unwrap();
            let template_string = PyString::new(py, "Hello {{ user }}!");
            let template = engine.from_string(template_string).unwrap();
            let context = PyDict::new(py);

            assert_eq!(template.render(py, Some(context), None).unwrap(), "Hello !");
        })
    }
}
