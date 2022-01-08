// GNU AGPL v3 License

use super::{create_login_session, session, CreateLoginSessionError};
use crate::{
    query::{with_database, Database, DatabaseError},
    Config,
};
use dashmap::DashMap;
use futures_util::future::{err, ok, ready, TryFutureExt};
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, ExtraTokenFields,
    HttpRequest, HttpResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RequestTokenError,
    Scope, StandardRevocableToken, StandardTokenResponse, TokenResponse, TokenUrl,
};
use once_cell::sync::OnceCell;
use reqwest::{Client as ReqwestClient, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use warp::{
    http::Uri, redirect::found as redirect, reject::custom as reject, reply::with_header, Filter,
    Rejection, Reply,
};

#[inline]
pub fn initialize_oauth2(cfg: &Config) {
    OAUTH2
        .set(Oauth2 {
            client: IdClient::new(
                ClientId::new(cfg.oauth2.client_id.clone()),
                Some(ClientSecret::new(cfg.oauth2.client_secret.clone())),
                AuthUrl::new(cfg.oauth2.auth_url.clone()).unwrap(),
                Some(TokenUrl::new(cfg.oauth2.token_url.clone()).unwrap()),
            )
            .set_redirect_uri(RedirectUrl::new(cfg.oauth2.redirect_url.clone()).unwrap()),
            extant_states: DashMap::new(),
            #[cfg(not(test))]
            transport: crate::CLIENT.clone(),
        })
        .unwrap_or_else(|_| panic!("`initialize_oauth2` called more than once"));
}

static OAUTH2: OnceCell<Oauth2> = OnceCell::new();

#[inline]
pub fn login(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("login").and(warp::get()).and(
        super::with_session()
            .map(|s: Option<_>| s.is_some())
            .and_then(|s| {
                ready({
                    if s {
                        Ok(warp::redirect::found("/".parse::<Uri>().unwrap()))
                    } else {
                        Err(warp::reject())
                    }
                })
            })
            .or(warp::any().map(|| warp::redirect::found(begin_oauth2_handshake()))),
    )
}

#[inline]
pub fn callback(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("callback").and(
        warp::query::raw()
            .and_then(
                |query: String| match serde_urlencoded::from_str::<CallbackArgs>(&query) {
                    Ok(u) => ok(u),
                    Err(_) => match serde_urlencoded::from_str::<BasicErrorResponse>(&query) {
                        Ok(e) => err(reject(EndOauthError::from(e))),
                        Err(e) => err(reject(EndOauthError::from(e))),
                    },
                },
            )
            .and(with_database())
            .and_then(|ca: CallbackArgs, db: Arc<_>| {
                let CallbackArgs { state, code } = ca;
                finish_oauth2_handshake(state, code, db).map_err(reject)
            })
            .map(|access_token: String| {
                // redirect back to homepage, and set the cookie
                let uri: Uri = "/".parse().unwrap();
                let cookie = format!("access_token={}", access_token);
                with_header(redirect(uri), "Set-Cookie", cookie)
            })
            .recover(|rej: Rejection| match rej.find::<EndOauthError>() {
                Some(err) => {
                    tracing::event!(tracing::Level::ERROR, "{}", err);
                    let uri: Uri = "/".parse().unwrap();

                    ok(redirect(uri))
                }
                None => err(rej),
            }),
    )
}

#[inline]
fn begin_oauth2_handshake() -> Uri {
    let oauth2 = OAUTH2.get().expect(NO_SET);

    // create a PKCE challenge for verification
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    // generate a URL to go to
    let (auth_url, csrf_token) = oauth2
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(challenge)
        .url();

    let auth_url = auth_url.to_string().parse::<Uri>().unwrap();

    // insert csrf token into state table
    // 15 minutes should be more than enough
    let expires = Instant::now() + Duration::from_secs(15 * 60);
    let csrf_token = csrf_token.secret().clone();

    // chance of 256-bit collision is zero
    oauth2
        .extant_states
        .insert(csrf_token, ExtantState { expires, verifier });

    auth_url
}

/// Finish the Oauth2 handshake, given a state and an auth code.
///
/// Sets the login in the login table, and returns the authentication.
#[inline]
async fn finish_oauth2_handshake(
    state: String,
    code: String,
    db: Arc<impl Database>,
) -> Result<String, EndOauthError> {
    let oa = OAUTH2.get().expect(NO_SET);

    // pull the entry from the table
    let ExtantState { expires, verifier } = match oa.extant_states.remove(&state) {
        Some((_, entry)) => entry,
        None => return Err(EndOauthError::StateNotFound(state)),
    };

    // if the state has expired, act like it's not found
    let now = Instant::now();
    if now >= expires {
        return Err(EndOauthError::StateNotFound(state));
    }

    // make a request from the authorization code
    let result_tok = match oa
        .client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(verifier)
        .request_async(http_transport)
        .await
    {
        Ok(tok) => tok,
        Err(e) => {
            return Err(match e {
                RequestTokenError::Request(err) => EndOauthError::from(err),
                RequestTokenError::ServerResponse(err) => EndOauthError::from(err),
                RequestTokenError::Parse(err, _) => EndOauthError::from(err.into_inner()),
                RequestTokenError::Other(msg) => EndOauthError::Msg(msg),
            })
        }
    };

    // from this, parse the:
    //  - access token
    //  - id token
    //  - expires_in
    let access_token = result_tok.access_token().secret().clone();
    let expires_in = result_tok
        .expires_in()
        .unwrap_or(Duration::from_secs(24 * 60 * 60));
    let id_token = result_tok.extra_fields().id_token.clone();

    // set login data
    create_login_session(access_token.clone(), now + expires_in, id_token, &*db).await?;

    Ok(access_token)
}

#[inline]
async fn http_transport(request: HttpRequest) -> Result<HttpResponse, ReqwestError> {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            return Ok(tests::fake_transport(request));
        } else {
            let oa = OAUTH2.get().expect(NO_SET);

            // build the request to send
            let req_builder = oa
                .transport
                .request(request.method, request.url.as_str())
                .body(request.body);
            let req_builder = request
                .headers
                .iter()
                .fold(req_builder, |req_builder, (name, value)| {
                    req_builder.header(name.as_str(), value.as_bytes())
                });
            let req = req_builder.build()?;

            // send and expect the response
            let res = oa.transport.execute(req).await?;

            // parse into a format the `oauth2` crate understands
            let status_code = res.status();
            let headers = res.headers().to_owned();
            let chunks = res.bytes().await?;

            Ok(HttpResponse {
                status_code,
                headers,
                body: chunks.to_vec(),
            })
        }
    }
}

