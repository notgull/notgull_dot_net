// GNU AGPL v3 License

use crate::{
    auth::{with_session, Session},
    csrf_integration::{self, verify_csrf_pair, Base64CsrfPair, CsrfError},
    Config,
};
use aws_sdk_s3::{error::PutObjectError, Client, Region, SdkError};
use aws_smithy_http::endpoint::Endpoint;
use bytes::Buf;
use dashmap::mapref::one::Ref;
use futures_util::{
    future::{err, ok, TryFutureExt},
    stream, StreamExt, TryStreamExt,
};
use once_cell::sync::OnceCell;
use std::convert::{TryFrom, TryInto};
use warp::{
    http::{StatusCode, Uri},
    hyper::Body,
    multipart::{FormData, Part},
    reject::custom as reject,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

const MAX_LEN: u64 = 5 * 1024 * 1024;

struct S3Data {
    client: Client,
    bucket_name: String,
}

static S3_DATA: OnceCell<S3Data> = OnceCell::new();

#[inline]
pub async fn initialize_s3(cfg: &Config) {
    let aws_cfg = aws_config::load_from_env().await;
    let mut s3_cfg_builder = aws_sdk_s3::config::Builder::from(&aws_cfg);
    let endpoint = cfg
        .s3
        .endpoint_url
        .clone()
        .map(|eu| Endpoint::immutable(eu.parse::<Uri>().unwrap()));

    s3_cfg_builder = s3_cfg_builder.region(Some(Region::new(cfg.s3.region.to_string())));

    if let Some(endpoint) = endpoint {
        s3_cfg_builder = s3_cfg_builder.endpoint_resolver(endpoint);
    }

    S3_DATA
        .set(S3Data {
            client: Client::from_conf(s3_cfg_builder.build()),
            bucket_name: cfg.s3.bucket_name.clone(),
        })
        .unwrap_or_else(|_| panic!("`initialize_s3` called twice"));
}

#[inline]
pub fn image(
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::path!("image")
        .and(warp::post())
        .and(with_session())
        .and_then(|s: Option<Ref<'static, String, Session>>| {
            let do_pass = match s {
                Some(s) => s.roles.0 & 0b01 != 0,
                None => false,
            };

            if do_pass {
                ok(())
            } else {
                err(reject(UploadImageError::PermissionDenied))
            }
        })
        .untuple_one()
        .and(with_upload_data())
        .and_then(|u| image_to_s3(u).map_err(reject))
        .map(|url: String| json(&UrlSer { url: &url }))
        .recover(|rej: Rejection| match rej.find::<UploadImageError>() {
            Some(uie) => {
                tracing::error!("Image upload error: {}", &uie);
                let (code, msg) = uie.as_err();
                ok(with_status(
                    json(&ErrSer {
                        error: true,
                        description: msg,
                    }),
                    code,
                ))
            }
            None => err(rej),
        })
}

#[inline]
fn with_upload_data(
) -> impl Filter<Extract = (UploadData,), Error = Rejection> + Clone + Send + Sync + 'static {
    warp::multipart::form()
        .max_length(MAX_LEN)
        .and_then(|form_data: FormData| parse_upload_data(form_data).map_err(reject))
}

#[derive(serde::Serialize)]
struct UrlSer<'a> {
    url: &'a str,
}

#[inline]
async fn image_to_s3(ud: UploadData) -> Result<String, UploadImageError> {
    let UploadData {
        category,
        subcategory,
        filename,
        data,
        content_type,
    } = ud;

    let s3data = S3_DATA
        .get()
        .expect("`initialize_s3` was not called before s3 functions");
    let path = format!("files/{}/{}/{}", category, subcategory, filename);
    let body: aws_smithy_http::body::SdkBody = data.into();

    s3data
        .client
        .put_object()
        .bucket(&s3data.bucket_name)
        .key(path.clone())
        .body(body.into())
        .content_type(content_type)
        .send()
        .await?;

    Ok(path)
}

#[inline]
async fn parse_upload_data(data: FormData) -> Result<UploadData, UploadImageError> {
    data.err_into::<UploadImageError>()
        .try_fold(Default::default(), build_upload_data)
        .await?
        .try_into()
}

#[inline]
async fn build_upload_data(
    mut data: IncompleteUploadData,
    part: Part,
) -> Result<IncompleteUploadData, UploadImageError> {
    match part.name().to_lowercase().as_str() {
        "category" => {
            data.category = Some(part_to_string(part).await?);
        }
        "subcategory" => {
            data.subcategory = Some(part_to_string(part).await?);
        }
        "filename" => {
            data.filename = Some(part_to_string(part).await?);
        }
        "csrf_token" => {
            data.csrf_token = Some(part_to_string(part).await?);
        }
        "csrf_cookie" => {
            data.csrf_cookie = Some(part_to_string(part).await?);
        }
        "data" => {
            data.content_type = Some(part.content_type().unwrap_or("unknown").to_string());
            data.data = Some(Body::wrap_stream(part.stream().map_ok(|mut buf| {
                let len = buf.remaining();
                buf.copy_to_bytes(len)
            })));
        }
        _ => {}
    }

    Ok(data)
}

