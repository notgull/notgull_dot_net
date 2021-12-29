// GNU AGPL v3 License

use crate::{Config, OauthVerify};
use once_cell::sync::OnceCell;

#[inline]
pub fn initialize_oauth_verify(cfg: &Config) {
    let _ = OAUTH_VERIFY.set(cfg.verify.clone());
}

#[inline]
pub fn verify_client_id(ci: &str) -> Result<(), VerifyError> {
    if OAUTH_VERIFY.get().unwrap().client_id == ci {
        Ok(())
    } else {
        Err(VerifyError::InvalidClientId)
    }
}

#[inline]
pub fn verify_redirect_uri(ru: &str) -> Result<(), VerifyError> {
    if OAUTH_VERIFY.get().unwrap().redirect_uri == ru {
        Ok(())
    } else {
        Err(VerifyError::InvalidRedirectUri)
    }
}

#[inline]
pub fn verify_client_secret(_ci: &str, cs: &str) -> Result<(), VerifyError> {
    if OAUTH_VERIFY.get().unwrap().client_secret == cs {
        Ok(())
    } else {
        Err(VerifyError::InvalidClientSecret)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum VerifyError {
    #[error("Client ID is invalid")]
    InvalidClientId,
    #[error("Redirect URI is invalid")]
    InvalidRedirectUri,
    #[error("Client secret is invalid")]
    InvalidClientSecret,
}

static OAUTH_VERIFY: OnceCell<OauthVerify> = OnceCell::new();

#[cfg(test)]
#[inline]
pub fn initialize_verify_test() {
    let _ = OAUTH_VERIFY.set(OauthVerify {
        client_id: "test1".into(),
        client_secret: "test2".into(),
        redirect_uri: "https://test3.test/callback".into(),
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn verify_sanity() {
        initialize_verify_test();
        verify_client_id("test1").unwrap();
        verify_client_secret("test1", "test2").unwrap();
        verify_redirect_uri("https://test3.test/callback").unwrap();
    }
}
