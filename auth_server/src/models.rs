// GNU AGPL v3 License

use crate::schema::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Clone, Queryable)]
//#[table_name = "ipaddresses"]
pub struct IpAddress {
    pub id: i32,
    pub user_id: i32,
    pub ip_address: String,
    pub last_used: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "ipaddresses"]
pub struct NewIpAddress {
    pub user_id: i32,
    pub ip_address: String,
}

#[derive(AsChangeset)]
#[table_name = "ipaddresses"]
pub struct IpAddressChange {
    pub last_used: Option<NaiveDateTime>,
}

#[derive(Clone, Queryable)]
//#[table = "managedusers"]
pub struct ManagedUser {
    pub id: i32,
    pub salt: Vec<u8>,
    pub email: String,
    pub username: String,
    pub login_attempts: i32,
    pub blocked_on: Option<NaiveDateTime>,
    pub shadow: i32,
}

#[derive(Insertable)]
#[table_name = "managedusers"]
pub struct NewManagedUser {
    pub username: String,
    pub email: String,
    pub salt: Vec<u8>,
    pub shadow: i32,
}

#[derive(AsChangeset)]
#[table_name = "managedusers"]
pub struct ManagedUserChange {
    pub username: Option<String>,
    pub email: Option<String>,
    pub salt: Option<Vec<u8>>,
    pub login_attempts: Option<i32>,
    pub blocked_on: Option<Option<NaiveDateTime>>,
    pub shadow: Option<i32>,
}

#[derive(Clone, Queryable)]
//#[table_name = "shadow"]
pub struct Shadow {
    pub id: i32,
    pub hashed_password: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "shadow"]
pub struct NewShadow {
    pub hashed_password: Vec<u8>,
}

#[derive(AsChangeset)]
#[table_name = "shadow"]
pub struct ShadowChange {
    pub hashed_password: Option<Vec<u8>>,
}

#[derive(Clone, Queryable)]
//#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub uuid: String,
    pub managed: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub uuid: String,
    pub managed: Option<i32>,
}

#[derive(AsChangeset)]
#[table_name = "users"]
pub struct UserChange {
    pub uuid: Option<String>,
    pub managed: Option<Option<i32>>,
}
