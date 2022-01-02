// GNU AGPL v3 License

mod model;
mod set_username;

use crate::models::{Blogpost, User};
use warp::{http::StatusCode, Filter, Reply};

#[inline]
pub fn api(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    // create model routes
    let user = model::model::<User>("user");
    let blogpost = model::model::<Blogpost>("blogpost");

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
        .or(not_found);

    warp::path("api").and(api).boxed()
}

#[derive(serde::Serialize)]
struct NotFoundError {
    error: bool,
    description: &'static str,
}
