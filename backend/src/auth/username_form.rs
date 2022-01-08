// GNU AGPL v3 License

use crate::{pagerender, templates, Title};
use futures_util::future::{self, ok};
use std::convert::Infallible;
use tera::Error as TeraError;
use warp::{
    http::StatusCode,
    reply::{html, with_header, with_status},
    Filter, Rejection, Reply,
};

#[inline]
pub fn username_form(
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static {
    warp::any()
        .map(|| Title {
            title: "Enter Username",
        })
        .and(pagerender::page_render_loader::<true>(0))
        .and_then(|data, mut state: pagerender::PageRenderState| {
            future::ready({
                templates::template("usernameform", data, state.template_options())
                    .map_err(|e| warp::reject::custom(UsernameFormError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, mut state: pagerender::PageRenderState| html(res))
        .recover(|rej: Rejection| match rej.find::<UsernameFormError>() {
            Some(UsernameFormError { pre }) => {
                tracing::event!(tracing::Level::ERROR, "{}", pre);
                ok(with_status(
                    "A templating error occurred",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
            None => unreachable!(),
        })
}

#[derive(Debug)]
struct UsernameFormError {
    pre: TeraError,
}

impl From<TeraError> for UsernameFormError {
    #[inline]
    fn from(te: TeraError) -> UsernameFormError {
        UsernameFormError { pre: te }
    }
}

impl warp::reject::Reject for UsernameFormError {}
