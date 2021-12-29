// GNU AGPL v3 License

#[macro_use]
extern crate diesel;

mod config;

pub mod auth_data;
pub mod authorize;
pub mod database;
pub mod forms;
pub mod hashing;
pub mod login;
#[cfg(test)]
pub mod mock_database;
pub mod models;
pub mod query;
pub mod schema;
pub mod send_token;
pub mod state_table;
pub mod tokens;
pub mod verify;

pub use auth_data::AuthData;
pub use config::*;
pub use verify::*;

use std::{convert::Infallible, env, io::Error, process};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use tracing::Level;
use warp::{Filter, Reply};

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(entry());
}

#[inline]
async fn entry() {
    // load configuration
    let cfg_path = env::args_os()
        .skip(1)
        .next()
        .unwrap_or_else(|| "notgull-auth.toml".into());
    let cfg = Config::read_from_file(cfg_path).await.unwrap_or_else(|e| {
        tracing::event!(Level::ERROR, "Unable to load config: {}", e);
        process::exit(1)
    });

    verify::initialize_oauth_verify(&cfg);
    state_table::intiailize_state_table();
    forms::initialize_forms(&cfg);

    // spawn the state table clearer
    let clearer = tokio::spawn(state_table::regularly_clear_expired());

    // create a route
    let route = authorize::authorize(|_| warp::reply::html(forms::login_form()))
        .or(send_token::token())
        .or(warp::any().map(|| warp::http::StatusCode::NOT_FOUND));

    // create the server
    serve(&cfg, route).await.unwrap_or_else(|e| {
        tracing::event!(Level::ERROR, "Unable to server: {}", e);
        process::exit(1)
    });

    clearer.await.unwrap();
}

#[inline]
async fn serve(
    cfg: &Config,
    route: impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static,
) -> Result<(), Error> {
    let (mut key_data, mut cert_data) = (vec![], vec![]);
    let mut key_file = BufReader::new(File::open(&cfg.key_path).await?);
    let mut cert_file = BufReader::new(File::open(&cfg.cert_path).await?);
    key_file.read_to_end(&mut key_data).await?;
    cert_file.read_to_end(&mut cert_data).await?;

    // set up the server
    let service = warp::serve(route)
        .tls()
        .cert(cert_data)
        .key(key_data)
        .run((cfg.hostname, cfg.port));

    service.await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oauth2::{
        basic::BasicClient, AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
        CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
    };
    use warp::{hyper::body::to_bytes, Reply};

    #[tokio::test]
    async fn oauth2() {
        crate::state_table::intiailize_state_table();
        crate::verify::initialize_verify_test();

        let auth_data = AuthData {
            access_token: "test1".into(),
            id_token: "test2".into(),
            refresh_token: "test3".into(),
            token_type: "Bearer".into(),
            expires_in: 3599,
        };
        let auth_data2 = auth_data.clone();

        let routes = authorize::authorize(move |state| {
            let at = state_table::add_entry_auth_data(state.clone(), auth_data2.clone()).unwrap();
            at
        })
        .or(send_token::token());

        // spin up an oauth2 client
        let client = BasicClient::new(
            ClientId::new("test1".into()),
            Some(ClientSecret::new("test2".into())),
            AuthUrl::new("http://test/authorize".into()).unwrap(),
            Some(TokenUrl::new("http://test/oauth/token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new("https://test3.test/callback".into()).unwrap());

        let (aurl, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".into()))
            .url();
        let path = format!("{}?{}", aurl.path(), aurl.query().unwrap_or(""));

        // browse to that URL
        let auth_token = warp::test::request()
            .path(&path)
            .method("GET")
            .filter(&routes)
            .await
            .unwrap()
            .into_response();
        let auth_token = to_bytes(auth_token).await.unwrap();
        let auth_token: String = std::str::from_utf8(&auth_token).unwrap().into();

        if auth_token.contains("<p") {
            let reason = auth_token.split("\n").nth(4).unwrap();
            panic!("/authorize failed: {}", reason);
        }

        // preform the final oauth2 handshake
        let result = client
            .exchange_code(AuthorizationCode::new(auth_token))
            .request_async(|h| async move {
                //panic!("{}", std::str::from_utf8(&h.body).unwrap());
                let rb = warp::test::request()
                    .path(h.url.path())
                    .method(h.method.as_str())
                    .body(&h.body);
                let rb = h
                    .headers
                    .iter()
                    .fold(rb, |rb, (key, value)| rb.header(key, value));
                let mut res = rb.filter(&routes).await.unwrap().into_response();
                Result::<_, std::io::Error>::Ok(oauth2::HttpResponse {
                    status_code: res.status(),
                    headers: std::mem::take(res.headers_mut()),
                    body: to_bytes(res.into_body()).await.unwrap().to_vec(),
                })
            })
            .await
            .unwrap();

        assert_eq!(result.access_token().secret(), "test1");
    }
}
