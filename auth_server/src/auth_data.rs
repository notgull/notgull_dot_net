// GNU AGPL v3 License

use crate::{
    models::{ManagedUser, User},
    query::{Database, DatabaseError},
    tokens::generate_auth_token,
};
use jsonwebtoken::errors::Error as JwtError;
use std::time::{Duration, SystemTime};

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(serde::Deserialize, Debug, Clone, PartialEq, Eq))]
pub struct AuthData {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(serde::Serialize)]
struct IdToken {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    iat: u64,
    auth_time: u64,
    nickname: String,
}

impl AuthData {
    #[inline]
    pub async fn authorize(
        user: User,
        muser: ManagedUser,
        client_id: String,
    ) -> Result<AuthData, JwtError> {
        let access_token = generate_auth_token();
        let refresh_token = generate_auth_token();
        let token_type = "Bearer".into();
        let expires_in = 3599u64;

        let now_time = SystemTime::now();
        let expire_time = now_time + Duration::from_secs(expires_in);

        let now_secs = now_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_micros(0))
            .as_secs();
        let expire_secs = expire_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_micros(0))
            .as_secs();

        let id_token = IdToken {
            iss: "notgull2".into(),
            sub: user.uuid,
            aud: client_id,
            exp: expire_secs,
            iat: now_secs,
            auth_time: now_secs,
            nickname: muser.username,
        };

        let id_token = jsonwebtoken::encode(
            &Default::default(),
            &id_token,
            &jsonwebtoken::EncodingKey::from_secret("notgull3".as_ref()),
        )?;

        Ok(AuthData {
            access_token,
            refresh_token,
            id_token,
            token_type,
            expires_in,
        })
    }
}
