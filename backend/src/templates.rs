// GNU AGPL v3 License

use crate::{Config, Urls};
use arc_swap::ArcSwap;
use notify::Watcher;
use once_cell::sync::OnceCell;
use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};
use tera::{Context, Error, Tera};
use tracing::{event, Level};
use walkdir::WalkDir;

struct TemplateState {
    templates: ArcSwap<Tera>,
    template_path: PathBuf,
    urls: Urls,
}

#[derive(Default)]
pub struct TemplateOptions {
    pub csrf_token: Option<String>,
}

#[inline]
pub fn template<T: serde::Serialize>(
    name: &str,
    data: T,
    options: TemplateOptions,
) -> Result<String, Error> {
    let TemplateOptions { csrf_token } = options;

    // load the global state
    let templates = TEMPLATES
        .get()
        .expect("`initialize_templates` not called before `template`");

    // create a Context to store data in
    let mut context = Context::from_serialize(data)?;

    // add urls
    context.insert("auth_url", &templates.urls.auth_url);
    context.insert("api_url", &templates.urls.api_url);
    context.insert("static_url", &templates.urls.static_url);

    // add csrf token
    if let Some(csrf_token) = csrf_token {
        context.insert("csrf_token", &csrf_token);
    }

    // preform the templating
    let templates = templates.templates.load();
    templates.render(name, &context)
}

#[inline]
pub async fn initialize_templates(cfg: &Config) -> Result<(), PopulateTemplateError> {
    TEMPLATES
        .set(TemplateState {
            template_path: cfg.template_path.clone(),
            templates: ArcSwap::from_pointee(Tera::default()),
            urls: cfg.urls.clone(),
        })
        .unwrap_or_else(|_| panic!("`initialize_templates` called more than once"));

    // populate our templates
    let res = tokio::task::spawn_blocking(|| populate_templates())
        .await
        .expect("Blocking task panicked");

    spawn_template_reloader();

    res
}

/// Populate the templates of the global state with the ones in the
/// template directory.
#[inline]
fn populate_templates() -> Result<(), PopulateTemplateError> {
    let templates = TEMPLATES.get().unwrap();
    let mut tera = Tera::default();

    let template_files = find_templates(&templates.template_path)?;

    // add the files into the tera instance
    tera.add_template_files(template_files)?;

    // swap into the global state
    templates.templates.swap(Arc::new(tera));

    Ok(())
}

/// Find the templates to load.
#[inline]
fn find_templates(
    template_path: &Path,
) -> Result<Vec<(PathBuf, Option<String>)>, PopulateTemplateError> {
    let root = fs::canonicalize(template_path)?;

    WalkDir::new(&root)
        .into_iter()
        .map(|result| result.map_err(Into::into))
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    // test if this is a file; if not, ignore
                    let path = entry.path();
                    if !path.is_file() {
                        return None;
                    }

                    // is this is not a .jinja file, then ignore
                    if path.extension().and_then(OsStr::to_str) != Some("jinja") {
                        return None;
                    }

                    // convert the path to the basename
                    let basename = match path.file_prefix().and_then(OsStr::to_str) {
                        Some(basename) => basename.to_string(),
                        None => return None,
                    };

                    Some((path.to_path_buf(), Some(basename)))
                })
                .transpose()
        })
        .collect()
}

/// Spawns a thread that watches for template changes.
#[inline]
fn spawn_template_reloader() {
    let templates_path = &TEMPLATES.get().unwrap().template_path;

    // channel to send events down
    let (tx, rx) = mpsc::channel();

    // set up an event watcher with a 2-second debounce
    let mut watcher =
        notify::watcher(tx, Duration::from_secs(2)).expect("Could not create watcher");
    watcher
        .watch(templates_path, notify::RecursiveMode::Recursive)
        .expect("Could not activate watcher");

    // spawn a thread that watches for events
    thread::Builder::new()
        .name("template-watcher".into())
        .spawn(move || {
            // move watcher into this thread
            let _watcher = watcher;

            // run populate_templates() whenever a change occurs
            while rx.recv().is_ok() {
                if let Err(e) = populate_templates() {
                    event!(Level::ERROR, "failed to repopulate templates: {}", e);
                }
            }
        })
        .expect("Unable to spawn watcher thread");
}

static TEMPLATES: OnceCell<TemplateState> = OnceCell::new();

#[derive(Debug, thiserror::Error)]
pub enum PopulateTemplateError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Tera(#[from] Error),
}

impl From<walkdir::Error> for PopulateTemplateError {
    #[inline]
    fn from(i: walkdir::Error) -> PopulateTemplateError {
        PopulateTemplateError::Io(i.into())
    }
}

#[cfg(test)]
pub fn initialize_test_templates() -> Result<(), Error> {
    let templates = vec![
        ("very_basic", "Hello, {{ name }}!"),
        ("base", include_str!("../templates/base.html.jinja")),
        ("blogpost", include_str!("../templates/blogpost.html.jinja")),
    ];

    let mut tera = Tera::default();
    tera.add_raw_templates(templates)?;
    let _ = TEMPLATES.set(TemplateState {
        templates: ArcSwap::from_pointee(tera),
        template_path: PathBuf::new(),
        urls: Urls {
            static_url: "https://test.static".into(),
            api_url: "https://test.api/api".into(),
            auth_url: "https://test.auth".into(),
        },
    });

    Ok(())
}

#[cfg(test)]
mod test {
    use super::{initialize_test_templates, template};

    #[derive(serde::Serialize)]
    struct Name {
        name: &'static str,
    }

    #[derive(serde::Serialize)]
    struct Title {
        title: &'static str,
    }

    #[test]
    fn very_basic_template_test() {
        initialize_test_templates().unwrap();

        let res = template("very_basic", Name { name: "John" }, Default::default()).unwrap();
        assert_eq!(res, "Hello, John!");
    }

    #[test]
    fn base_test() {
        initialize_test_templates().unwrap();

        let res = template(
            "base",
            Title {
                title: "WorldWideWeb",
            },
            Default::default(),
        )
        .unwrap();
        assert!(res.contains("<title>WorldWideWeb</title>"));
    }
}
