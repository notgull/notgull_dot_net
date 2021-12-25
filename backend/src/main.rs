// GNU AGPL v3 License

#![feature(path_file_prefix)]

#[macro_use]
extern crate diesel;

mod config;
mod query;
mod routes;
mod serve;

pub mod api;
pub mod blog;
pub mod database;
pub mod markdown;
pub mod models;
pub mod schema;
pub mod templates;

#[cfg(test)]
pub mod mock_database;

pub use config::*;
pub use query::*;

use std::{env, ffi::OsString, process};

fn main() {
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
        .skip(1)
        .next()
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

    markdown::initialize_markdown();

    // load the database
    if let Err(e) = database::initialize_database() {
        eprintln!("Unable to connect to database: {}", e);
        process::exit(1)
    }

    // load the routes to use
    let routes = routes::routes(&cfg);

    // serve them
    if let Err(e) = serve::serve(routes, &cfg).await {
        eprintln!("Unable to launch server: {}", e);
        process::exit(1);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PageRenderError {
    #[error("{0}")]
    Tera(#[from] tera::Error),
    #[error("{0}")]
    Database(#[from] DatabaseError),
}

impl warp::reject::Reject for PageRenderError {}
