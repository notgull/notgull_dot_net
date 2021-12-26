// GNU AGPL v3 License

use crate::{markdown, models::Blogpost, pagerender, templates, Database, PageRenderError};
use chrono::NaiveDateTime;
use futures_util::{future, TryFutureExt};
use std::sync::Arc;
use warp::{
    reply::{html, with_header},
    Filter, Reply,
};

#[inline]
pub fn blog(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path("blog").and(view_blogpost().or(list_blogpost()))
}

#[inline]
pub fn list_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path::end()
        .and(warp::get())
        .map(|| Title { title: "Blog" })
        .and(pagerender::page_render_loader::<true>())
        .and_then(|data, mut state: pagerender::PageRenderState| {
            future::ready({
                let mut options = templates::TemplateOptions::default();
                options.csrf_token = Some(state.csrf_token());

                templates::template("bloglist", data, options)
                    .map_err(|e| warp::reject::custom(PageRenderError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, mut state: pagerender::PageRenderState| {
            with_header(
                html(res),
                "Set-Cookie",
                format!("csrf_cookie={}", state.csrf_cookie()),
            )
        })
}

#[derive(serde::Serialize)]
struct Title<'a> {
    title: &'a str,
}

#[inline]
pub fn view_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path!(String)
        .and(warp::get())
        .and(crate::with_database())
        .and(pagerender::page_render_loader::<false>())
        .and_then(|url, database, _| {
            view_blogpost_inner(url, database).map_err(|e| warp::reject::custom(e))
        })
}

#[inline]
async fn view_blogpost_inner(
    url: String,
    database: Arc<impl Database>,
) -> Result<impl Reply, PageRenderError> {
    // load blogpost and then user from database
    let (blogpost, user) = database.get_blogpost_and_user_by_url(url).await?;

    // format
    blogpost.render_to_html(&user.name).map(html)
}

impl Blogpost {
    #[inline]
    pub fn render_to_html(self, author_name: &str) -> Result<String, PageRenderError> {
        let Blogpost {
            title,
            tags,
            created_at,
            body,
            ..
        } = self;
        let body = markdown::markdown(&body);
        let rendered = RenderedBlogpost {
            title: &title,
            author_name,
            created_at,
            body: &body,
            taglist: tags.split(',').collect(),
        };
        let result = templates::template("blogpost", rendered, Default::default())?;
        Ok(result)
    }
}

#[derive(serde::Serialize)]
struct RenderedBlogpost<'a, 'b> {
    title: &'a str,
    author_name: &'b str,
    created_at: NaiveDateTime,
    body: &'a str,
    taglist: Vec<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::view_blogpost;
    use crate::{markdown, models::Blogpost, templates};
    use warp::Reply;

    #[test]
    fn render_to_html() {
        templates::initialize_test_templates().unwrap();
        markdown::initialize_markdown();

        let blogpost = Blogpost {
            id: 1,
            title: "Chasing Suns".into(),
            tags: "story,humor,nothing".into(),
            url: "chasing-suns".into(),
            body: "...and we spent so much *time* chasing ~~suns~~, we forgot what **we** were really after.".into(),
            author_id: 1,
            created_at: chrono::Local::now().naive_local(),
        };
        let author_name = "John Notgull";

        let html = blogpost.render_to_html(author_name).unwrap();

        // check to see if it contains strings
        let string_contained = vec![
            "<title>Chasing Suns</title>",
            "<h1>Chasing Suns</h1>",
            ">story</",
            ">humor</",
            ">nothing</",
            "<strong>we</strong>",
            "and",
        ];

        for string in string_contained {
            assert!(html.contains(string), "Could not find `{}`", string);
        }
    }

    #[tokio::test]
    async fn test_blogpost_route() {
        templates::initialize_test_templates().unwrap();
        markdown::initialize_markdown();

        let filter = view_blogpost();

        let value = warp::test::request()
            .method("GET")
            .path("/chasing-suns")
            .filter(&filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), 200);
        let response = warp::hyper::body::to_bytes(value.into_body())
            .await
            .unwrap();
        let response = String::from_utf8(response.to_vec()).unwrap();
        assert!(response.contains("we spent so much time chasing suns"));
    }
}
