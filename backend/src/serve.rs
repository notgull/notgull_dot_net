// GNU AGPL v3 License

use crate::Config;
use futures_util::future;
use std::{convert::Infallible, io::Error};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use warp::{Filter, Reply};

#[inline]
pub async fn serve(
    filter: impl Filter<Extract = impl Reply, Error = Infallible> + Clone + Send + Sync + 'static,
    cfg: &Config,
) -> Result<(), Error> {
    // read private and public keys to file
    let mut key_data = vec![];
    let mut cert_data = vec![];
    let mut key_file = BufReader::new(File::open(&cfg.tls.private_key).await?);
    let mut cert_file = BufReader::new(File::open(&cfg.tls.public_key).await?);
    key_file.read_to_end(&mut key_data).await?;
    cert_file.read_to_end(&mut cert_data).await?;

    // set up initial server configuration
    let http_service = warp::serve(filter.clone())
        .run((cfg.hostname, cfg.http_port));
        
    let https_service = warp::serve(filter)
        .tls()
        .cert(cert_data)
        .key(key_data)
        .run((cfg.hostname, cfg.port));

    // run the server
    future::join(http_service, https_service).await;

    Ok(())
}
