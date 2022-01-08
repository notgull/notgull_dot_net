// GNU AGPL v3 License

#![feature(path_file_prefix)]
#![warn(clippy::pedantic)]

#[macro_use]
extern crate diesel;

mod config;
mod http_client;
mod query;
mod routes;
mod serve;

pub mod admin;
pub mod api;
pub mod auth;
pub mod blog;
#[path = "csrf.rs"]
pub mod csrf_integration;
pub mod database;
pub mod error_page;
pub mod frontpages;
pub mod markdown;
pub mod models;
pub mod pagerender;
pub mod schema;
pub mod templates;

#[cfg(test)]
pub mod mock_database;

pub use config::*;
pub use http_client::CLIENT;
pub use query::*;

use std::{env, ffi::OsString, io, process};

fn main() {
    env_logger::init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(entry());
}

#[inline]
async fn entry() {
    // divine the configuration path
    let cfg_path = env::args_os()
        .nth(1)
        .unwrap_or_else(|| OsString::from("notgull.toml"));

    // load the config from the file
    let cfg = Config::load_from_file(cfg_path).await.unwrap_or_else(|e| {
        eprintln!("Unable to load configuration: {}", e);
        process::exit(1)
    });

    if let Err(e) = templates::initialize_templates(&cfg).await {
        eprintln!("Unable to initialize templates: {}", e);
        process::exit(10)
    }

    api::initialize_api(&cfg).await;
    markdown::initialize_markdown();
    csrf_integration::initialize_csrf(&cfg);
    auth::initialize_auth(&cfg);

    // load the database
    if let Err(e) = database::initialize_database() {
        eprintln!("Unable to connect to database: {}", e);
        process::exit(1)
    }

    // load the routes to use
    let routes = routes::routes(&cfg);

    let task = tokio::spawn(auth::clear_auth_task());

    // serve them
    if let Err(e) = serve::serve(routes, &cfg).await {
        eprintln!("Unable to launch server: {}", e);
        process::exit(1);
    }

    task.await.expect("Auth clearing task failed");
}

#[derive(Debug, thiserror::Error)]
pub enum PageRenderError {
    #[error("{0}")]
    Tera(#[from] tera::Error),
    #[error("{0}")]
    Database(#[from] DatabaseError),
    #[error("{0}")]
    Csrf(#[from] csrf_integration::CsrfError),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("Permission denied")]
    PermissionDenied,
}

impl warp::reject::Reject for PageRenderError {}

#[derive(serde::Serialize)]
pub struct Title<'a> {
    title: &'a str,
}
