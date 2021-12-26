// GNU AGPL v3 License

use crate::{
    csrf_integration::{self, EncryptedCsrfPair},
    PageRenderError,
};
use futures_util::future;
use warp::{reject::custom as reject, Filter};

/// Loader filter for page rendering.
#[inline]
pub fn page_render_loader<const DO_CSRF: bool>(
) -> impl Filter<Extract = (PageRenderState,), Error = warp::Rejection> + Clone + Send + Sync + 'static
{
    warp::any().and_then(|| {
        future::ready({
            if DO_CSRF {
                match csrf_integration::generate_csrf_pair() {
                    Err(e) => Err(reject(PageRenderError::from(e))),
                    Ok(EncryptedCsrfPair { token, cookie }) => Ok(PageRenderState {
                        csrf_token: Some(token),
                        csrf_cookie: Some(cookie),
                    }),
                }
            } else {
                Ok(PageRenderState {
                    csrf_token: None,
                    csrf_cookie: None,
                })
            }
        })
    })
}

pub struct PageRenderState {
    csrf_token: Option<String>,
    csrf_cookie: Option<String>,
}

impl PageRenderState {
    #[inline]
    pub fn csrf_token(&mut self) -> String {
        self.csrf_token
            .take()
            .expect("`csrf_token` has already been taken")
    }

    #[inline]
    pub fn csrf_cookie(&mut self) -> String {
        self.csrf_cookie
            .take()
            .expect("`csrf_cookie` has already been taken")
    }
}
