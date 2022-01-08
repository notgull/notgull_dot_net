// GNU AGPL v3 License

use crate::{admin, api, auth, blog, error_page, frontpages, Config};
use dashmap::mapref::one::Ref;
use futures_util::future::{err, ok, ready};
use std::convert::Infallible;
use tracing::Level;
use warp::{
    http::StatusCode,
    reply::{with_header, with_status},
    Filter, Rejection, Reply,
};

/// Sum route that directs this website.
#[inline]
pub fn routes(
    cfg: &Config,
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
        .boxed()
        .or(blog::blog())
        .boxed()
        .or(auth::login())
        .boxed()
        .or(auth::callback())
        .boxed()
        .or(admin::admin())
        .boxed()
        .or(favicon_route(cfg))
        .boxed()
        .or(frontpages::root_and_front(cfg))
        .boxed()
        .recover(|rej: Rejection| {
            tracing::debug!("Rejection encountered: {:?}", &rej);
            ready(error_page::process_error(rej).map_err(|err| {
                tracing::error!("Cannot process error template: {:?}", err);
                warp::reject()
            }))
        })
        .recover(|_| {
            ready(Result::<_, Infallible>::Ok(with_status(
                "An error occurred while rendering the error display page",
                StatusCode::INTERNAL_SERVER_ERROR,
            )))
        })
    //        .map(|repl| with_header(repl, "Access-Control-Allow-Origin", "*"))
}

#[inline]
fn favicon_route(
    cfg: &Config,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("favicon.ico").and(warp::fs::file(cfg.favicon_path.clone()))
}
