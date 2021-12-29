// GNU AGPL v3 License

use crate::{
    random_token, AUD, AUTH_CODE, CLIENT_ID, CLIENT_SECRET, ISS, NAME, REDIRECT_URI, STATES, SUB,
};
use bytes::Bytes;
use data_encoding::BASE64;
use futures_util::future::{err, ok, ready, TryFutureExt};
use std::time::{Duration, SystemTime};
use warp::{
    http::StatusCode,
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn token(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("oauth" / "token").and(warp::post()).and(
        warp::body::bytes()
            .and_then(|data: Bytes| {
                ready({
                    match serde_json::from_slice::<TokenArgs>(&data) {
                        Ok(ta) => Ok(ta),
                        Err(_) => serde_urlencoded::from_bytes::<TokenArgs>(&data)
                            .map_err(|e| reject(TokenError::from(e))),
                    }
                })
            })
            .and(warp::header::optional::<String>("authorization"))
            .and_then(|args: TokenArgs, auth: Option<String>| {
                let TokenArgs {
                    grant_type,
                    redirect_uri,
                    code,
                    client_id,
                    client_secret,
                } = args;

                let (client_id, client_secret) = match (client_id, client_secret, auth) {
                    (Some(c), Some(s), _) => (c, s),
                    (_, _, Some(auth)) => match parse_authorization(&auth) {
                        Ok(cs) => cs,
                        Err(e) => return err(reject(e)),
                    },
                    _ => return err(reject(TokenError::NoAuth)),
                };

                if grant_type != "authorization_code" {
                    err(reject(TokenError::NotAuthGrant))
                } else if client_id != CLIENT_ID {
                    err(reject(TokenError::BadClientId))
                } else if client_secret != CLIENT_SECRET {
                    err(reject(TokenError::BadClientSecret))
                } else if redirect_uri != REDIRECT_URI {
                    err(reject(TokenError::BadRedirectUri))
                } else if code != AUTH_CODE {
                    err(reject(TokenError::CodeNotFound))
                } else {
                    ok(())
                }
            })
            .untuple_one()
            .and_then(|| ready(token_result().map_err(reject)))
            .map(|res| json(&res))
            .recover(|rej: Rejection| match rej.find::<TokenError>() {
                Some(te) => {
                    let (code, error) = te.as_code_and_desc();
                    let error_description = te.to_string();
                    ok(with_status(
                        json(&TokenErr {
                            error,
                            error_description,
                        }),
                        code,
                    ))
                }
                None => err(rej),
            }),
    )
}

#[inline]
fn token_result() -> Result<TokenResult, TokenError> {
    let access_token = random_token();
    let refresh_token = random_token();

    let now = SystemTime::now();
    let expires = now + Duration::from_secs(60 * 60);
    let (now_secs, expires_secs) = (
        time_to_secs_since_epoch(now),
        time_to_secs_since_epoch(expires),
    );

    let id_token = IdToken {
        iss: ISS,
        sub: SUB,
        aud: AUD,
        exp: expires_secs,
        iat: now_secs,
        nickname: NAME.to_string(),
    };

    let id_token = jsonwebtoken::encode(
        &Default::default(),
        &id_token,
        &jsonwebtoken::EncodingKey::from_secret("notgull".as_ref()),
    )?;

    Ok(TokenResult {
        access_token,
        refresh_token,
        id_token,
        token_type: "Bearer",
    })
}

#[inline]
fn parse_authorization(authorization: &str) -> Result<(String, String), TokenError> {
    use TokenError::BadAuthHeader as Parse;

    let (basic, data) = authorization.as_bytes().split_at(6);
    if basic != b"Basic " {
        return Err(Parse);
    }

    // data is base64 encoded, split by a colon
    let data = BASE64.decode(data)?;
    let data = std::str::from_utf8(&data).map_err(|_| Parse)?;
    let mut i = data.split(":");
    let client_id = i.next().ok_or(Parse)?.to_string();
    let client_secret = i.next().ok_or(Parse)?.to_string();

    Ok((client_id, client_secret))
}

#[inline]
fn time_to_secs_since_epoch(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

#[derive(serde::Deserialize)]
struct TokenArgs {
    grant_type: String,
    redirect_uri: String,
    code: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(serde::Serialize)]
struct TokenResult {
    access_token: String,
    refresh_token: String,
    id_token: String,
    token_type: &'static str,
}

#[derive(serde::Serialize)]
struct IdToken {
    iss: &'static str,
    sub: &'static str,
    aud: &'static str,
    exp: u64,
    iat: u64,
    nickname: String,
}

#[derive(serde::Serialize)]
struct TokenErr {
    error: &'static str,
    error_description: String,
}

#[derive(Debug, thiserror::Error)]
enum TokenError {
    #[error("Url encode/json: {0}")]
    UrlEncode(#[from] serde_urlencoded::de::Error),
    #[error("Proto: not authorization grant")]
    NotAuthGrant,
    #[error("Proto: bad client ID")]
    BadClientId,
    #[error("Proto: bad client secret")]
    BadClientSecret,
    #[error("Proto: bad redirect uri")]
    BadRedirectUri,
    #[error("Proto: code not found")]
    CodeNotFound,
    #[error("JWT: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("Decode : {0}")]
    Decode(#[from] data_encoding::DecodeError),
    #[error("Auth: unable to parse auth header")]
    BadAuthHeader,
    #[error("Auth: No auth found")]
    NoAuth,
}

impl warp::reject::Reject for TokenError {}

impl TokenError {
    #[inline]
    fn as_code_and_desc(&self) -> (StatusCode, &'static str) {
        (StatusCode::BAD_REQUEST, "invalid_request")
    }
}
