// GNU AGPL v3 License

use crate::{
    auth::{self, with_session, Permissions, Session},
    csrf_integration::{self, CsrfError},
    models::Model,
    query::{with_database, Database, DatabaseError},
};
use bytes::Bytes;
use dashmap::mapref::one::Ref;
use futures_util::future::{self, TryFutureExt};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::Level;
use warp::{http::StatusCode, reject::custom as reject, reply::json, Filter, Reply};

#[inline]
pub fn model<M: Model + 'static, I>(
    name: &'static str,
    invalidator: I,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    M: Serialize,
    M::ListFilter: DeserializeOwned + Send + 'static,
    M::NewInstance: DeserializeOwned + Send + 'static,
    M::UpdateInstance: DeserializeOwned + Send + 'static,
    I: Fn(i32) + Clone + Copy + Send + Sync + 'static,
{
    // base that gets the body to deserialize from, as well as the current database
    let loader = loader_filter();

    // implement the filters on top of that
    let list = list_filter::<M, _, _>(&loader);
    let get = get_filter::<M, _, _>(&loader);
    let create = create_filter::<M, _, _>(&loader);
    let update = update_filter::<M, _, _, I>(&loader, invalidator);
    let delete = delete_filter::<M, _, _, I>(&loader, invalidator);

    // combine into final filter
    warp::path(name)
        .and(
            list.or(get)
                .or(create)
                .or(update)
                .or(delete)
                .recover(|rej: warp::Rejection| {
                    future::ready({
                        match rej.find::<ModelError>() {
                            Some(me) => {
                                tracing::event!(Level::ERROR, "{}", me);

                                let (status_code, description) = me.as_status_and_description();
                                Ok(warp::reply::with_status(
                                    warp::reply::json(&SerModelError {
                                        error: true,
                                        description,
                                    }),
                                    status_code,
                                ))
                            }
                            None => Err(rej),
                        }
                    })
                }),
        )
        .boxed()
}

#[inline]
fn loader_filter() -> impl Filter<Extract = LoaderData<impl Database>, Error = warp::Rejection>
       + Clone
       + Send
       + Sync
       + 'static {
    csrf_integration::check_csrf::<ModelError>()
        .and(with_database())
        .and(
            with_session().map(|s: Option<Ref<'static, String, Session>>| match s {
                Some(s) => s.roles,
                None => Permissions(0b0),
            }),
        )
}

#[inline]
fn check_permsissions<T>(
    data: T,
    user_perms: Permissions,
    required_perms: Permissions,
) -> Result<T, warp::Rejection> {
    if required_perms.applies_to(user_perms) {
        Ok(data)
    } else {
        Err(reject(ModelError::PermissionDenied))
    }
}

type LoaderData<D> = (Bytes, Arc<D>, Permissions);

/// List the model based on a filter.
#[inline]
fn list_filter<M: Model, D: Database + Send + Sync + 'static, F>(
    loader: &F,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    M: Serialize,
    M::ListFilter: DeserializeOwned + Send + 'static,
    F: Filter<Extract = LoaderData<D>, Error = warp::Rejection> + Clone + Send + Sync + 'static,
{
    warp::path::end()
        .and(warp::get())
        .and(loader.clone())
        .and(warp::any().map(|| M::LIST_PERMS))
        .and_then(|body: Bytes, db, uperms, rperms| {
            future::ready(check_permsissions((body, db), uperms, rperms))
        })
        .untuple_one()
        .and_then(|body: Bytes, db| {
            future::ready({
                let filters = serde_urlencoded::from_bytes::<M::ListFilter>(&body);
                match filters {
                    Ok(filters) => Ok((filters, db)),
                    Err(e) => Err(reject(ModelError::from(e))),
                }
            })
        })
        .untuple_one()
        .and_then(|filters, db: Arc<_>| async move {
            M::list(&*db, filters)
                .await
                .map_err(|e| reject(ModelError::from(e)))
        })
        .map(|instances| json(&instances))
}

/// Get a single model based on its id.
#[inline]
fn get_filter<M: Model, D: Database + Send + Sync + 'static, F>(
    loader: &F,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    M: Serialize,
    F: Filter<Extract = LoaderData<D>, Error = warp::Rejection> + Clone + Send + Sync + 'static,
{
    warp::path!(i32)
        .and(warp::get())
        .and(loader.clone())
        .and(warp::any().map(|| M::GET_PERMS))
        .and_then(|id, _, db, uperms, rperms| {
            future::ready(check_permsissions((id, db), uperms, rperms))
        })
        .untuple_one()
        .and_then(|id, db: Arc<_>| async move {
            M::get(&*db, id)
                .await
                .map_err(|e| reject(ModelError::from(e)))
        })
        .map(|instance| json(&instance))
}

