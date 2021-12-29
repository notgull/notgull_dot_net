// GNU AGPL v3 License

use crate::{
    hashing,
    models::*,
    query::{Database, DatabaseError},
};
use chrono::Utc;
use std::sync::{
    atomic::{AtomicI32, Ordering::SeqCst},
    Mutex,
};

pub struct MockDatabase {
    ip_addresses: Mutex<Vec<IpAddress>>,
    managed_users: Mutex<Vec<ManagedUser>>,
    users: Mutex<Vec<User>>,
    shadow: Mutex<Vec<Shadow>>,
    global_id: AtomicI32,
}

impl MockDatabase {
    #[inline]
    pub fn new() -> MockDatabase {
        MockDatabase {
            ip_addresses: Mutex::new(vec![]),
            managed_users: Mutex::new(vec![]),
            users: Mutex::new(vec![]),
            shadow: Mutex::new(vec![]),
            global_id: AtomicI32::new(1),
        }
    }

    #[inline]
    pub fn test_database() -> MockDatabase {
        let mut this = Self::new();
        let salt = hashing::generate_salt();
        let ip_addresses = [IpAddress {
            id: 1,
            user_id: 1,
            ip_address: "127.0.0.1".into(),
            last_used: Utc::now().naive_utc(),
        }];
        let managed_users = [ManagedUser {
            id: 1,
            salt: salt.clone(),
            email: "test@test.net".into(),
            username: "test".into(),
            login_attempts: 0,
            blocked_on: None,
            shadow: 1,
        }];
        let users = [User {
            id: 1,
            uuid: "testtesttesttest".into(),
            managed: Some(1),
        }];
        let shadow = [Shadow {
            id: 1,
            hashed_password: hashing::hash_password(b"testing".as_ref(), &salt).unwrap(),
        }];

        this.ip_addresses.get_mut().unwrap().extend(ip_addresses);
        this.managed_users.get_mut().unwrap().extend(managed_users);
        this.users.get_mut().unwrap().extend(users);
        this.shadow.get_mut().unwrap().extend(shadow);
        *this.global_id.get_mut() = 2;
        this
    }

    #[inline]
    fn global_id(&self) -> i32 {
        self.global_id.fetch_add(1, SeqCst)
    }
}

#[async_trait::async_trait]
impl Database for MockDatabase {
    #[inline]
    async fn fetch_user_by_uuid(&self, uuid: String) -> Result<User, DatabaseError> {
        let users = self.users.lock().unwrap();
        users
            .iter()
            .find(|u| u.uuid == uuid)
            .cloned()
            .ok_or(DatabaseError::NotFound)
    }

    #[inline]
    async fn fetch_user_by_username_or_email(
        &self,
        query: String,
    ) -> Result<(User, ManagedUser), DatabaseError> {
        let users = self.users.lock().unwrap();
        let managed_users = self.managed_users.lock().unwrap();

        let mu = managed_users
            .iter()
            .find(|mu| mu.username == query || mu.email == query)
            .ok_or(DatabaseError::NotFound)?
            .clone();
        let user = users
            .iter()
            .find(|u| u.managed == Some(mu.id))
            .ok_or(DatabaseError::NotFound)?
            .clone();
        Ok((user, mu))
    }

    #[inline]
    async fn fetch_user_password(
        &self,
        mu: ManagedUser,
        ip_address: String,
    ) -> Result<(Vec<u8>, i32), DatabaseError> {
        let managed_users = self.managed_users.lock().unwrap();
        let mut ip_addresses = self.ip_addresses.lock().unwrap();
        let shadow = self.shadow.lock().unwrap();

        match ip_addresses
            .iter_mut()
            .find(|ip| ip.ip_address == ip_address && ip.user_id == mu.id)
        {
            Some(ip) => {
                ip.last_used = Utc::now().naive_utc();
            }
            None => {
                ip_addresses.push(IpAddress {
                    id: self.global_id(),
                    user_id: mu.id,
                    ip_address,
                    last_used: Utc::now().naive_utc(),
                });
            }
        }

        let pwd = shadow
            .iter()
            .find(|s| mu.shadow == s.id)
            .ok_or(DatabaseError::NotFound)?
            .hashed_password
            .clone();
        let last_used = managed_users
            .iter()
            .find(|m| m.id == mu.id)
            .ok_or(DatabaseError::NotFound)?
            .login_attempts;
        Ok((pwd, last_used))
    }
}

#[cfg(test)]
mod test {
    use super::MockDatabase;
    use crate::{hashing, query::Database};

    #[tokio::test]
    async fn by_uuid() {
        let db = MockDatabase::test_database();
        let user = db
            .fetch_user_by_uuid("testtesttesttest".into())
            .await
            .unwrap();
        assert_eq!(user.id, 1);
    }

    #[tokio::test]
    async fn by_username() {
        let db = MockDatabase::test_database();
        let (user, muser) = db
            .fetch_user_by_username_or_email("test".into())
            .await
            .unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(muser.email, "test@test.net");
    }

    #[tokio::test]
    async fn by_email() {
        let db = MockDatabase::test_database();
        let (user, muser) = db
            .fetch_user_by_username_or_email("test@test.net".into())
            .await
            .unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(muser.username, "test");
    }

    #[tokio::test]
    async fn pwd_hash() {
        let db = MockDatabase::test_database();
        let (_, muser) = db
            .fetch_user_by_username_or_email("test@test.net".into())
            .await
            .unwrap();
        let (hashed, login_attempts) = db
            .fetch_user_password(muser, "127.0.0.1".into())
            .await
            .unwrap();
        assert!(hashing::verify_password(b"testing".as_ref(), &hashed).is_ok());
        assert_eq!(login_attempts, 0);
    }
}
