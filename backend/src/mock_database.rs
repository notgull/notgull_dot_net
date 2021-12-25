// GNU AGPL v3 License

use crate::{
    models::{
        Blogpost, BlogpostChange, BlogpostFilter, NewBlogpost, NewUser, User, UserChange,
        UserFilter,
    },
    Database, DatabaseError,
};
use chrono::prelude::*;
use std::sync::{
    atomic::{AtomicI32, Ordering::SeqCst},
    Mutex,
};

macro_rules! apply_change {
    ($base: ident: $field: ident) => {
        if let Some(val) = $field {
            $base.$field = val;
        }
    };
    ($base: ident: $field: ident, $($tt: tt)*) => {
        apply_change!($base: $field);
        apply_change!($base: $($tt)*);
    }
}

/// Mock database used for basic testing.
pub struct MockDatabase {
    last_id: AtomicI32,
    blogposts: Mutex<Vec<Blogpost>>,
    users: Mutex<Vec<User>>,
}

impl MockDatabase {
    #[inline]
    pub fn new() -> Self {
        Self {
            last_id: AtomicI32::new(1),
            blogposts: Mutex::new(Vec::new()),
            users: Mutex::new(Vec::new()),
        }
    }

    #[inline]
    fn next_id(&self) -> i32 {
        self.last_id.fetch_add(1, SeqCst)
    }

    #[inline]
    fn get_blogpost_by(
        &self,
        mut f: impl FnMut(&Blogpost) -> bool,
    ) -> Result<Blogpost, DatabaseError> {
        self.blogposts
            .lock()
            .unwrap()
            .iter()
            .find(move |item| f(item))
            .cloned()
            .ok_or(DatabaseError::NotFound)
    }

    #[inline]
    fn get_user_by(&self, mut f: impl FnMut(&User) -> bool) -> Result<User, DatabaseError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(move |item| f(item))
            .cloned()
            .ok_or(DatabaseError::NotFound)
    }

    #[inline]
    pub fn with_test_data() -> Self {
        // test data includes two users:
        //  - John Notgull
        //  - Alan Smithee
        // and two blogposts
        let user1 = User {
            id: 1,
            uuid: "65a7e8c5-c235-49a9-ba00-6d9c049776f4".into(),
            name: "John Notgull".into(),
            roles: 0xFFFFFFFF,
        };
        let user2 = User {
            id: 2,
            uuid: "995a066d-de0e-4378-92e6-407f7aa1dc19".into(),
            name: "Alan Smithee".into(),
            roles: 0,
        };

        let blog1 = Blogpost {
            id: 1,
            title: "Chasing Suns".into(),
            tags: "story,humurous,funny".into(),
            url: "chasing-suns".into(),
            body: "...and we spent so much time chasing suns, we forgot what we were really after."
                .into(),
            author_id: 1,
            created_at: Local::now().naive_local(),
        };
        let blog2 = Blogpost {
            id: 2,
            title: "How to make a website".into(),
            tags: "tutorial,technical,funny".into(),
            url: "how-to-make-a-website".into(),
            body: r#"
Hello, I am John Notgull. *What* **if** we made a website?

- It'd be cool.
- It'd be neat.
- Why not?
            "#
            .into(),
            author_id: 1,
            created_at: Local::now().naive_local(),
        };

        let mut this = Self::new();
        this.users.get_mut().unwrap().extend([user1, user2]);
        this.blogposts.get_mut().unwrap().extend([blog1, blog2]);
        *this.last_id.get_mut() = 3;
        this
    }
}

#[async_trait::async_trait]
impl Database for MockDatabase {
    #[inline]
    async fn get_blogpost_by_id(&self, sid: i32) -> Result<Blogpost, DatabaseError> {
        self.get_blogpost_by(|b| b.id == sid)
    }

    #[inline]
    async fn get_blogpost_and_user_by_url(
        &self,
        surl: String,
    ) -> Result<(Blogpost, User), DatabaseError> {
        let blogpost = self.get_blogpost_by(|b| b.url == surl)?;
        let user = self.get_user_by(|u| u.id == blogpost.author_id)?;
        Ok((blogpost, user))
    }

