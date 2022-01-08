// GNU AGPL v3 License

use crate::{markdown, pagerender, templates, Config, FrontpageEntry, PageRenderError, Title};
use bytes::Bytes;
use dashmap::DashMap;
use futures_util::future::{ok, ready, TryFutureExt};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    mem,
    sync::{Arc, Mutex},
};
use tokio::{fs::File, io::AsyncReadExt};
use warp::{
    reject::{custom as reject, not_found},
    reply::{html, with_header},
    Filter, Rejection, Reply,
};

#[inline]
pub fn root_and_front(
    cfg: &Config,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    rootpage().or(frontpages(cfg))
}

#[inline]
fn rootpage(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path::end()
        .and(warp::get())
        .map(|| Title { title: "Homepage" })
        .and(pagerender::page_render_loader::<true>(0))
        .and_then(|data, mut state: pagerender::PageRenderState| {
            ready({
                templates::template("homepage", data, state.template_options())
                    .map_err(|e| reject(PageRenderError::from(e)))
                    .map(move |t| (t, state))
            })
        })
        .untuple_one()
        .map(|res: String, pr: pagerender::PageRenderState| html(res))
}

#[inline]
fn frontpages(
    cfg: &Config,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    let frontpage_map = Arc::new(
        cfg.frontpage_map
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    Arc::new(FrontpageCache {
                        entry: v.clone(),
                        cache: RwLock::new(None),
                    }),
                )
            })
            .collect::<HashMap<String, Arc<FrontpageCache>>>(),
    );

    warp::path!(String)
        .and_then(move |name| {
            ready(
                frontpage_map
                    .get(&name)
                    .map(|e| (&*e).clone())
                    .ok_or_else(not_found),
            )
        })
        .and_then(|entry: Arc<FrontpageCache>| {
            let fut = async move {
                // if the cache is valid, use that
                let cache_lock = entry.cache.read();
                if let Some(cache) = &*cache_lock {
                    // fast path: we have a cache we can read from
                    let cache = cache.clone();
                    mem::drop(cache_lock);
                    return Ok((entry, cache));
                }
                mem::drop(cache_lock);

                let mut buf = String::new();
                let mut file = File::open(&entry.entry.path).await?;
                file.read_to_string(&mut buf).await?;

                // convert to html via markdown
                let res = tokio::task::spawn_blocking(move || markdown::markdown(&buf))
                    .await
                    .expect("Blocking task failed");

                // convert to Bytes and store in cache
                let res: Arc<str> = res.into_boxed_str().into();
                let mut cache_lock = entry.cache.write();
                *cache_lock = Some(res.clone());
                mem::drop(cache_lock);

                Result::<_, PageRenderError>::Ok((entry, res))
            };

            fut.map_err(reject)
        })
        .untuple_one()
        .and(pagerender::page_render_loader::<false>(0))
        .and_then(
            |entry: Arc<FrontpageCache>, body: Arc<str>, mut state: pagerender::PageRenderState| {
                ready({
                    templates::template(
                        "frontpage",
                        Frontpage {
                            title: entry.entry.name.clone(),
                            body: &*body,
                        },
                        state.template_options(),
                    )
                    .map_err(|e| reject(PageRenderError::from(e)))
                })
            },
        )
        .map(html)
}

struct FrontpageCache {
    entry: FrontpageEntry,
    cache: RwLock<Option<Arc<str>>>,
}

#[derive(serde::Serialize)]
struct Frontpage<'a> {
    title: String,
    body: &'a str,
}
