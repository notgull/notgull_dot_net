// GNU AGPL v3 License

mod image;
mod model;
mod set_username;

use crate::{
    models::{Blogpost, User},
    Config,
};
use warp::{http::StatusCode, Filter, Reply};

#[inline]
pub async fn initialize_api(cfg: &Config) {
    image::initialize_s3(cfg).await;
}

#[inline]
fn no_cache(_: i32) {}

#[inline]
pub fn api(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    // create model routes
    let user = model::model::<User, _>("user", no_cache);
    let blogpost = model::model::<Blogpost, _>("blogpost", crate::blog::invalidate_cache);

    // handle 404's by sending back an error message
    let not_found = warp::any().map(|| {
        warp::reply::with_status(
            warp::reply::json(&NotFoundError {
                error: true,
                description: "No route found",
            }),
            StatusCode::NOT_FOUND,
        )
    });

    let api = user
        .or(blogpost)
        .or(set_username::set_username())
        .or(image::image())
        .or(not_found);

    warp::path("api").and(api).boxed()
}

#[derive(serde::Serialize)]
struct NotFoundError {
    error: bool,
    description: &'static str,
}
