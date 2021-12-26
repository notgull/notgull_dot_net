// GNU AGPL v3 License

use crate::Config;
use bytes::Bytes;
use csrf::{AesGcmCsrfProtection, CsrfProtection};
use data_encoding::BASE64;
use once_cell::sync::OnceCell;
use std::convert::TryInto;

/// Initialize CSRF operations for the server.
#[inline]
pub fn initialize_csrf(cfg: &Config) {
    CSRF_KEY
        .set(
            cfg.csrf_key
                .as_bytes()
                .get(0..32)
                .expect("CSRF key in config is less than 32 bytes")
                .try_into()
                .unwrap(),
        )
        .expect("`initialize_csrf` already called");
}

pub struct EncryptedCsrfPair {
    pub token: String,
    pub cookie: String,
}

pub struct Base64CsrfPair {
    pub token: String,
    pub cookie: String,
}

/// Generate a CSRF pair.
#[inline]
pub fn generate_csrf_pair() -> Result<EncryptedCsrfPair, CsrfError> {
    let protect = AesGcmCsrfProtection::from_key(*CSRF_KEY.get().expect(NOT_INIT));

    let (token, cookie) = protect.generate_token_pair(None, 60 * 60 * 1)?;

    Ok(EncryptedCsrfPair {
        token: token.b64_string(),
        cookie: cookie.b64_string(),
    })
}

/// Verify the CSRF from an input and a cookie.
#[inline]
pub fn decode_and_verify_csrf(data: Bytes, cookie: String) -> Result<Bytes, CsrfError> {
    let TokenDeser { csrf_token } = match serde_urlencoded::from_bytes(&data) {
        Ok(d) => d,
        Err(_) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(e) => return Err(CsrfError::from(e)),
        },
    };
    verify_csrf_pair(Base64CsrfPair {
        token: csrf_token,
        cookie,
    })?;
    Ok(data)
}

/// Verify a CSRF pair.
#[inline]
pub fn verify_csrf_pair(pair: Base64CsrfPair) -> Result<(), CsrfError> {
    let protect = AesGcmCsrfProtection::from_key(*CSRF_KEY.get().expect(NOT_INIT));

    // decode from base 64
    let Base64CsrfPair { token, cookie } = pair;
    let token = BASE64.decode(token.as_bytes())?;
    let cookie = BASE64.decode(cookie.as_bytes())?;

    // parse them through AES
    let token = protect.parse_token(&token)?;
    let cookie = protect.parse_cookie(&cookie)?;

    // verify them
    if !protect.verify_token_pair(&token, &cookie) {
        Err(CsrfError::VerificationFailed)
    } else {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CsrfError {
    #[error("{0}")]
    Internal(#[from] csrf::CsrfError),
    #[error("Failed to verify CSRF token pair")]
    VerificationFailed,
    #[error("Base64 decode failed: {0}")]
    Decode(#[from] data_encoding::DecodeError),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("Unable to find CSRF cookie")]
    CookieNotFound,
}

#[derive(serde::Deserialize)]
struct TokenDeser {
    csrf_token: String,
}

// The key used for AES CSRF operations.
static CSRF_KEY: OnceCell<[u8; 32]> = OnceCell::new();

const NOT_INIT: &str = "`initialize_csrf` was not called before using CSRF utilities";

#[cfg(test)]
#[inline]
pub fn initialize_csrf_test() {
    let _ = CSRF_KEY.set(*b"testtesttesttesttesttesttesttest");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_sanity() {
        initialize_csrf_test();
        let EncryptedCsrfPair { token, cookie } = generate_csrf_pair().unwrap();
        verify_csrf_pair(Base64CsrfPair { token, cookie }).unwrap();
    }
}
