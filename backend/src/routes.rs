// GNU AGPL v3 License

use crate::{api, blog, Config};
use std::convert::Infallible;
use warp::{http::StatusCode, Filter, Reply};

/// Sum route that directs this website.
#[inline]
pub fn routes(
    _cfg: &Config,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static {
    // our only route, for now
    api::api()
        .or(blog::blog())
        .or(warp::any().map(|| StatusCode::NOT_FOUND))
}