    #[inline]
    async fn insert_blogpost(&self, bp: NewBlogpost) -> Result<i32, DatabaseError> {
        let NewBlogpost {
            title,
            tags,
            url,
            body,
            author_id,
        } = bp;
        let id = self.next_id();
        let blogpost = Blogpost {
            id,
            title,
            tags,
            url,
            body,
            author_id,
            created_at: Local::now().naive_local(),
        };
        self.blogposts.lock().unwrap().push(blogpost);
        Ok(id)
    }

    #[inline]
    async fn update_blogpost(&self, id: i32, bp: BlogpostChange) -> Result<(), DatabaseError> {
        let mut blogposts = self.blogposts.lock().unwrap();
        let blogpost = blogposts
            .iter_mut()
            .find(|bp| bp.id == id)
            .ok_or(DatabaseError::NotFound)?;
        let BlogpostChange {
            title,
            tags,
            url,
            body,
            author_id,
        } = bp;
        apply_change!(blogpost: title, tags, url, body, author_id);

        Ok(())
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

        Ok(self
            .blogposts
            .lock()
            .unwrap()
            .iter()
            .filter(move |bp| {
                let mut cond = true;
                if let Some(title) = title.as_deref() {
                    cond = cond && bp.title.contains(title);
                }
                if let Some(tags) = tags.as_deref() {
                    cond = cond && bp.tags.contains(tags);
                }
                if let Some(url) = url.as_deref() {
                    cond = cond && bp.url.contains(url);
                }
                if let Some(body) = body.as_deref() {
                    cond = cond && bp.body.contains(body);
                }
                if let Some(author_id) = author_id {
                    cond = cond && bp.author_id == author_id;
                }
                cond
            })
            .cloned()
            .collect())
    }

    #[inline]
    async fn delete_blogpost(&self, sid: i32) -> Result<(), DatabaseError> {
        self.blogposts.lock().unwrap().retain(|bp| bp.id != sid);
        Ok(())
    }

    #[inline]
    async fn get_user_by_id(&self, sid: i32) -> Result<User, DatabaseError> {
        self.get_user_by(|user| user.id == sid)
    }

    #[inline]
    async fn get_user_by_uuid(&self, suuid: String) -> Result<User, DatabaseError> {
        self.get_user_by(|user| user.uuid == suuid)
    }

    #[inline]
    async fn insert_user(&self, user: NewUser) -> Result<i32, DatabaseError> {
        let NewUser { uuid, name, roles } = user;
        let id = self.next_id();
        let user = User {
            id,
            name,
            uuid,
            roles,
        };
        self.users.lock().unwrap().push(user);
        Ok(id)
    }

    #[inline]
    async fn update_user(&self, id: i32, user: UserChange) -> Result<(), DatabaseError> {
        let mut users = self.users.lock().unwrap();
        let UserChange { uuid, name, roles } = user;
        let user = users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or(DatabaseError::NotFound)?;
        apply_change!(user: uuid, name, roles);

        Ok(())
    }

