// GNU AGPL v3 License

mod authorize;
mod constants;
mod random;
mod token;

pub use constants::*;
pub use random::*;

use dashmap::DashSet;
use once_cell::sync::OnceCell;
use std::{convert::Infallible, io::Error as IoError, net::IpAddr};
use tokio::{fs::File, io::AsyncReadExt};
use warp::{http::StatusCode, Filter, Reply};

pub static STATES: OnceCell<DashSet<String>> = OnceCell::new();

#[tokio::main]
async fn main() {
    let key = read_file("oauth2.rsa").await.unwrap();
    let cert = read_file("oauth2.pem").await.unwrap();
    let routes = routes();

    warp::serve(routes)
        .tls()
        .key(key)
        .cert(cert)
        .run(("127.0.0.1".parse::<IpAddr>().unwrap(), 8200))
        .await;
}

#[inline]
async fn read_file(name: &str) -> Result<Vec<u8>, IoError> {
    let mut b = vec![];
    let mut file = File::open(name).await?;
    file.read_to_end(&mut b).await?;
    Ok(b)
}

fn routes(
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static {
    authorize::authorize()
        .or(token::token())
        .or(warp::any().map(|| StatusCode::NOT_FOUND))
}
