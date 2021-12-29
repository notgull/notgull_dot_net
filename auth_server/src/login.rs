// GNU AGPL v3 License

use crate::{
    hashing,
    query::{Database, DatabaseError},
    state_table, AuthData,
};
use futures_util::future::{self, FutureExt, TryFutureExt};
use jsonwebtoken::errors::Error as JwtError;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::sleep;
use warp::{
    http::StatusCode,
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn login(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("login").and(warp::get()).and(
        crate::query::with_database()
            .and(warp::query::raw())
            .and_then(|database, query: String| {
                future::ready({
                    match serde_urlencoded::from_str::<LoginArgs>(&query) {
                        Ok(d) => Ok((d, database)),
                        Err(e) => Err(reject(LoginError::from(e))),
                    }
                })
            })
            .untuple_one()
            .and(warp::addr::remote())
            .and_then(|u_and_p, db: Arc<_>, remote_addr: Option<SocketAddr>| {
                let LoginArgs {
                    username,
                    password,
                    state,
                    client_id,
                } = u_and_p;
                let remote_addr = match remote_addr {
                    Some(ra) => ra.to_string(),
                    None => "Unknown".to_string(),
                };

                login_internal(username, password, db, remote_addr, client_id).map(move |res| {
                    match res {
                        Ok(res) => Ok((res, state)),
                        Err(e) => Err(reject(e)),
                    }
                })
            })
            .untuple_one()
            .and_then(|auth_data, state: String| {
                future::ready({
                    match state_table::add_entry_auth_data(state.clone(), auth_data) {
                        Ok(tok) => Ok((tok, state)),
                        Err(e) => Err(reject(LoginError::from(e))),
                    }
                })
            })
            .untuple_one()
            .map(|tok, state| json(&Code { code: tok, state }))
            .recover(|rej: Rejection| {
                // sleep for 1000 ms to prevent brute force attacks
                sleep(Duration::from_millis(1000)).map(move |()| match rej.find::<LoginError>() {
                    Some(err) => {
                        tracing::event!(tracing::Level::ERROR, "{}", err);
                        let (error, code) = err.as_error();
                        Ok(with_status(json(&Error { error }), code))
                    }
                    None => Err(rej),
                })
            }),
    )
}

#[inline]
async fn login_internal(
    username: String,
    password: String,
    db: Arc<impl Database>,
    addr: String,
    client_id: String,
) -> Result<AuthData, LoginError> {
    // fetch the user
    let (user, muser) = db.fetch_user_by_username_or_email(username).await?;

    // fetch the password
    let (hash, _login_attempts) = db.fetch_user_password(muser.clone(), addr).await?;

    // verify the password
    hashing::verify_password(password.as_bytes(), &hash)?;

    // create an auth data
    let ad = AuthData::authorize(user, muser, client_id).await?;

    Ok(ad)
}

#[derive(serde::Deserialize)]
struct LoginArgs {
    username: String,
    password: String,
    state: String,
    client_id: String,
}

#[derive(Debug, thiserror::Error)]
enum LoginError {
    #[error("{0}")]
    UrlEncode(#[from] serde_urlencoded::de::Error),
    #[error("{0}")]
    Database(#[from] DatabaseError),
    #[error("{0}")]
    Hash(#[from] hashing::HashError),
    #[error("{0}")]
    Reject(#[from] state_table::RejectError),
    #[error("{0}")]
    Jwt(#[from] JwtError),
}

impl warp::reject::Reject for LoginError {}

impl LoginError {
    #[inline]
    fn as_error(&self) -> (&'static str, StatusCode) {
        match self {
            Self::UrlEncode(..) => ("Unable to parse URL-encoded data", StatusCode::BAD_REQUEST),
            Self::Database(DatabaseError::NotFound) | Self::Hash(..) => (
                "The username/password combination was not found in the database",
                StatusCode::UNAUTHORIZED,
            ),
            Self::Database(..) => ("An SQL error occurred", StatusCode::INTERNAL_SERVER_ERROR),
            Self::Reject(..) => ("State table rejected credentials", StatusCode::BAD_REQUEST),
            Self::Jwt(..) => (
                "An internal JWT error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        }
    }
}

#[derive(serde::Serialize)]
struct Error {
    error: &'static str,
}

#[derive(serde::Serialize)]
struct Code {
    code: String,
    state: String,
}

#[cfg(tests)]
mod tests {
    use super::{login, Code, Error};
    use crate::state_table;
    use std::net::{IpAddr, SocketAddr};
    use warp::{http::StatusCode, hyper::body::to_bytes, Reply};

    #[tokio::test]
    async fn login_test() {
        state_table::intiailize_state_table();
        let login = login();
        state_table::store_entry(
            "test1".into(),
            "test".into(),
            "test3".into(),
            "test4".into(),
            "test5".into(),
        );

        const PATH: &str = "/login?username=test&password=testing&client_id=test4&state=test";
        let res = warp::test::request()
            .path(PATH)
            .remote_addr(SocketAddr::new("127.0.0.1".parse().unwrap(), 65000))
            .filter(&login)
            .await
            .unwrap()
            .into_reponse();

        assert_eq!(res.status(), StatusCode::OK);

        let body = to_bytes(res.into_body()).await.unwrap();
        if body.contains(b"\"error\"") {
            let body = serde_json::from_slice::<Error>(&body).unwrap();
            panic!("/login failed: {}", body.error);
        } else {
            let body = serde_json::from_slice::<Code>(&body).unwrap();
            assert_eq!(body.code, "test1");
            assert_eq!(body.state, "test");
        }
    }
}
