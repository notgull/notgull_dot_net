// GNU AGPL v3 License

use crate::{
    pagerender::{page_render_loader, PageRenderState},
    templates, PageRenderError, Title,
};
use futures_util::future::ready;
use warp::{reject::custom as reject, reply::html, Filter, Rejection, Reply};

#[inline]
pub fn user_list(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("users")
        .and(warp::get())
        .and(page_render_loader::<true>(0b10))
        .and_then(|mut pr: PageRenderState| {
            ready({
                templates::template(
                    "userlist",
                    Title { title: "User List" },
                    pr.template_options(),
                )
                .map_err(|e| reject(PageRenderError::from(e)))
            })
        })
        .map(|res: String| html(res))
}

#[inline]
pub fn user_info(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("users" / i32)
        .and(warp::get())
        .and(page_render_loader::<true>(0b10))
        .and_then(|id: i32, mut pr: PageRenderState| {
            ready({
                templates::template(
                    "userinfo",
                    UserInfo {
                        title: "User Info",
                        editing_user_id: id,
                    },
                    pr.template_options(),
                )
                .map_err(|e| reject(PageRenderError::from(e)))
            })
        })
        .map(|res: String| html(res))
}

#[derive(serde::Serialize)]
struct UserInfo<'a> {
    title: &'a str,
    editing_user_id: i32,
}
