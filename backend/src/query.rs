// GNU AGPL v3 License

use crate::models::{
    Blogpost, BlogpostChange, BlogpostFilter, NewBlogpost, NewUser, User, UserChange, UserFilter,
};
use diesel::result::Error as DieselError;
use std::{convert::Infallible, sync::Arc};
use warp::Filter;

#[async_trait::async_trait]
pub trait Database {
    /// Fetch a `Blogpost` by its ID.
    async fn get_blogpost_by_id(&self, id: i32) -> Result<Blogpost, DatabaseError>;
    /// Fetch a `Blogpost` and `User` by its URL.
    async fn get_blogpost_and_user_by_url(
        &self,
        url: String,
    ) -> Result<(Blogpost, User), DatabaseError>;
    /// Insert a new `Blogpost` into the database.
    async fn insert_blogpost(&self, bp: NewBlogpost) -> Result<i32, DatabaseError>;
    /// Update a `Blogpost` with potential new information.
    async fn update_blogpost(&self, id: i32, bp: BlogpostChange) -> Result<(), DatabaseError>;
    /// List all of the `Blogpost`s in the database, using some parameters.
    /// as filters.
    async fn list_blogposts(&self, filter: BlogpostFilter) -> Result<Vec<Blogpost>, DatabaseError>;
    /// Delete a `Blogpost` by its ID.
    async fn delete_blogpost(&self, id: i32) -> Result<(), DatabaseError>;

    /// Fetch a `User` by its ID.
    async fn get_user_by_id(&self, id: i32) -> Result<User, DatabaseError>;
    /// Fetch a `User` by its UUID.
    async fn get_user_by_uuid(&self, uuid: String) -> Result<User, DatabaseError>;
    /// Insert a new `User` into the database.
    async fn insert_user(&self, user: NewUser) -> Result<i32, DatabaseError>;
    /// Update a `User` with potential new information.
    async fn update_user(&self, id: i32, user: UserChange) -> Result<(), DatabaseError>;
    /// List all available `User`s.
    async fn list_users(&self, filter: UserFilter) -> Result<Vec<User>, DatabaseError>;
    /// Delete a `User` by its ID.
    async fn delete_user(&self, id: i32) -> Result<(), DatabaseError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Unable to find the item by its given parameter")]
    NotFound,
    #[error("{0}")]
    Diesel(#[source] DieselError),
    #[error("{0}")]
    Pool(#[from] diesel::r2d2::PoolError),
}

impl From<DieselError> for DatabaseError {
    #[inline]
    fn from(de: DieselError) -> DatabaseError {
        match de {
            DieselError::NotFound => DatabaseError::NotFound,
            de => DatabaseError::Diesel(de),
        }
    }
}

#[inline]
pub fn with_database(
) -> impl Filter<Extract = (Arc<impl Database>,), Error = Infallible> + Clone + Send + Sync + 'static
{
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let test_db = crate::mock_database::MockDatabase::with_test_data();
            let test_db = Arc::new(test_db);
            warp::any().map(move || test_db.clone())
        } else {
            warp::any().map(|| Arc::new(crate::database::SqlDatabase))
        }
    }
}