/// Create a new instance of the model.
#[inline]
fn create_filter<M: Model, D: Database + Send + Sync + 'static, F>(
    loader: &F,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    M: Serialize,
    M::NewInstance: DeserializeOwned + Send + 'static,
    F: Filter<Extract = LoaderData<D>, Error = warp::Rejection> + Clone + Send + Sync + 'static,
{
    warp::path::end()
        .and(warp::post())
        .and(loader.clone())
        .and(warp::any().map(|| M::CREATE_PERMS))
        .and_then(|body: Bytes, db, uperms, rperms| {
            future::ready(check_permsissions((body, db), uperms, rperms))
        })
        .untuple_one()
        .and_then(|body: Bytes, db| {
            future::ready({
                let new = serde_json::from_slice::<M::NewInstance>(&body);
                match new {
                    Ok(new) => Ok((new, db)),
                    Err(e) => Err(reject(ModelError::from(e))),
                }
            })
        })
        .untuple_one()
        .and_then(move |new, db: Arc<_>| async move {
            let res = M::create(&*db, new)
                .await
                .map_err(|e| reject(ModelError::from(e)));
            res
        })
        .map(|id| {
            let wrapper = IdWrapper { id };
            warp::reply::with_status(json(&wrapper), StatusCode::CREATED)
        })
}

/// Update the model based on a few facts.
#[inline]
fn update_filter<M: Model, D: Database + Send + Sync + 'static, F, I>(
    loader: &F,
    invalidator: I,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    M::UpdateInstance: DeserializeOwned + Send + 'static,
    F: Filter<Extract = LoaderData<D>, Error = warp::Rejection> + Clone + Send + Sync + 'static,
    I: Fn(i32) + Clone + Copy + Send + Sync + 'static,
{
    warp::path!(i32)
        .and(warp::patch())
        .and(loader.clone())
        .and(warp::any().map(|| M::UPDATE_PERMS))
        .and_then(|id, body: Bytes, db, uperms, rperms| {
            future::ready(check_permsissions((id, body, db), uperms, rperms))
        })
        .untuple_one()
        .and_then(|id, body: Bytes, db| {
            future::ready({
                let changes = serde_json::from_slice::<M::UpdateInstance>(&body);
                match changes {
                    Ok(changes) => Ok((id, changes, db)),
                    Err(e) => Err(reject(ModelError::from(e))),
                }
            })
        })
        .untuple_one()
        .and_then(move |id, changes, db: Arc<_>| async move {
            let res = M::update(&*db, id, changes)
                .await
                .map_err(|e| reject(ModelError::from(e)));
            invalidator(id);
            res
        })
        .map(|()| StatusCode::NO_CONTENT)
}

