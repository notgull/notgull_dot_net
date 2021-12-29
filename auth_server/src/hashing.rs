// GNU AGPL v3 License

use argon2::Argon2;
use password_hash::{
    PasswordHash, PasswordHashString, PasswordHasher, PasswordVerifier, Salt, SaltString,
};
use rand_core::OsRng;

#[inline]
pub fn hash_password(pwd: &[u8], salt: &[u8]) -> Result<Vec<u8>, HashError> {
    let salt = Salt::new(std::str::from_utf8(salt)?)?;
    let argon = Argon2::default();
    Ok(argon
        .hash_password(pwd, &salt)?
        .serialize()
        .as_bytes()
        .to_vec())
}

#[inline]
pub fn verify_password(pwd: &[u8], hash: &[u8]) -> Result<(), HashError> {
    let hash: String = String::from_utf8(hash.to_vec())?;
    let hash = PasswordHashString::new(&hash)?;
    let hash = hash.password_hash();
    let argon = Argon2::default();

    argon.verify_password(pwd, &hash).map_err(Into::into)
}

#[inline]
pub fn generate_salt() -> Vec<u8> {
    SaltString::generate(&mut OsRng).as_bytes().to_vec()
}

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("{0}")]
    Hash(#[from] argon2::password_hash::errors::Error),
    #[error("{0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("{0}")]
    OwnedUtf8(#[from] std::string::FromUtf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_sanity() {
        let salt = generate_salt();
        const PWD: &str = "HelloWorld1111";
        let hash = hash_password(PWD.as_bytes(), &salt).unwrap();
        verify_password(PWD.as_bytes(), &hash).unwrap();
    }
}
