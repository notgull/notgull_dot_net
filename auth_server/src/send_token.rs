// GNU AGPL v3 License

use crate::{state_table, verify_client_secret, AuthData, VerifyError};
use bytes::Bytes;
use data_encoding::BASE64;
use futures_util::future;
use tracing::Level;
use warp::{
    http::{header, StatusCode},
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn token(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("oauth" / "token").and(warp::post()).and(
        warp::body::bytes()
            .and(warp::header::optional(header::AUTHORIZATION.as_str()))
            .and_then(|data: Bytes, authorization: Option<String>| {
                future::ready({
                    let res = match serde_json::from_slice::<TokenArgs>(&data) {
                        Ok(o) => Ok(o),
                        Err(_) => serde_urlencoded::from_bytes::<TokenArgs>(&data)
                            .map_err(|e| reject(TokenError::from(e))),
                    };
                    res.map(move |o| (o, authorization))
                })
            })
            .untuple_one()
            .and_then(|args: TokenArgs, authorization: Option<String>| {
                future::ready({
                    let TokenArgs {
                        grant_type,
                        client_id,
                        client_secret,
                        code,
                        redirect_uri,
                    } = args;

                    let cics = match (client_id, client_secret, authorization) {
                        (Some(ci), Some(cs), _) => Ok((ci, cs)),
                        (_, _, Some(authorization)) => decode_authorization(&authorization),
                        _ => Err(TokenError::AuthorizationNotFound),
                    };

                    let res: Result<AuthData, TokenError> =
                        cics.and_then(|(client_id, client_secret)| {
                            if grant_type != "authorization_code" {
                                Err(TokenError::NotAuthorizationCode)
                            } else if let Err(e) = verify_client_secret(&client_id, &client_secret)
                            {
                                Err(e.into())
                            } else {
                                state_table::check_entry(code, client_id, redirect_uri)
                                    .map_err(|e| e.into())
                            }
                        });

                    res.map_err(reject)
                })
            })
            .map(|a: AuthData| json(&a))
            .recover(|rej: Rejection| {
                future::ready({
                    match rej.find::<TokenError>() {
                        Some(tok) => {
                            tracing::event!(Level::ERROR, "/oauth/token: {}", tok);
                            let (err, code) = tok.as_error();
                            Ok(with_status(json(&err), code))
                        }
                        None => Err(rej),
                    }
                })
            }),
    )
}

#[inline]
fn decode_authorization(authorization: &str) -> Result<(String, String), TokenError> {
    use TokenError::CouldNotParseAuth as Parse;

    let authbytes = authorization.as_bytes();
    let (former, data) = authbytes.split_at(6);
    if former != b"Basic " {
        return Err(Parse);
    }

    // decode the base64
    let client_and_secret = BASE64.decode(&data)?;
    let client_and_secret = String::from_utf8(client_and_secret).map_err(|_| Parse)?;
    let mut i = client_and_secret.split(':');
    let client_id = i.next().ok_or(Parse)?.to_string();
    let client_secret = i.next().ok_or(Parse)?.to_string();

    Ok((client_id, client_secret))
}

#[derive(serde::Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
struct TokenArgs {
    grant_type: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    code: String,
    redirect_uri: String,
}

#[derive(serde::Serialize)]
struct OauthError {
    error: &'static str,
    error_description: &'static str,
}

#[derive(thiserror::Error, Debug)]
enum TokenError {
    #[error("{0}")]
    UrlEncode(#[from] serde_urlencoded::de::Error),
    #[error("Authorization not found in body or in headers")]
    AuthorizationNotFound,
    #[error("Not authorization code")]
    NotAuthorizationCode,
    #[error("Could not parse auth header")]
    CouldNotParseAuth,
    #[error("{0}")]
    Verify(#[from] VerifyError),
    #[error("{0}")]
    Reject(#[from] state_table::RejectError),
    #[error("{0}")]
    Decode(#[from] data_encoding::DecodeError),
}

impl TokenError {
    #[inline]
    fn as_error(&self) -> (OauthError, StatusCode) {
        match self {
            Self::AuthorizationNotFound => (
                OauthError {
                    error: "invalid_request",
                    error_description: "AUTHORIZATION was not recovered",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::CouldNotParseAuth => (
                OauthError {
                    error: "invalid_request",
                    error_description: "Could not parse AUTHORIZATION header",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::UrlEncode(..) => (
                OauthError {
                    error: "invalid_request",
                    error_description: "Unable to parse URL parameters",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::NotAuthorizationCode => (
                OauthError {
                    error: "unsupported_grant_type",
                    error_description: "We only support authorization code grant",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::Verify(VerifyError::InvalidClientId | VerifyError::InvalidClientSecret) => (
                OauthError {
                    error: "invalid_client",
                    error_description: "Client verification failed",
                },
                StatusCode::UNAUTHORIZED,
            ),
            Self::Verify(..) => (
                OauthError {
                    error: "invalid_request",
                    error_description: "Unable to verify request details",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::Reject(..) => (
                OauthError {
                    error: "invalid_request",
                    error_description: "State table rejected your request",
                },
                StatusCode::BAD_REQUEST,
            ),
            Self::Decode(..) => (
                OauthError {
                    error: "invalid_request",
                    error_description: "Unable to decode Base64",
                },
                StatusCode::BAD_REQUEST,
            ),
        }
    }
}

impl warp::reject::Reject for TokenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthData;
    use warp::Reply;

    #[tokio::test]
    async fn test_route() {
        crate::state_table::intiailize_state_table();
        crate::verify::initialize_verify_test();
        crate::state_table::store_entry(
            "test10".into(),
            "test11".into(),
            "test12".into(),
            "test13".into(),
            "http://test14".into(),
        );

        let args = TokenArgs {
            grant_type: "authorization_code".into(),
            client_id: Some("test13".into()),
            client_secret: Some("test2".into()),
            code: "test10".into(),
            redirect_uri: "http://test14".into(),
        };
        let auth_data = AuthData {
            access_token: "foo".into(),
            refresh_token: "bar".into(),
            id_token: "baz".into(),
            token_type: "Bearer".into(),
            expires_in: 3599,
        };

        crate::state_table::add_entry_auth_data("test11".into(), auth_data.clone()).unwrap();

        let body = serde_urlencoded::to_string(args).unwrap();

        let filter = token();

        let res = warp::test::request()
            .path("/oauth/token")
            .body(body)
            .method("POST")
            .filter(&filter)
            .await
            .unwrap()
            .into_response();
        let res = warp::hyper::body::to_bytes(res.into_body()).await.unwrap();
        let res: AuthData = serde_json::from_slice(&res).unwrap();

        assert_eq!(res, auth_data);
    }
}