#[inline]
fn delete_filter<M: Model, D: Database + Send + Sync + 'static, F, I>(
    loader: &F,
    invalidator: I,
) -> impl Filter<Extract = (impl Reply,), Error = warp::Rejection> + Clone + Send + Sync + 'static
where
    F: Filter<Extract = LoaderData<D>, Error = warp::Rejection> + Clone + Send + Sync + 'static,
    I: Fn(i32) + Clone + Copy + Send + Sync + 'static,
{
    warp::path!(i32)
        .and(warp::delete())
        .and(loader.clone())
        .and(warp::any().map(|| M::DELETE_PERMS))
        .and_then(|id: i32, _, db, uperms, rperms| {
            future::ready(check_permsissions((id, db), uperms, rperms))
        })
        .untuple_one()
        .and_then(move |id, db: Arc<_>| async move {
            let res = M::delete(&*db, id)
                .await
                .map_err(|e| reject(ModelError::from(e)));
            invalidator(id);
            res
        })
        .map(|()| StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
struct IdWrapper {
    id: i32,
}

#[derive(Serialize)]
struct SerModelError {
    error: bool,
    description: &'static str,
}

#[derive(Debug, thiserror::Error)]
enum ModelError {
    #[error("{0}")]
    UrlEncoding(#[from] serde_urlencoded::de::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Database(#[from] DatabaseError),
    #[error("{0}")]
    Csrf(#[from] csrf_integration::CsrfError),
    #[error("User is unable to access resource")]
    PermissionDenied,
}

impl ModelError {
    #[inline]
    fn as_status_and_description(&self) -> (StatusCode, &'static str) {
        match self {
            ModelError::UrlEncoding(..) => (
                StatusCode::BAD_REQUEST,
                "Unable to parse URL-encoded query parameters",
            ),
            ModelError::Json(..) => (
                StatusCode::BAD_REQUEST,
                "Unable to parse JSON-encoded request body",
            ),
            ModelError::Database(DatabaseError::NotFound) => {
                (StatusCode::NOT_FOUND, "Unable to find the specified model")
            }
            ModelError::Database(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An SQL error occurred during processing",
            ),
            ModelError::Csrf(..) => (StatusCode::BAD_REQUEST, "CSRF verification failed"),
            ModelError::PermissionDenied => (StatusCode::UNAUTHORIZED, "Permission denied"),
        }
    }
}

impl warp::reject::Reject for ModelError {}

#[cfg(test)]
mod tests {
    use super::{
        create_filter, delete_filter, get_filter, list_filter, loader_filter, update_filter,
        IdWrapper,
    };
    use crate::{
        auth::{
            fake_access_token, fake_access_token_fewer_perms, initialize_auth_test, Permissions,
        },
        csrf_integration::{self, EncryptedCsrfPair},
        models::{Blogpost, Model},
        query::{with_database, Database, DatabaseError},
    };
    use bytes::Bytes;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use warp::{
        http::StatusCode,
        hyper::body::{to_bytes, Body},
        test::RequestBuilder,
        Reply,
    };

    #[inline]
    fn url_encode<S: Into<String>>(s: S) -> String {
        use percent_encoding::NON_ALPHANUMERIC;
        percent_encoding::percent_encode(s.into().as_bytes(), NON_ALPHANUMERIC).to_string()
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Dummy {
        data: String,
    }

    #[derive(Deserialize)]
    struct DummyFilter {
        data: Option<String>,
    }

    #[derive(Deserialize)]
    struct NewDummy {
        data: String,
    }

    #[derive(Deserialize)]
    struct DummyChanges {
        data: Option<String>,
    }

    #[inline]
    fn no_cache(_: i32) {}

    #[async_trait::async_trait]
    impl Model for Dummy {
        const GET_PERMS: Permissions = Permissions(0b1);
        const LIST_PERMS: Permissions = Permissions(0b1);
        const CREATE_PERMS: Permissions = Permissions(0b1);
        const UPDATE_PERMS: Permissions = Permissions(0b1);
        const DELETE_PERMS: Permissions = Permissions(0b1);

        type ListFilter = DummyFilter;
        type NewInstance = NewDummy;
        type UpdateInstance = DummyChanges;

        /// Get a single instance by its ID.
        async fn get(_db: &(impl Database + Send + Sync), id: i32) -> Result<Self, DatabaseError> {
            if id == 1 {
                Ok(Self {
                    data: "get()".into(),
                })
            } else {
                Err(DatabaseError::NotFound)
            }
        }
        /// List instances using a filter.
        async fn list(
            _db: &(impl Database + Send + Sync),
            filter: Self::ListFilter,
        ) -> Result<Vec<Self>, DatabaseError> {
            if filter.data.as_deref() == Some("foobar") {
                Ok(vec![
                    Self {
                        data: "list() foobar 1".into(),
                    },
                    Self {
                        data: "list() foobar 2".into(),
                    },
                    Self {
                        data: "list() foobar 3".into(),
                    },
                ])
            } else if filter.data == None {
                Ok(vec![
                    Self {
                        data: "list() 1".into(),
                    },
                    Self {
                        data: "list() 2".into(),
                    },
                    Self {
                        data: "list() 3".into(),
                    },
                ])
            } else {
                panic!()
            }
        }
        /// Create a new instance.
        async fn create(
            _db: &(impl Database + Send + Sync),
            new: Self::NewInstance,
        ) -> Result<i32, DatabaseError> {
            assert_eq!(new.data, "create()");
            Ok(2)
        }
        /// Update this instance with new properties.
        async fn update(
            _db: &(impl Database + Send + Sync),
            id: i32,
            patch: Self::UpdateInstance,
        ) -> Result<(), DatabaseError> {
            assert_eq!(id, 1);
            assert!(patch.data.as_deref() == Some("update()") || patch.data == None);
            Ok(())
        }
        /// Delete this instance by its ID.
        async fn delete(_db: &(impl Database + Send + Sync), id: i32) -> Result<(), DatabaseError> {
            assert_eq!(id, 1);
            Ok(())
        }
    }

    #[tokio::test]
    async fn list_no_filter() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let list = list_filter::<Dummy, _, _>(&loader_filter());
        let value = warp::test::request()
            .path(&format!(
                "/?csrf_token={}&csrf_cookie={}",
                url_encode(token),
                url_encode(cookie),
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&list)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let values: Vec<Dummy> = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            values,
            vec![
                Dummy {
                    data: "list() 1".into()
                },
                Dummy {
                    data: "list() 2".into()
                },
                Dummy {
                    data: "list() 3".into()
                },
            ],
        );
    }

    #[tokio::test]
    async fn list_filtered() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let list = list_filter::<Dummy, _, _>(&loader_filter());
        let value = warp::test::request()
            .path(&format!(
                "/?data=foobar&csrf_token={}&csrf_cookie={}&access_token={}",
                url_encode(token),
                url_encode(cookie),
                url_encode(tok.to_string())
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&list)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let values: Vec<Dummy> = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            values,
            vec![
                Dummy {
                    data: "list() foobar 1".into(),
                },
                Dummy {
                    data: "list() foobar 2".into(),
                },
                Dummy {
                    data: "list() foobar 3".into(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn list_filter_bytes_unmolested() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let list = list_filter::<Dummy, _, _>(&loader_filter());
        let value = warp::test::request()
            .path(&format!(
                "/?irrelevant=foobar&csrf_token={}&csrf_cookie={}",
                url_encode(token),
                url_encode(cookie),
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&list)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let values: Vec<Dummy> = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            values,
            vec![
                Dummy {
                    data: "list() 1".into()
                },
                Dummy {
                    data: "list() 2".into()
                },
                Dummy {
                    data: "list() 3".into()
                },
            ],
        );
    }

    #[tokio::test]
    async fn get() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let get = get_filter::<Dummy, _, _>(&loader_filter());
        let value = warp::test::request()
            .path(&format!(
                "/1?csrf_token={}&csrf_cookie={}",
                url_encode(token),
                url_encode(cookie)
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&get)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: Dummy = serde_json::from_slice(&value).unwrap();

        assert_eq!(value.data, "get()");
    }

    #[tokio::test]
    async fn get_not_found() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let get = super::model::<Dummy, _>("dummy", no_cache);
        let value = warp::test::request()
            .path(&format!(
                "/dummy/2?csrf_token={}&csrf_cookie={}",
                url_encode(token),
                url_encode(cookie)
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&get)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NOT_FOUND);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "error": true,
                "description": "Unable to find the specified model",
            })
        );
    }

    #[tokio::test]
    async fn create() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let create = create_filter::<Dummy, _, _>(&loader_filter());
        let body = format!(
            r#"{{"data":"create()","csrf_token":"{}","csrf_cookie":"{}"}}"#,
            token, cookie
        );
        let value = warp::test::request()
            .path("/")
            .method("POST")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&create)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::CREATED);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "id": 2
            })
        );
    }

    #[tokio::test]
    async fn update() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let update = update_filter::<Dummy, _, _, _>(&loader_filter(), no_cache);
        let body = format!(
            r#"{{"data":"update()","csrf_token":"{}","csrf_cookie":"{}","access_token":"{}"}}"#,
            token, cookie, tok
        );
        let value = warp::test::request()
            .path("/1")
            .method("PATCH")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&update)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn update_partial() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let update = update_filter::<Dummy, _, _, _>(&loader_filter(), no_cache);
        let body = format!(
            r#"{{"csrf_token":"{}","csrf_cookie":"{}","access_token":"{}"}}"#,
            token, cookie, tok
        );
        let value = warp::test::request()
            .path("/1")
            .method("PATCH")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&update)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete() {
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        initialize_auth_test();
        let tok = fake_access_token();
        let delete = delete_filter::<Dummy, _, _, _>(&loader_filter(), no_cache);
        let body = format!(
            r#"{{"csrf_token":"{}","csrf_cookie":"{}","access_token":"{}"}}"#,
            token, cookie, tok
        );
        let value = warp::test::request()
            .path("/1")
            .method("DELETE")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&delete)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn blogpost_list() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        let value = warp::test::request()
            .path(&format!(
                "/tbp?csrf_token={}&csrf_cookie={}&access_token={}",
                url_encode(token),
                url_encode(cookie),
                url_encode(tok)
            ))
            .method("GET")
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: Vec<Blogpost> = serde_json::from_slice(&value).unwrap();
        assert!(value.iter().any(|bp| bp.title == "Chasing Suns"));
        assert!(value.iter().any(|bp| bp.title == "How to make a website"));
    }

    #[tokio::test]
    async fn blogpost_get() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        for (id, title) in [(1, "Chasing Suns"), (2, "How to make a website")] {
            let value = warp::test::request()
                .path(&format!(
                    "/tbp/{}?csrf_token={}&csrf_cookie={}&access_token={}",
                    id,
                    url_encode(token.clone()),
                    url_encode(cookie.clone()),
                    url_encode(tok),
                ))
                .method("GET")
                .header("Cookie", format!("access_token={}", tok))
                .filter(&model_filter)
                .await
                .unwrap()
                .into_response();

            assert_eq!(value.status(), StatusCode::OK);

            let value = to_bytes(value.into_body()).await.unwrap();
            let value: Blogpost = serde_json::from_slice(&value).unwrap();
            assert_eq!(value.title, title);
        }
    }

    #[tokio::test]
    async fn blogpost_get_not_found() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        let value = warp::test::request()
            .path(&format!(
                "/tbp/3?csrf_token={}&csrf_cookie={}&access_token={}",
                url_encode(token),
                url_encode(cookie),
                url_encode(tok)
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NOT_FOUND);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&value).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "error": true,
                "description": "Unable to find the specified model",
            })
        );
    }

    #[tokio::test]
    async fn blogpost_create() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        let body = format!(
            r#"{{
                "title":"Test1",
                "tags":"test2",
                "url":"test3",
                "body":"test4",
                "author_id":1,
                "csrf_token":"{}",
                "csrf_cookie":"{}"
            }}"#,
            &token, &cookie,
        );
        let value = warp::test::request()
            .path("/tbp/")
            .method("POST")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        let value = String::from_utf8(to_bytes(value.into_body()).await.unwrap().to_vec()).unwrap();
        if value.contains("error") {
            panic!("{}", value);
        }
        let IdWrapper { id } = serde_json::from_str(&value).unwrap();

        assert_eq!(id, 3);

        let value = warp::test::request()
            .path(&format!(
                "/tbp/{}?csrf_token={}&csrf_cookie={}&access_token={}",
                id,
                url_encode(token),
                url_encode(cookie),
                url_encode(tok),
            ))
            .method("GET")
            .header("Cookie", format!("access_token={}", tok))
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: Blogpost = serde_json::from_slice(&value).unwrap();

        assert_eq!(value.title, "Test1");
    }

    #[tokio::test]
    async fn blogpost_update() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        let body = format!(
            r#"{{
                "title":"Breaking Bones",
                "csrf_token":"{}",
                "csrf_cookie":"{}",
                "access_token":"{}"
            }}"#,
            &token, &cookie, tok
        );
        let value = warp::test::request()
            .path("/tbp/1")
            .method("PATCH")
            .body(body)
            .header("Cookie", format!("access_token={}", tok))
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NO_CONTENT);

        let value = warp::test::request()
            .path(&format!(
                "/tbp/1?csrf_token={}&csrf_cookie={}&access_token={}",
                url_encode(token),
                url_encode(cookie),
                url_encode(tok)
            ))
            .method("GET")
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::OK);

        let value = to_bytes(value.into_body()).await.unwrap();
        let value: Blogpost = serde_json::from_slice(&value).unwrap();

        assert_eq!(value.title, "Breaking Bones")
    }

    #[tokio::test]
    async fn blogpost_delete() {
        csrf_integration::initialize_csrf_test();
        initialize_auth_test();
        let tok = fake_access_token();
        let EncryptedCsrfPair { token, cookie } = csrf_integration::generate_csrf_pair().unwrap();
        let model_filter = super::model::<Blogpost, _>("tbp", no_cache);
        let body = format!(
            r#"{{"csrf_token":"{}","csrf_cookie":"{}","access_token":"{}"}}"#,
            &token, &cookie, tok
        );
        let value = warp::test::request()
            .path("/tbp/1")
            .method("DELETE")
            .header("Cookie", format!("access_token={}", tok))
            .body(body)
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NO_CONTENT);

        let value = warp::test::request()
            .path(&format!(
                "/tbp/1?csrf_token={}&csrf_cookie={}&access_token={}",
                url_encode(token),
                url_encode(cookie),
                url_encode(tok)
            ))
            .method("GET")
            .filter(&model_filter)
            .await
            .unwrap()
            .into_response();

        assert_eq!(value.status(), StatusCode::NOT_FOUND);
    }
}