#[inline]
pub fn clear_expired_states() {
    let oauth = OAUTH2.get().unwrap();
    let now = Instant::now();
    oauth.extant_states.retain(|_, state| state.expires > now);
}

struct Oauth2 {
    client: IdClient,
    extant_states: DashMap<String, ExtantState>,
    #[cfg(not(test))]
    transport: ReqwestClient,
}

struct ExtantState {
    expires: Instant,
    verifier: PkceCodeVerifier,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Serialize))]
struct CallbackArgs {
    state: String,
    code: String,
}

#[derive(Deserialize)]
struct CallbackError {
    error: &'static str,
}

type IdClient = Client<
    BasicErrorResponse,
    IdTokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
>;

type IdTokenResponse = StandardTokenResponse<ExtraIdTokenField, BasicTokenType>;

#[derive(Debug, Serialize, Deserialize)]
struct ExtraIdTokenField {
    id_token: String,
}

impl ExtraTokenFields for ExtraIdTokenField {}

#[inline]
#[cfg(test)]
pub fn initialize_oauth2_test() {
    let _ = OAUTH2.set(Oauth2 {
        client: IdClient::new(
            ClientId::new("notgull1".into()),
            Some(ClientSecret::new("notgull2".into())),
            AuthUrl::new("http://test1.test/authorize".into()).unwrap(),
            Some(TokenUrl::new("http://test1.test/oauth/token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new("http://test2.test/callback".into()).unwrap()),
        extant_states: DashMap::new(),
    });
}

#[derive(Debug, thiserror::Error)]
enum EndOauthError {
    #[error("URL: {0}")]
    UrlEncode(#[from] serde_urlencoded::de::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Transport failed: {0}")]
    Reqwest(#[from] ReqwestError),
    #[error("Oauth2 error: {0}")]
    Oauth(BasicErrorResponse),
    #[error("Database: {0}")]
    Database(#[from] DatabaseError),
    #[error("JWT: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("Could not find state in table: {0}")]
    StateNotFound(String),
    #[error("{0}")]
    Msg(String),
}

impl From<BasicErrorResponse> for EndOauthError {
    #[inline]
    fn from(ber: BasicErrorResponse) -> EndOauthError {
        EndOauthError::Oauth(ber)
    }
}

impl From<CreateLoginSessionError> for EndOauthError {
    #[inline]
    fn from(clse: CreateLoginSessionError) -> EndOauthError {
        match clse {
            CreateLoginSessionError::Database(db) => Self::from(db),
            CreateLoginSessionError::Jwt(j) => Self::from(j),
        }
    }
}

impl warp::reject::Reject for EndOauthError {}

const NO_SET: &str = "`initialize_oauth2` was not called before Oauth2 functions";

#[cfg(test)]
mod tests {
    use super::{callback, login, CallbackArgs};
    use crate::auth::{initialize_auth_test, session};
    use oauth2::{HttpRequest, HttpResponse};
    use warp::{
        http::{StatusCode, Uri},
        Filter, Reply,
    };

    // fake transport function
    #[inline]
    pub fn fake_transport(req: HttpRequest) -> HttpResponse {
        let HttpRequest {
            url,
            method,
            headers,
            body,
        } = req;

        // TODO: ensure details match up

        let id_token = serde_json::json!({
            "sub": "65a7e8c5-c235-49a9-ba00-6d9c049776f4",
            "name": "notgull"
        });
        let id_token = jsonwebtoken::encode(
            &Default::default(),
            &id_token,
            &jsonwebtoken::EncodingKey::from_secret("secret".as_ref()),
        )
        .unwrap();

        let body = serde_json::json!({
            "access_token": "testing",
            "id_token": id_token,
            "refresh_token": "testing2",
            "token_type": "Bearer",
            "expires": 3599
        });

        HttpResponse {
            status_code: StatusCode::OK,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    #[tokio::test]
    async fn oauth_test() {
        initialize_auth_test();
        let routes = login().or(callback());

        // first, run /authorize
        let res = warp::test::request()
            .path("/login")
            .method("GET")
            .filter(&routes)
            .await
            .unwrap()
            .into_response();
        let location = res
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<Uri>()
            .unwrap();

        // location should tell us where to go
        let Responding {
            client_id,
            response_type,
            state,
            scope,
            redirect_uri,
        } = serde_urlencoded::from_str(location.query().unwrap()).unwrap();
        let scope = scope.split(' ').collect::<Vec<_>>();

        assert_eq!(client_id, "notgull1");
        assert_eq!(response_type, "code");
        assert_eq!(redirect_uri, "http://test2.test/callback");
        assert!(scope.contains(&"openid"));

        // now, run /oauth/token through the callback
        let data = CallbackArgs {
            state,
            code: "SomeAuthCode".into(),
        };
        let data = serde_urlencoded::to_string(data).unwrap();
        let path = format!("/callback?{}", data);

        let res = warp::test::request()
            .path(&path)
            .filter(&routes)
            .await
            .unwrap()
            .into_response();
        let session = session("testing").unwrap();
        assert_eq!(session.roles.0, 0xFFFFFFFF);
    }

    #[derive(serde::Deserialize)]
    struct Responding {
        client_id: String,
        response_type: String,
        state: String,
        scope: String,
        redirect_uri: String,
    }
}
