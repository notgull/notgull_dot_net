// GNU AGPL v3 License

use crate::{
    models::{
        Blogpost, BlogpostChange, BlogpostFilter, NewBlogpost, NewUser, User, UserChange,
        UserFilter,
    },
    schema, Database, DatabaseError,
};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool, PoolError, PooledConnection},
};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use std::env;
use tokio::task::spawn_blocking;

/// Initialize the web server's database connection pool.
#[inline]
pub fn initialize_database() -> Result<(), InitDatabaseError> {
    dotenv().ok();

    // determine the database url, connect to the database,
    // and build a pool
    let database_url = env::var("DATABASE_URL").map_err(|_| InitDatabaseError::NoDatabaseUrl)?;
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager)?;

    // insert the pool into the static OnceCell
    POOL.set(pool)
        .unwrap_or_else(|_| panic!("`initialize_database` called twice"));

    Ok(())
}

/// Try to retrieve a connection from the pool, pushing the wait onto
/// the blocking task pool if it isn't immediately available.
#[inline]
pub fn connect() -> Result<PgConn, PoolError> {
    let pool = POOL
        .get()
        .expect("Did not call `initialize_database` before `connect`");
    pool.get()
}

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgConn = PooledConnection<ConnectionManager<PgConnection>>;

static POOL: OnceCell<PgPool> = OnceCell::new();

#[derive(Debug, thiserror::Error)]
pub enum InitDatabaseError {
    #[error("{0}")]
    Pool(#[from] PoolError),
    #[error("Unable to find `DATABASE_URL` environment variable")]
    NoDatabaseUrl,
}

#[derive(Copy, Clone)]
pub struct SqlDatabase;

#[async_trait::async_trait]
impl Database for SqlDatabase {
    #[inline]
    async fn get_blogpost_by_id(&self, sid: i32) -> Result<Blogpost, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::blogposts::dsl::*;

            let conn = connect()?;
            let blogpost = blogposts
                .filter(id.eq(sid))
                .first(&conn)
                .optional()?
                .ok_or(DatabaseError::NotFound)?;
            Ok(blogpost)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn get_blogpost_and_user_by_url(
        &self,
        surl: String,
    ) -> Result<(Blogpost, User), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::{blogposts::dsl::*, users};

            let conn = connect()?;
            let blogpost = blogposts
                .filter(url.eq(surl))
                .inner_join(users::table)
                .first(&conn)
                .optional()?
                .ok_or(DatabaseError::NotFound)?;
            Ok(blogpost)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn insert_blogpost(&self, bp: NewBlogpost) -> Result<i32, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::blogposts::dsl::*;

            let conn = connect()?;
            let blogpost: Blogpost = diesel::insert_into(blogposts)
                .values(bp)
                .get_result(&conn)?;
            Ok(blogpost.id)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn update_blogpost(&self, sid: i32, bp: BlogpostChange) -> Result<(), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::blogposts::dsl::*;

            let conn = connect()?;
            diesel::update(blogposts)
                .filter(id.eq(sid))
                .set(bp)
                .execute(&conn)?;
            Ok(())
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn list_blogposts(&self, filter: BlogpostFilter) -> Result<Vec<Blogpost>, DatabaseError> {
        let BlogpostFilter {
            title,
            tags,
            url,
            body,
            author_id,
        } = filter;
        let [stitle, stags, surl, sbody] =
            [title, tags, url, body].map(|t| t.map(|t| format!("%{}%", t)));
        let sauthor_id = author_id;

        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::blogposts::dsl::*;

            // filter on each of the listed fields
            let conn = connect()?;
            let mut query = blogposts.into_boxed();
            if let Some(stitle) = stitle {
                query = query.filter(title.like(stitle));
            }
            if let Some(stags) = stags {
                query = query.filter(tags.like(stags));
            }
            if let Some(surl) = surl {
                query = query.filter(url.like(surl));
            }
            if let Some(sbody) = sbody {
                query = query.filter(body.like(sbody));
            }
            if let Some(sauthor_id) = sauthor_id {
                query = query.filter(author_id.eq(sauthor_id));
            }

            let posts = query.order_by(created_at.desc()).load(&conn)?;
            Ok(posts)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn delete_blogpost(&self, sid: i32) -> Result<(), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::blogposts::dsl::*;

            let conn = connect()?;
            diesel::delete(blogposts.filter(id.eq(sid))).execute(&conn)?;
            Ok(())
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn get_user_by_id(&self, sid: i32) -> Result<User, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            let user = users
                .filter(id.eq(sid))
                .first(&conn)
                .optional()?
                .ok_or(DatabaseError::NotFound)?;
            Ok(user)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn get_user_by_uuid(&self, suuid: String) -> Result<User, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            let user = users
                .filter(uuid.eq(suuid))
                .first(&conn)
                .optional()?
                .ok_or(DatabaseError::NotFound)?;
            Ok(user)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn insert_user(&self, user: NewUser) -> Result<i32, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            let user: User = diesel::insert_into(users).values(user).get_result(&conn)?;
            Ok(user.id)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn update_user(&self, sid: i32, user: UserChange) -> Result<(), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            diesel::update(users)
                .filter(id.eq(sid))
                .set(user)
                .execute(&conn)?;
            Ok(())
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn list_users(&self, filter: UserFilter) -> Result<Vec<User>, DatabaseError> {
        let UserFilter { name } = filter;
        let sname = name.map(|name| format!("%{}%", name));

        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            let mut query = users.into_boxed();
            if let Some(sname) = sname {
                query = query.filter(name.like(sname));
            }

            let userlist: Vec<User> = query.order_by(name).load(&conn)?;
            Ok(userlist)
        })
        .await
        .expect("Blocking task panicked")
    }

    #[inline]
    async fn delete_user(&self, sid: i32) -> Result<(), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users::dsl::*;

            let conn = connect()?;
            diesel::delete(users.filter(id.eq(sid))).execute(&conn)?;
            Ok(())
        })
        .await
        .expect("Blocking task panicked")
    }
}
