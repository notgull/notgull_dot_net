// GNU AGPL v3 License

use super::{
    schema::{blogposts, users},
    Database, DatabaseError,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Queryable, Identifiable, AsChangeset, Serialize)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub roles: i64,
}

#[derive(Insertable, Deserialize)]
#[table_name = "users"]
pub struct NewUser {
    pub uuid: String,
    pub name: String,
    pub roles: i64,
}

#[derive(Deserialize)]
pub struct UserFilter {
    pub name: Option<String>,
}

#[derive(Default, Deserialize, AsChangeset)]
#[table_name = "users"]
pub struct UserChange {
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub roles: Option<i64>,
}

#[derive(Clone, Queryable, Identifiable, AsChangeset, Deserialize, Serialize)]
#[table_name = "blogposts"]
pub struct Blogpost {
    pub id: i32,
    pub title: String,
    pub tags: String,
    pub url: String,
    pub body: String,
    pub author_id: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[table_name = "blogposts"]
pub struct NewBlogpost {
    pub title: String,
    pub tags: String,
    pub url: String,
    pub body: String,
    pub author_id: i32,
}

#[derive(Deserialize)]
pub struct BlogpostFilter {
    pub title: Option<String>,
    pub tags: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub author_id: Option<i32>,
}

#[derive(Default, Deserialize, AsChangeset)]
#[table_name = "blogposts"]
pub struct BlogpostChange {
    pub title: Option<String>,
    pub tags: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub author_id: Option<i32>,
}

#[async_trait]
pub trait Model: Sized {
    type ListFilter;
    type NewInstance;
    type UpdateInstance;

    /// Get a single instance by its ID.
    async fn get(db: &(impl Database + Send + Sync), id: i32) -> Result<Self, DatabaseError>;
    /// List instances using a filter.
    async fn list(
        db: &(impl Database + Send + Sync),
        filter: Self::ListFilter,
    ) -> Result<Vec<Self>, DatabaseError>;
    /// Create a new instance.
    async fn create(
        db: &(impl Database + Send + Sync),
        new: Self::NewInstance,
    ) -> Result<i32, DatabaseError>;
    /// Update this instance with new properties.
    async fn update(
        db: &(impl Database + Send + Sync),
        id: i32,
        patch: Self::UpdateInstance,
    ) -> Result<(), DatabaseError>;
    /// Delete this instance by its ID.
    async fn delete(db: &(impl Database + Send + Sync), id: i32) -> Result<(), DatabaseError>;
}

#[async_trait]
impl Model for User {
    type ListFilter = UserFilter;
    type NewInstance = NewUser;
    type UpdateInstance = UserChange;

    #[inline]
    async fn get(db: &(impl Database + Send + Sync), id: i32) -> Result<Self, DatabaseError> {
        db.get_user_by_id(id).await
    }

    #[inline]
    async fn list(
        db: &(impl Database + Send + Sync),
        filter: Self::ListFilter,
    ) -> Result<Vec<Self>, DatabaseError> {
        db.list_users(filter).await
    }

    #[inline]
    async fn create(
        db: &(impl Database + Send + Sync),
        new: Self::NewInstance,
    ) -> Result<i32, DatabaseError> {
        db.insert_user(new).await
    }

    #[inline]
    async fn update(
        db: &(impl Database + Send + Sync),
        id: i32,
        patch: Self::UpdateInstance,
    ) -> Result<(), DatabaseError> {
        db.update_user(id, patch).await
    }

    #[inline]
    async fn delete(db: &(impl Database + Send + Sync), id: i32) -> Result<(), DatabaseError> {
        db.delete_user(id).await
    }
}

#[async_trait]
impl Model for Blogpost {
    type ListFilter = BlogpostFilter;
    type NewInstance = NewBlogpost;
    type UpdateInstance = BlogpostChange;

    #[inline]
    async fn get(db: &(impl Database + Send + Sync), id: i32) -> Result<Self, DatabaseError> {
        db.get_blogpost_by_id(id).await
    }

    #[inline]
    async fn list(
        db: &(impl Database + Send + Sync),
        filter: Self::ListFilter,
    ) -> Result<Vec<Self>, DatabaseError> {
        db.list_blogposts(filter).await
    }

    #[inline]
    async fn create(
        db: &(impl Database + Send + Sync),
        new: Self::NewInstance,
    ) -> Result<i32, DatabaseError> {
        db.insert_blogpost(new).await
    }

    #[inline]
    async fn update(
        db: &(impl Database + Send + Sync),
        id: i32,
        patch: Self::UpdateInstance,
    ) -> Result<(), DatabaseError> {
        db.update_blogpost(id, patch).await
    }

    #[inline]
    async fn delete(db: &(impl Database + Send + Sync), id: i32) -> Result<(), DatabaseError> {
        db.delete_blogpost(id).await
    }
}