    #[inline]
    async fn list_users(&self, filter: UserFilter) -> Result<Vec<User>, DatabaseError> {
        let UserFilter { name } = filter;

        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(move |user| {
                let mut cond = true;
                if let Some(name) = name.as_deref() {
                    cond = cond && user.name.contains(&name);
                }
                cond
            })
            .cloned()
            .collect())
    }

    #[inline]
    async fn delete_user(&self, sid: i32) -> Result<(), DatabaseError> {
        self.users.lock().unwrap().retain(|user| user.id != sid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MockDatabase;
    use crate::{
        models::{BlogpostChange, NewBlogpost, NewUser, UserChange},
        Database,
    };

    #[tokio::test]
    async fn get_blogpost_by_id() {
        let database = MockDatabase::with_test_data();
        assert_eq!(
            database.get_blogpost_by_id(1).await.unwrap().title,
            "Chasing Suns"
        );
        assert_eq!(
            database.get_blogpost_by_id(2).await.unwrap().title,
            "How to make a website"
        );
    }

    #[tokio::test]
    async fn get_blogpost_by_url() {
        let database = MockDatabase::with_test_data();
        assert_eq!(
            database
                .get_blogpost_and_user_by_url("chasing-suns".into())
                .await
                .unwrap()
                .0
                .title,
            "Chasing Suns"
        );
        assert_eq!(
            database
                .get_blogpost_and_user_by_url("how-to-make-a-website".into())
                .await
                .unwrap()
                .0
                .title,
            "How to make a website"
        );
    }

    #[tokio::test]
    async fn insert_blogpost() {
        let database = MockDatabase::with_test_data();
        let bp = NewBlogpost {
            title: "Breaking Bones".into(),
            tags: "we,break,bones".into(),
            url: "breaking-bones".into(),
            body: "I broke some bones today".into(),
            author_id: 1,
        };
        let id = database.insert_blogpost(bp).await.unwrap();
        assert_eq!(
            database.get_blogpost_by_id(id).await.unwrap().title,
            "Breaking Bones"
        );
    }

    #[tokio::test]
    async fn update_blogpost() {
        let database = MockDatabase::with_test_data();
        let bp = database
            .get_blogpost_and_user_by_url("chasing-suns".into())
            .await
            .unwrap()
            .0;

        let id = bp.id;
        let change = BlogpostChange {
            title: Some("Breaking Bones".into()),
            ..Default::default()
        };

        database.update_blogpost(id, change).await.unwrap();
        assert_eq!(
            database.get_blogpost_by_id(id).await.unwrap().title,
            "Breaking Bones"
        );
    }

    #[tokio::test]
    async fn delete_blogpost() {
        let database = MockDatabase::with_test_data();
        database.delete_blogpost(1).await.unwrap();
        assert!(database.get_blogpost_by_id(1).await.is_err())
    }

    #[tokio::test]
    async fn get_user_by_id() {
        let database = MockDatabase::with_test_data();
        assert_eq!(
            database.get_user_by_id(1).await.unwrap().name,
            "John Notgull"
        );
        assert_eq!(
            database.get_user_by_id(2).await.unwrap().name,
            "Alan Smithee"
        );
    }

    #[tokio::test]
    async fn get_user_by_uuid() {
        let database = MockDatabase::with_test_data();
        assert_eq!(
            database
                .get_user_by_uuid("65a7e8c5-c235-49a9-ba00-6d9c049776f4".into())
                .await
                .unwrap()
                .name,
            "John Notgull"
        );
        assert_eq!(
            database
                .get_user_by_uuid("995a066d-de0e-4378-92e6-407f7aa1dc19".into())
                .await
                .unwrap()
                .name,
            "Alan Smithee"
        );
    }

    #[tokio::test]
    async fn insert_user() {
        let database = MockDatabase::with_test_data();
        let bp = NewUser {
            name: "Shawn Spencer".into(),
            roles: 0,
            uuid: "50d36fd5-51d2-4d6a-b739-c0425de085ac".into(),
        };
        let id = database.insert_user(bp).await.unwrap();
        assert_eq!(
            database.get_user_by_id(id).await.unwrap().name,
            "Shawn Spencer"
        );
    }

    #[tokio::test]
    async fn update_user() {
        let database = MockDatabase::with_test_data();
        let bp = database
            .get_user_by_uuid("995a066d-de0e-4378-92e6-407f7aa1dc19".into())
            .await
            .unwrap();

        let id = bp.id;
        let change = UserChange {
            name: Some("Burton Guster".into()),
            ..Default::default()
        };

        database.update_user(id, change).await.unwrap();
        assert_eq!(
            database.get_user_by_id(id).await.unwrap().name,
            "Burton Guster"
        );
    }

    #[tokio::test]
    async fn delete_user() {
        let database = MockDatabase::with_test_data();
        database.delete_user(1).await.unwrap();
        assert!(database.get_user_by_id(1).await.is_err())
    }
}
