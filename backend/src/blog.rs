// GNU AGPL v3 License

use crate::{
    markdown,
    models::Blogpost,
    pagerender,
    templates::{self, TemplateOptions},
    Database, PageRenderError, Title,
};
use bytes::Bytes;
use chrono::NaiveDateTime;
use dashmap::{mapref::entry::Entry, DashMap};
use futures_util::{future, TryFutureExt};
use once_cell::sync::Lazy;
use std::sync::Arc;
use warp::{reply::html, Filter, Reply};

static BLOGPOST_CACHE: Lazy<DashMap<i32, Bytes>> = Lazy::new(DashMap::new);

#[must_use]
#[inline]
pub fn blog(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path("blog").and(
        view_blogpost()
            .or(list_blogpost())
            .or(create_blogpost())
            .or(delete_blogpost())
            .or(edit_blogpost()),
    )
}

#[must_use]
#[inline]
pub fn list_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path::end()
        .and(warp::get())
        .map(|| Title { title: "Blog" })
        .and(pagerender::page_render_loader::<true>(0))
        .and_then(|data, mut state: pagerender::PageRenderState| {
            future::ready({
                let options = state.template_options();

                templates::template("bloglist", data, options)
                    .map_err(|e| warp::reject::custom(PageRenderError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, _| html(res))
        .with(warp::reply::with::header("Cache-Control", "max-age=3600"))
}

#[must_use]
#[inline]
pub fn create_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path!("create")
        .and(warp::get())
        .map(|| Title {
            title: "Create New Blogpost",
        })
        .and(pagerender::page_render_loader::<true>(0b1))
        .and_then(|data, mut state: pagerender::PageRenderState| {
            future::ready({
                let options = state.template_options();

                templates::template("blogcreate", data, options)
                    .map_err(|e| warp::reject::custom(PageRenderError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, _| html(res))
        .with(warp::reply::with::header("Cache-Control", "max-age=3600"))
}

#[must_use]
#[inline]
pub fn edit_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path!("edit" / i32)
        .and(warp::get())
        .map(|id| EditParams {
            title: "Edit Blogpost",
            blogpost_id: id,
        })
        .and(pagerender::page_render_loader::<true>(0b1))
        .and_then(|data, mut state: pagerender::PageRenderState| {
            future::ready({
                let options = state.template_options();

                templates::template("blogedit", data, options)
                    .map_err(|e| warp::reject::custom(PageRenderError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, _| html(res))
        .with(warp::reply::with::header("Cache-Control", "max-age=3600"))
}

#[derive(serde::Serialize)]
struct EditParams<'a> {
    title: &'a str,
    blogpost_id: i32,
}

#[derive(serde::Serialize)]
struct BlogpostDelete<'a> {
    title: String,
    blogpost_id: i32,
    blogpost_name: &'a str,
}

#[must_use]
#[inline]
pub fn delete_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path!("delete" / i32)
        .and(warp::get())
        .and(crate::with_database())
        .and(pagerender::page_render_loader::<true>(0b1))
        .and_then(|id, db, mut render_state: pagerender::PageRenderState| {
            delete_blogpost_inner(id, db, render_state.template_options())
                .map_err(warp::reject::custom)
        })
        .with(warp::reply::with::header("Cache-Control", "max-age=3600"))
}

#[must_use]
#[inline]
pub fn view_blogpost(
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static {
    warp::path!(String)
        .and(warp::get())
        .and(crate::with_database())
        .and(pagerender::page_render_loader::<false>(0))
        .and_then(|url, database, pr| {
            view_blogpost_inner(url, database, pr).map_err(warp::reject::custom)
        })
}

#[inline]
async fn view_blogpost_inner(
    url: String,
    database: Arc<impl Database>,
    mut pr: pagerender::PageRenderState,
) -> Result<impl Reply, PageRenderError> {
    // load blogpost and then user from database
    let (blogpost, user) = database.get_blogpost_and_user_by_url(url).await?;

    // if the blogpost is already in the cache, return that
    let cache = &*BLOGPOST_CACHE;
    match cache.entry(blogpost.id) {
        Entry::Vacant(v) => {
            // format
            let post = tokio::task::spawn_blocking(move || {
                blogpost.render_to_html(user.name.as_deref().unwrap_or("Anonymous"), &mut pr)
            })
            .await
            .expect("Blocking markdown task panicked")?;

            let the_ref = v.insert(post.into_bytes().into());
            let post: &Bytes = &*the_ref;
            Ok(html(post.clone()))
        }
        Entry::Occupied(o) => Ok(html(o.get().clone())),
    }
}

#[inline]
async fn delete_blogpost_inner(
    id: i32,
    db: Arc<impl Database>,
    to: TemplateOptions,
) -> Result<impl Reply, PageRenderError> {
    // load blogpost by ID
    let blogpost = db.get_blogpost_by_id(id).await?;

    let title = format!("Delete \"{}\"", blogpost.url);

    let data = BlogpostDelete {
        title,
        blogpost_id: blogpost.id,
        blogpost_name: &blogpost.title,
    };

    // format
    let form = templates::template("blogdelete", data, to)?;
    Ok(html(form))
}

impl Blogpost {
    /// Render this blogpost into HTML.
    ///
    /// # Errors
    ///
    /// No errors can actually occur as of now.
    #[inline]
    pub fn render_to_html(
        self,
        author_name: &str,
        pr: &mut pagerender::PageRenderState,
    ) -> Result<String, PageRenderError> {
        let Blogpost {
            title,
            tags,
            created_at,
            body,
            id,
            ..
        } = self;
        let body = markdown::markdown(&body);
        let rendered = RenderedBlogpost {
            title: &title,
            author_name,
            created_at,
            body: &body,
            taglist: tags.split(',').collect(),
            blogpost_id: id,
        };
        let result = templates::template("blogpost", rendered, pr.template_options())?;
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
    blogpost_id: i32,
}

#[inline]
pub fn invalidate_cache(id: i32) {
    let cache = &*BLOGPOST_CACHE;
    cache.remove(&id);
}

#[cfg(test)]
mod tests {
    use super::view_blogpost;
    use crate::{markdown, models::Blogpost, pagerender::PageRenderState, templates};
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

        let html = blogpost
            .render_to_html(author_name, &mut PageRenderState::default())
            .unwrap();

        // check to see if it contains strings
        let string_contained = vec![
            "<title>Chasing Suns",
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
