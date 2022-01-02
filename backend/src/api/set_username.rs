// GNU AGPL v3 License

use crate::{
    auth::{self, with_session, Session},
    csrf_integration::{self, CsrfError},
    models::UserChange,
    query::{with_database, Database, DatabaseError},
};
use bytes::Bytes;
use dashmap::mapref::one::Ref;
use futures_util::future::{err, ok, ready};
use std::sync::Arc;
use warp::{
    http::StatusCode,
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn set_username(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("username").and(warp::post()).and(
        csrf_integration::check_csrf::<SetUsernameError>()
            .and_then(|bytes: Bytes| {
                ready({
                    serde_json::from_slice::<Username>(&bytes)
                        .map_err(|e| reject(SetUsernameError::from(e)))
                })
            })
            .and(
                with_session()
                    .and_then(|s: Option<Ref<'static, String, Session>>| match s {
                        None => err(reject(SetUsernameError::NoSession)),
                        Some(s) => ok((s.id, s.access_token.clone())),
                    })
                    .untuple_one(),
            )
            .and(with_database())
            .and_then(
                |Username { username }: Username, id: i32, at: String, db: Arc<_>| async move {
                    match update_username(&*db, id, username.clone()).await {
                        Ok(()) => {
                            auth::set_session_name(&at, username);
                            Ok(())
                        }
                        Err(e) => Err(reject(SetUsernameError::from(e))),
                    }
                },
            )
            .untuple_one()
            .map(|| json(&Id { id: 0 }))
            .recover(|rej: Rejection| match rej.find::<SetUsernameError>() {
                Some(sue) => {
                    tracing::event!(tracing::Level::ERROR, "{}", sue);
                    let (error, code) = sue.as_err();
                    ok(with_status(json(&Err { error }), code))
                }
                None => err(rej),
            }),
    )
}

#[inline]
async fn update_username(
    db: &impl Database,
    id: i32,
    username: String,
) -> Result<(), DatabaseError> {
    db.update_user(
        id,
        UserChange {
            name: Some(username),
            ..Default::default()
        },
    )
    .await
}

#[derive(serde::Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
struct Username {
    username: String,
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct Id {
    id: i32,
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct Err<'a> {
    error: &'a str,
}

#[derive(Debug, thiserror::Error)]
enum SetUsernameError {
    #[error("{0}")]
    Csrf(#[from] CsrfError),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Database(#[from] DatabaseError),
    #[error("no session?")]
    NoSession,
}

impl warp::reject::Reject for SetUsernameError {}

impl SetUsernameError {
    #[inline]
    fn as_err(&self) -> (&'static str, StatusCode) {
        match self {
            Self::Csrf(..) => ("CSRF verification failed", StatusCode::BAD_REQUEST),
            Self::Json(..) => ("JSON deserialization failed", StatusCode::BAD_REQUEST),
            Self::Database(DatabaseError::NotFound) => {
                ("Unable to find user", StatusCode::NOT_FOUND)
            }
            Self::Database(..) => (
                "Unknown database error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Self::NoSession => (
                "You were logged out during username submission",
                StatusCode::UNAUTHORIZED,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{set_username, Err, Id};
    use crate::{
        auth,
        csrf_integration::{self, EncryptedCsrfPair},
    };
    use warp::{http::StatusCode, hyper::body::to_bytes, Reply};

    #[tokio::test]
    async fn set_username_test() {
        csrf_integration::initialize_csrf_test();
        auth::initialize_auth_test();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let route = set_username();

        let body = format!(
            r#"{{"username":"Spawn Spencer","csrf_token":"{}","csrf_cookie":"{}"}}"#,
            token, cookie
        );
        let cookie = format!("access_token={}", auth::fake_access_token());

        let res = warp::test::request()
            .path("/username")
            .method("POST")
            .header("Cookie", cookie)
            .body(body)
            .filter(&route)
            .await
            .unwrap()
            .into_response();

        assert_eq!(res.status(), StatusCode::OK);

        let value = String::from_utf8(to_bytes(res.into_body()).await.unwrap().to_vec()).unwrap();
        if value.contains("error") {
            let Err { error } = serde_json::from_str(&value).unwrap();
            panic!("Route had error: {}", error);
        }

        let Id { id } = serde_json::from_str(&value).unwrap();
        assert_eq!(id, 0);
    }
}
