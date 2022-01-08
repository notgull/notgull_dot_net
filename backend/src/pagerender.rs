// GNU AGPL v3 License

use crate::{
    auth::{with_session, Permissions, Session},
    csrf_integration::{self, EncryptedCsrfPair},
    templates::TemplateOptions,
    PageRenderError,
};
use dashmap::mapref::one::Ref;
use futures_util::future;
use warp::{reject::custom as reject, Filter};

/// Loader filter for page rendering.
#[inline]
pub fn page_render_loader<const DO_CSRF: bool>(
    permissions: i64,
) -> impl Filter<Extract = (PageRenderState,), Error = warp::Rejection> + Clone + Send + Sync + 'static
{
    let permissions = Permissions(permissions);
    with_session().and_then(move |s: Option<Ref<'static, String, Session>>| {
        future::ready({
            let id = s.as_ref().map(|s| s.id);
            let perms = s.map(|s| s.roles).unwrap_or_else(Default::default);

            if !permissions.applies_to(perms) {
                return future::err(reject(PageRenderError::PermissionDenied));
            }

            if DO_CSRF {
                match csrf_integration::generate_csrf_pair() {
                    Err(e) => Err(reject(PageRenderError::from(e))),
                    Ok(EncryptedCsrfPair { token, cookie }) => Ok(PageRenderState {
                        csrf_tokens: Some((token, cookie)),
                        id,
                        perms,
                    }),
                }
            } else {
                Ok(PageRenderState {
                    csrf_tokens: None,
                    id,
                    perms,
                })
            }
        })
    })
}

#[derive(Default)]
pub struct PageRenderState {
    csrf_tokens: Option<(String, String)>,
    id: Option<i32>,
    perms: Permissions,
}

impl PageRenderState {
    #[inline]
    pub fn csrf_tokens(&mut self) -> (String, String) {
        self.csrf_tokens
            .take()
            .expect("`csrf_token` has already been taken")
    }

    #[inline]
    pub fn template_options(&mut self) -> TemplateOptions {
        TemplateOptions {
            csrf_tokens: self.csrf_tokens.take(),
            id: self.id,
            perms: self.perms,
        }
    }
}
