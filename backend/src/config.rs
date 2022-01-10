// GNU AGPL v3 License

use std::{
    collections::HashMap,
    ffi::CString,
    io,
    net::IpAddr,
    path::{Path, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

#[derive(serde::Deserialize)]
pub struct Config {
    pub hostname: IpAddr,
    pub port: u16,
    pub http_port: u16,
    pub template_path: PathBuf,
    pub csrf_key: CString,
    pub favicon_path: PathBuf,
    pub frontpage_map: HashMap<String, FrontpageEntry>,
    pub tls: TlsDetails,
    pub urls: Urls,
    pub oauth2: Oauth2Details,
    pub s3: S3Details,
}

#[derive(serde::Deserialize, Clone)]
pub struct FrontpageEntry {
    pub path: PathBuf,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct Oauth2Details {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
}

#[derive(serde::Deserialize)]
pub struct TlsDetails {
    pub private_key: PathBuf,
    pub public_key: PathBuf,
}

#[derive(Clone, serde::Deserialize)]
pub struct Urls {
    pub static_url: String,
    pub api_url: String,
    pub auth_url: String,
    pub web_url: String,
}

#[derive(serde::Deserialize)]
pub struct S3Details {
    pub bucket_name: String,
    pub endpoint_url: Option<String>,
    pub region: String,
}

impl Config {
    //// Load this config from the file.
    #[inline]
    pub async fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Config, ConfigError> {
        // load the file into the buffer, then parse it
        let mut buffer = vec![];
        let mut file = BufReader::new(File::open(p).await?);
        file.read_to_end(&mut buffer).await?;
        let config: Config = toml::de::from_slice(&buffer)?;
        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
}
