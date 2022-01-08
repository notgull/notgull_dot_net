// GNU AGPL v3 License

mod oauth;
mod username_form;

pub use oauth::{callback, login};
pub use username_form::username_form;

use crate::{
    models::User,
    query::{Database, DatabaseError},
    Config,
};
use dashmap::{mapref::one::Ref, DashMap};
use oauth::initialize_oauth2;
use once_cell::sync::OnceCell;
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};
use tokio::time::interval;
use warp::{Filter, Rejection, Reply};

#[inline]
pub fn with_session(
) -> impl Filter<Extract = (Option<Ref<'static, String, Session>>,), Error = Infallible>
       + Clone
       + Send
       + Sync
       + 'static {
    warp::cookie::optional::<String>("access_token")
        .map(|access_token: Option<String>| access_token.as_deref().and_then(session))
}

#[inline]
pub fn initialize_auth(cfg: &Config) {
    initialize_oauth2(cfg);
    let _ = LOGIN_TABLE.set(DashMap::new());
}

#[cfg(test)]
const FAKE_SESSION_ACCESS_TOKEN: &str = "fakeAccessToken";
#[cfg(test)]
const FAKE_SESSION_FEWER_PERMS: &str = "fewerPerms";

#[inline]
#[cfg(test)]
pub fn initialize_auth_test() {
    oauth::initialize_oauth2_test();
    let _ = LOGIN_TABLE.set(DashMap::new());

    // insert a fake session
    LOGIN_TABLE.get().unwrap().insert(
        FAKE_SESSION_ACCESS_TOKEN.into(),
        Session {
            name: Some("John Notgull".into()),
            roles: Permissions(0xFFFFFFFF),
            id: 1,
            access_token: FAKE_SESSION_ACCESS_TOKEN.into(),
            expires: Instant::now() + Duration::from_secs(60 * 60 * 24 * 365),
        },
    );

    // insert a fake session, with fewer permissions
    LOGIN_TABLE.get().unwrap().insert(
        FAKE_SESSION_FEWER_PERMS.into(),
        Session {
            name: Some("Brad Bradley".into()),
            roles: Permissions(0x0),
            id: 2,
            access_token: FAKE_SESSION_FEWER_PERMS.into(),
            expires: Instant::now() + Duration::from_secs(60 * 60 * 24 * 365),
        },
    );
}

#[inline]
#[cfg(test)]
pub fn fake_access_token() -> &'static str {
    FAKE_SESSION_ACCESS_TOKEN
}

#[inline]
#[cfg(test)]
pub fn fake_access_token_fewer_perms() -> &'static str {
    FAKE_SESSION_FEWER_PERMS
}

#[inline]
fn clear_expired_auth() {
    oauth::clear_expired_states();
    let now = Instant::now();
    LOGIN_TABLE
        .get()
        .unwrap()
        .retain(|_, session| session.expires > now);
}

#[inline]
pub async fn clear_auth_task() {
    let mut i = interval(Duration::from_secs(60 * 60 * 4));
    loop {
        i.tick().await;
        clear_expired_auth();
    }
}

/// Create a new session in the login table.
#[inline]
pub async fn create_login_session(
    access_token: String,
    expires: Instant,
    id_token: String,
    db: &impl Database,
) -> Result<(), CreateLoginSessionError> {
    let IdToken { sub } = jsonwebtoken::dangerous_insecure_decode(&id_token)?.claims;
    let User {
        roles, name, id, ..
    } = db.get_user_by_uuid(sub).await?;
    let login_table = LOGIN_TABLE.get().expect(NO_SET);

    // insert the session
    login_table.insert(
        access_token.clone(),
        Session {
            name,
            roles: Permissions(roles),
            id,
            access_token,
            expires,
        },
    );

    Ok(())
}

/// Get a login session from the table.
#[inline]
pub fn session(access: &str) -> Option<Ref<'static, String, Session>> {
    let s = LOGIN_TABLE.get().expect(NO_SET).get(access);
    if let Some(s) = s.as_ref() {
        tracing::debug!("Found user: {:?}", s.name);
    } else {
        tracing::debug!("No user found");
    }
    s
}

/// Set the name for a session, used for set_username().
#[inline]
pub fn set_session_name(access: &str, name: String) {
    if let Some(mut s) = LOGIN_TABLE.get().expect(NO_SET).get_mut(access) {
        s.name = Some(name);
    }
}

static LOGIN_TABLE: OnceCell<DashMap<String, Session>> = OnceCell::new();

#[derive(Debug)]
pub struct Session {
    pub name: Option<String>,
    pub roles: Permissions,
    pub id: i32,
    pub access_token: String,
    expires: Instant,
}

#[derive(Debug, Copy, Clone, Default, serde::Serialize)]
#[serde(transparent)]
pub struct Permissions(pub i64);

impl Permissions {
    #[inline]
    pub fn applies_to(self, user_roles: Permissions) -> bool {
        tracing::debug!(
            "Comparing roles: user is {:b}, password is {:b}",
            user_roles.0,
            self.0
        );
        self.0 & user_roles.0 == self.0
    }
}

impl From<i64> for Permissions {
    #[inline]
    fn from(i: i64) -> Permissions {
        Permissions(i)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateLoginSessionError {
    #[error("{0}")]
    Database(#[from] DatabaseError),
    #[error("{0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

/// The ID token details we care about.
#[derive(serde::Deserialize)]
struct IdToken {
    sub: String,
}

const NO_SET: &str = "`initialize_auth` not called before auth functions";

#[cfg(test)]
mod tests {
    use super::Permissions;

    #[test]
    fn sanity_perm_matches() {
        let perm_req = Permissions(0b101);
        let perm_user = Permissions(0b11101);
        assert!(perm_req.applies_to(perm_user));
    }
}
