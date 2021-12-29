// GNU AGPL v3 License

use crate::{AUTH_CODE, CLIENT_ID, REDIRECT_URI, STATES};
use futures_util::future;
use warp::{
    http::{StatusCode, Uri},
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn authorize(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("authorize").and(warp::get()).and(
        warp::query::raw()
            .and_then(|query: String| {
                future::ready({
                    // decode args from the query
                    serde_urlencoded::from_str::<AuthArgs>(&query)
                        .map_err(|e| reject(AuthError::from(e)))
                })
            })
            .and_then(|args: AuthArgs| {
                future::ready({
                    let AuthArgs {
                        state,
                        scope,
                        response_type,
                        client_id,
                        redirect_uri,
                    } = args;

                    let res = if response_type != "code" {
                        Err(AuthError::BadResponseType)
                    } else if client_id != CLIENT_ID {
                        Err(AuthError::BadClientId)
                    } else if redirect_uri != REDIRECT_URI {
                        Err(AuthError::BadRedirectUri)
                    } else {
                        Ok(state)
                    };

                    res.map_err(reject)
                })
            })
            .map(|state: String| {
                STATES.get().unwrap().insert(state.clone());
                let query = serde_urlencoded::to_string(AuthSuccess {
                    state,
                    auth_code: AUTH_CODE,
                })
                .unwrap();
                let url = format!("{}?{}", REDIRECT_URI, query)
                    .parse::<Uri>()
                    .unwrap();
                warp::redirect::found(url)
            })
            .recover(|rej: Rejection| {
                future::ready({
                    match rej.find::<AuthError>() {
                        Some(ae) => {
                            let (code, error) = ae.as_err();
                            let error_description = ae.to_string();
                            Ok(with_status(
                                json(&AuthErr {
                                    error,
                                    error_description,
                                }),
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
struct AuthArgs {
    state: String,
    scope: String,
    response_type: String,
    client_id: String,
    redirect_uri: String,
}

#[derive(serde::Serialize)]
struct AuthErr {
    error: &'static str,
    error_description: String,
}

#[derive(serde::Serialize)]
struct AuthSuccess {
    state: String,
    auth_code: &'static str,
}

#[derive(Debug, thiserror::Error)]
enum AuthError {
    #[error("URL Encode: {0}")]
    Url(#[from] serde_urlencoded::de::Error),
    #[error("Proto: Bad reponse_type")]
    BadResponseType,
    #[error("Auth: Bad client ID")]
    BadClientId,
    #[error("Auth: Bad redirect URI")]
    BadRedirectUri,
}

impl AuthError {
    #[inline]
    fn as_err(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadResponseType => (StatusCode::BAD_REQUEST, "unsupported_response_type"),
            Self::Url(..) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::BadClientId | Self::BadRedirectUri => {
                (StatusCode::UNAUTHORIZED, "unauthorized_client")
            }
        }
    }
}

impl warp::reject::Reject for AuthError {}
