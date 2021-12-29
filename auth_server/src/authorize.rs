// GNU AGPL v3 License

use crate::{state_table, tokens, verify_client_id, verify_redirect_uri, VerifyError};
use futures_util::future;
use tracing::Level;
use warp::{http::StatusCode, reject::custom as reject, Filter, Rejection, Reply};

#[inline]
pub fn authorize<R: Reply + Send + Sync + 'static>(
    render: impl Fn(String) -> R + Send + Sync + Clone + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("authorize").and(warp::get()).and(
        warp::query::raw()
            .and_then(|query: String| {
                future::ready({
                    serde_urlencoded::from_str(&query).map_err(|e| reject(AuthorizeError::from(e)))
                })
            })
            .and_then(|args: AuthorizeArgs| {
                future::ready({
                    let AuthorizeArgs {
                        response_type,
                        client_id,
                        redirect_uri,
                        scope,
                        state,
                    } = args;

                    // this needs to be "code" flow
                    if response_type != "code" {
                        Err(reject(AuthorizeError::NotCodeFlow))
                    } else if let Err(e) = verify_client_id(&client_id) {
                        Err(reject(AuthorizeError::Verify(e)))
                    } else if let Err(e) = verify_redirect_uri(&redirect_uri) {
                        Err(reject(AuthorizeError::Verify(e)))
                    } else {
                        Ok((state, scope, client_id, redirect_uri))
                    }
                })
            })
            .untuple_one()
            .and(warp::any().map(|| tokens::generate_auth_token()))
            .map(
                |state: String, scope, client_id, redirect_uri, auth_token| {
                    state_table::store_entry(
                        auth_token,
                        state.clone(),
                        scope,
                        client_id,
                        redirect_uri,
                    );
                    state
                },
            )
            .map(move |state| {
                // render the login page
                render(state)
            })
            .recover(|rej: warp::Rejection| {
                future::ready({
                    match rej.find::<AuthorizeError>() {
                        Some(err) => {
                            tracing::event!(Level::ERROR, "/authorize: {}", err);
                            let (desc, code) = err.desc_and_code();
                            Ok(warp::reply::with_status(
                                warp::reply::html(format!(
                                    r#"
                                        <html>
                                        <body>
                                        <p style="color: red">
                                            {}
                                        </p>
                                        </body>
                                        </html>
                                    "#,
                                    desc
                                )),
                                code,
                            ))
                        }
                        None => Err(rej),
                    }
                })
            }),
    )
}

#[derive(serde::Deserialize)]
struct AuthorizeArgs {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
}

#[derive(Debug, thiserror::Error)]
enum AuthorizeError {
    #[error("{0}")]
    Url(#[from] serde_urlencoded::de::Error),
    #[error("Response type is not a valid code flow")]
    NotCodeFlow,
    #[error("{0}")]
    Verify(#[from] VerifyError),
}

impl AuthorizeError {
    #[inline]
    fn desc_and_code(&self) -> (&'static str, StatusCode) {
        match self {
            Self::Url(..) => (
                "Unable to parse URL encoded payload",
                StatusCode::BAD_REQUEST,
            ),
            Self::NotCodeFlow => ("Code flow was not set", StatusCode::BAD_REQUEST),
            Self::Verify(..) => (
                "Verification of flow details failed",
                StatusCode::BAD_REQUEST,
            ),
        }
    }
}

impl warp::reject::Reject for AuthorizeError {}