#[inline]
async fn part_to_string(mut buf: Part) -> Result<String, UploadImageError> {
    let buf = buf.data().await.ok_or(UploadImageError::NoPartData)?;
    let mut buf = buf?;

    let len = buf.remaining();
    let mut data = vec![0u8; len];
    buf.copy_to_slice(&mut data);

    String::from_utf8(data).map_err(UploadImageError::from)
}

struct UploadData {
    category: String,
    subcategory: String,
    filename: String,
    data: Body,
    content_type: String,
}

#[derive(Default)]
struct IncompleteUploadData {
    category: Option<String>,
    subcategory: Option<String>,
    filename: Option<String>,
    data: Option<Body>,
    content_type: Option<String>,
    csrf_token: Option<String>,
    csrf_cookie: Option<String>,
}

impl TryFrom<IncompleteUploadData> for UploadData {
    type Error = UploadImageError;

    #[inline]
    fn try_from(iud: IncompleteUploadData) -> Result<Self, UploadImageError> {
        let IncompleteUploadData {
            category,
            subcategory,
            filename,
            data,
            content_type,
            csrf_token,
            csrf_cookie,
        } = iud;

        if let (Some(token), Some(cookie)) = (csrf_token, csrf_cookie) {
            let pair = Base64CsrfPair { token, cookie };
            verify_csrf_pair(pair)?;
        } else {
            return Err(UploadImageError::IncompleteData("csrf"));
        }

        Ok(Self {
            category: category.ok_or(UploadImageError::IncompleteData("category"))?,
            subcategory: subcategory.ok_or(UploadImageError::IncompleteData("subcategory"))?,
            filename: filename.ok_or(UploadImageError::IncompleteData("filename"))?,
            data: data.ok_or(UploadImageError::IncompleteData("data"))?,
            content_type: content_type.ok_or(UploadImageError::IncompleteData("content_type"))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum UploadImageError {
    #[error("Could not parse form data as appropriate upload data: {0}")]
    IncompleteData(&'static str),
    #[error("Could not parse multipart data: {0}")]
    Multipart(#[from] warp::Error),
    #[error("Could not parse bytes at UTF-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Part has no data")]
    NoPartData,
    #[error("S3 Error: {0}")]
    S3(#[from] Box<SdkError<PutObjectError>>),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("CSRF: {0}")]
    Csrf(#[from] CsrfError),
}

impl From<SdkError<PutObjectError>> for UploadImageError {
    #[inline]
    fn from(s: SdkError<PutObjectError>) -> Self {
        Self::S3(Box::new(s))
    }
}

impl UploadImageError {
    #[inline]
    fn as_err(&self) -> (StatusCode, &'static str) {
        match self {
            Self::IncompleteData(field) => (StatusCode::BAD_REQUEST, field),
            Self::Multipart(..) => (
                StatusCode::BAD_REQUEST,
                "Unspecified multipart error occurred",
            ),
            Self::Utf8(..) => (StatusCode::BAD_REQUEST, "String was not UTF-8"),
            Self::NoPartData => (StatusCode::BAD_REQUEST, "Part has no data?"),
            Self::S3(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An error occurred while uploading to S3",
            ),
            Self::PermissionDenied => (StatusCode::UNAUTHORIZED, "Permission denied"),
            Self::Csrf(..) => (StatusCode::BAD_REQUEST, "CSRF failure"),
        }
    }
}

impl warp::reject::Reject for UploadImageError {}

#[derive(serde::Serialize)]
struct ErrSer {
    error: bool,
    description: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csrf_integration::{generate_csrf_pair, initialize_csrf_test, EncryptedCsrfPair};
    use multipart::{
        client::Multipart,
        mock::{ClientRequest, HttpBuffer},
    };
    use warp::hyper::body::to_bytes;

    #[tokio::test]
    async fn upload_image_test() {
        initialize_csrf_test();

        const TCATEGORY: &str = "category";
        const TSCATEGORY: &str = "subcategory";
        const TFILENAME: &str = "file.txt";
        const TFILE: &str = "This is the file that we are sending.";

        let EncryptedCsrfPair { token, cookie } = generate_csrf_pair().unwrap();

        // mock up a multipart body
        let cr = ClientRequest::default();
        let mut mp = Multipart::from_request(cr).unwrap();

        // write in expected values
        mp.write_text("category", TCATEGORY).unwrap();
        mp.write_text("subcategory", TSCATEGORY).unwrap();
        mp.write_text("filename", TFILENAME).unwrap();
        mp.write_text("csrf_token", token).unwrap();
        mp.write_text("csrf_cookie", cookie).unwrap();
        mp.write_stream(
            "data",
            &mut TFILE.as_bytes(),
            None,
            Some("text/plain".parse().unwrap()),
        )
        .unwrap();
        let HttpBuffer { buf, boundary, .. } = mp.send().unwrap();

        let route = with_upload_data();

        let ud = warp::test::request()
            .path("/")
            .body(buf)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .filter(&route)
            .await
            .unwrap();
        let UploadData {
            category,
            subcategory,
            filename,
            content_type,
            data,
        } = ud;
        let data = to_bytes(data).await.unwrap();

        assert_eq!(category, TCATEGORY);
        assert_eq!(subcategory, TSCATEGORY);
        assert_eq!(filename, TFILENAME);
        assert_eq!(data, TFILE.as_bytes());
        assert_eq!(content_type, "text/plain");
    }
}
