// GNU AGPL v3 License

use crate::models::{ManagedUser, User};
use diesel::{r2d2::PoolError, result::Error as DieselError};
use std::{convert::Infallible, sync::Arc};
use warp::Filter;

#[async_trait::async_trait]
pub trait Database {
    /// Fetch a user by its UUID.
    async fn fetch_user_by_uuid(&self, uuid: String) -> Result<User, DatabaseError>;

    /// Given a username and email, return a managed user matching that.
    async fn fetch_user_by_username_or_email(
        &self,
        query: String,
    ) -> Result<(User, ManagedUser), DatabaseError>;

    /// Given a user, fetch its hashed password.
    ///
    /// Also, log the IP address used to log in.
    async fn fetch_user_password(
        &self,
        mu: ManagedUser,
        ip_address: String,
    ) -> Result<(Vec<u8>, i32), DatabaseError>;
}

#[cfg(test)]
#[inline]
pub fn database() -> impl Database {
    crate::mock_database::MockDatabase::test_database()
}

#[cfg(not(test))]
#[inline]
pub fn database() -> impl Database {
    crate::database::SqlDatabase
}

#[inline]
pub fn with_database(
) -> impl Filter<Extract = (Arc<impl Database>,), Error = Infallible> + Clone + Send + Sync + 'static
{
    let db = Arc::new(database());
    warp::any().map(move || db.clone())
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("{0}")]
    Pool(#[from] PoolError),
    #[error("Could not find object")]
    NotFound,
    #[error("{0}")]
    Sql(#[source] DieselError),
}

impl From<DieselError> for DatabaseError {
    #[inline]
    fn from(de: DieselError) -> Self {
        match de {
            DieselError::NotFound => DatabaseError::NotFound,
            de => DatabaseError::Sql(de),
        }
    }
}
