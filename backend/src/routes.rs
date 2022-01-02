// GNU AGPL v3 License

use crate::{api, auth, blog, Config};
use dashmap::mapref::one::Ref;
use futures_util::future::{err, ok, ready};
use std::convert::Infallible;
use tracing::Level;
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// Sum route that directs this website.
#[inline]
pub fn routes(
    _cfg: &Config,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static {
    api::api()
        // if the user is logged in and not named, force them to grab it
        .or(warp::get()
            .and(auth::with_session())
            .map(|s: Option<Ref<'static, String, auth::Session>>| match s {
                None => true,
                Some(s) => s.name.is_some(),
            })
            .and_then(|name_exists| {
                if name_exists {
                    err(warp::reject())
                } else {
                    tracing::event!(Level::DEBUG, "Asking for new username");
                    ok(())
                }
            })
            .untuple_one()
            .and(auth::username_form()))
        .or(blog::blog())
        .or(auth::login())
        .or(auth::callback())
        .recover(|rej: Rejection| {
            tracing::debug!("Rejection encountered: {:?}", &rej);
            ready(Result::<&'static str, Rejection>::Err(rej))
        })
        .or(warp::any().map(|| StatusCode::NOT_FOUND))
}
