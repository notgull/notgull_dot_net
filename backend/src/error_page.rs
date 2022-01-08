// MIT/Apache2 License

use crate::{query::DatabaseError, templates, PageRenderError};
use std::{error::Error, io::ErrorKind};
use warp::{
    http::StatusCode,
    reply::{html, with_status},
    Rejection, Reply,
};

#[inline]
pub fn process_error(rej: Rejection) -> Result<impl Reply, tera::Error> {
    let (code, desc) = match (rej.find::<PageRenderError>(), rej.is_not_found()) {
        (Some(pre), _) => {
            let mut err_ref: Option<&dyn Error> = Some(&pre);
            //while let Some(err) = err_ref {
            //    tracing::error!("- {}", err);
            //    err_ref = err.source();
            //}

            pre.as_err()
        }
        (_, true) => (StatusCode::NOT_FOUND, "No route found for path"),
        _ => {
            tracing::error!("Unexpected rejection: {:?}", rej);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred",
            )
        }
    };

    // get further details
    let code_num = code.as_u16();
    let title = format!("Error Code {}", code_num);
    let info = ErrInfo {
        title,
        error_code: code_num,
        description: desc,
        our_fault: code_num / 100 == 5,
    };

    let out = templates::template("error", info, Default::default())?;
    Ok(with_status(html(out), code))
}

#[derive(serde::Serialize)]
struct ErrInfo {
    title: String,
    error_code: u16,
    description: &'static str,
    our_fault: bool,
}

impl PageRenderError {
    #[inline]
    fn as_err(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Tera(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Template processing failed",
            ),
            Self::Database(DatabaseError::NotFound) => (
                StatusCode::NOT_FOUND,
                "Unable to fetch resource from database",
            ),
            Self::Database(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unknown SQL error occurred",
            ),
            Self::Csrf(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate CSRF tokens",
            ),
            Self::Io(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unknown I/O error occurred",
            ),
            Self::PermissionDenied => (StatusCode::UNAUTHORIZED, "Permission denied"),
        }
    }
}
