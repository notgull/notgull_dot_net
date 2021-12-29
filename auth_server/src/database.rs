// GNU AGPL v3 License

use crate::{
    models::*,
    query::{Database, DatabaseError},
    schema,
};
use chrono::Utc;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool, PoolError, PooledConnection},
};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use std::env;
use tokio::task::spawn_blocking;

#[inline]
pub fn initialize_database() -> Result<(), InitDatabaseError> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").map_err(|_| InitDatabaseError::NoDatabaseUrl)?;
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager)?;
    let _ = POOL.set(pool);

    Ok(())
}

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
    #[error("Unable to find database URL")]
    NoDatabaseUrl,
    #[error("{0}")]
    Pool(#[from] PoolError),
}

#[derive(Copy, Clone)]
pub struct SqlDatabase;

#[async_trait::async_trait]
impl Database for SqlDatabase {
    #[inline]
    async fn fetch_user_by_uuid(&self, suuid: String) -> Result<User, DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::users;

            let conn = connect()?;

            let user = users::table.filter(users::uuid.eq(suuid)).first(&conn)?;
            Ok(user)
        })
        .await
        .expect(BFAIL)
    }

    #[inline]
    async fn fetch_user_by_username_or_email(
        &self,
        query: String,
    ) -> Result<(User, ManagedUser), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::{managedusers, users};

            let conn = connect()?;
            let (user, mu) = users::table
                .inner_join(managedusers::table)
                .filter(
                    managedusers::username
                        .eq(&query)
                        .or(managedusers::email.eq(&query)),
                )
                .first(&conn)?;
            Ok((user, mu))
        })
        .await
        .expect(BFAIL)
    }

    #[inline]
    async fn fetch_user_password(
        &self,
        mu: ManagedUser,
        ip_address: String,
    ) -> Result<(Vec<u8>, i32), DatabaseError> {
        spawn_blocking(move || {
            use diesel::prelude::*;
            use schema::{ipaddresses, managedusers, shadow};

            let conn = connect()?;

            // first, insert or update the new IP address
            let new_ip = NewIpAddress {
                user_id: mu.id,
                ip_address,
            };
            let changed_ip = IpAddressChange {
                last_used: Some(Utc::now().naive_utc()),
            };
            diesel::insert_into(ipaddresses::table)
                .values(&new_ip)
                .on_conflict((ipaddresses::user_id, ipaddresses::ip_address))
                .do_update()
                .set(&changed_ip)
                .execute(&conn)?;

            // second, verify the hashed password
            let (hashed_password, login_attempts) = managedusers::table
                .inner_join(shadow::table)
                .filter(managedusers::id.eq(mu.id))
                .select((shadow::hashed_password, managedusers::login_attempts))
                .first(&conn)?;

            Ok((hashed_password, login_attempts))
        })
        .await
        .expect(BFAIL)
    }
}

const BFAIL: &'static str = "Blocking task panicked";
