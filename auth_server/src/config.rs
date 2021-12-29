// GNU AGPL v3 License

use std::{
    ffi::CString,
    io,
    net::IpAddr,
    path::{Path, PathBuf},
};
use tokio::{
    fs,
    io::{AsyncReadExt, BufReader},
};

#[derive(serde::Deserialize)]
pub struct Config {
    pub hostname: IpAddr,
    pub port: u16,
    pub key_path: PathBuf,
    pub cert_path: PathBuf,
    pub pepper: CString,
    pub verify: OauthVerify,
    pub urls: Urls,
}

#[derive(Clone, serde::Deserialize)]
pub struct OauthVerify {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(serde::Deserialize)]
pub struct Urls {
    pub base_url: String,
}

impl Config {
    #[inline]
    pub async fn read_from_file<P: AsRef<Path>>(p: P) -> Result<Self, ConfigLoadError> {
        // deserialize toml config from file
        let mut file = BufReader::new(fs::File::open(p).await?);
        let mut data = vec![];
        file.read_to_end(&mut data).await?;
        let cfg: Self = toml::de::from_slice(&data)?;
        Ok(cfg)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
}
