// GNU AGPL v3 License

use crate::{
    auth::{with_session, Session},
    pagerender::{page_render_loader, PageRenderState},
    templates, PageRenderError, Title,
};
use dashmap::mapref::one::Ref;
use futures_util::future::ready;
use warp::{reject::custom as reject, reply::html, Filter, Rejection, Reply};

mod user;

#[inline]
pub fn admin(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path("admin").and(admin_console().or(user::user_info()).or(user::user_list()))
}

#[inline]
pub fn admin_console(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path::end()
        .and(warp::get())
        .and(page_render_loader::<false>(0b10))
        .and_then(|mut pr: PageRenderState| {
            ready({
                templates::template(
                    "adminconsole",
                    Title {
                        title: "Admin Console",
                    },
                    pr.template_options(),
                )
                .map_err(|e| reject(PageRenderError::from(e)))
            })
        })
        .map(|res: String| html(res))
}
