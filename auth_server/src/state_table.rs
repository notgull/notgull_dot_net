// GNU AGPL v3 License

use crate::AuthData;
use dashmap::{mapref::entry::Entry, DashMap};
use once_cell::sync::OnceCell;
use std::time::{Duration, Instant};
use tokio::time::interval;

// the global state table
static STATE_TABLE: OnceCell<StateTable> = OnceCell::new();

#[derive(Default)]
struct StateTable {
    entries: DashMap<String, StateTableEntry>,
    // maps state entries to auth token entries; both should be unique anyways
    // ensures state lookup is O(1)
    statemap: DashMap<String, String>,
}

#[inline]
pub fn intiailize_state_table() {
    let _ = STATE_TABLE.set(StateTable::default());
}

/// An entry in the state table, keeping track of ongoing authentication.
struct StateTableEntry {
    // state token
    state: String,
    // auth token, this is what we're keyed by, but it's good to have it in one place
    auth_token: String,
    // scopes of the operation
    scopes: String,
    // client ID
    client_id: String,
    // redirect URI
    redirect_uri: String,

    // authorization data
    auth_data: Option<AuthData>,

    expiry: Instant,
}

/// Store an entry in the state table.
#[inline]
pub fn store_entry(
    auth_token: String,
    state: String,
    scopes: String,
    client_id: String,
    redirect_uri: String,
) {
    let entry = StateTableEntry {
        auth_token: auth_token.clone(),
        state: state.clone(),
        scopes,
        client_id,
        redirect_uri,
        auth_data: None,
        expiry: Instant::now() + Duration::from_secs(60 * 60),
    };

    let table = STATE_TABLE.get().unwrap();
    table.entries.insert(auth_token.clone(), entry);
    table.statemap.insert(state, auth_token);
}

/// Add authorization data into the state table, returns the auth code.
#[inline]
pub fn add_entry_auth_data(state: String, auth_data: AuthData) -> Result<String, RejectError> {
    let table = STATE_TABLE.get().unwrap();
    let auth_token = table
        .statemap
        .get(&state)
        .ok_or(RejectError::NotLoggedIn)?
        .clone();
    let mut entry = table.entries.get_mut(&auth_token).unwrap();
    entry.auth_data = Some(auth_data);
    Ok(entry.auth_token.clone())
}

/// Check an auth token, state, client ID and redirect URI against the table.
#[inline]
pub fn check_entry(
    auth_token: String,
    client_id: String,
    redirect_uri: String,
) -> Result<AuthData, RejectError> {
    let table = STATE_TABLE.get().unwrap();
    let value = table
        .entries
        .get(&auth_token)
        .ok_or(RejectError::EntryNotFound)?;

    // check the entry's properties
    if value.client_id != client_id {
        return Err(RejectError::ClientIdMismatch);
    } else if value.redirect_uri != redirect_uri {
        return Err(RejectError::RedirectUriMistmatch);
    } else if value.expiry < Instant::now() {
        table.statemap.remove(&value.state);
        std::mem::drop(value);
        table.entries.remove(&auth_token);
        return Err(RejectError::Expired);
    }

    // entry has been fully checked, return the auth data
    table.statemap.remove(&value.state);
    std::mem::drop(value);
    let value = table
        .entries
        .remove(&auth_token)
        .ok_or(RejectError::EntryNotFound)?
        .1;
    value.auth_data.ok_or(RejectError::NotLoggedIn)
}

#[inline]
pub async fn regularly_clear_expired() {
    let mut ticker = interval(Duration::from_secs(60 * 60));
    loop {
        ticker.tick().await;
        clear_expired_entries();
    }
}

#[inline]
fn clear_expired_entries() {
    let now = Instant::now();
    STATE_TABLE
        .get()
        .unwrap()
        .entries
        .retain(|_, val| val.expiry > now);
}

#[derive(Debug, thiserror::Error)]
pub enum RejectError {
    #[error("Unable to find state table entry")]
    EntryNotFound,
    #[error("State does not match the entry state")]
    StateMismatch,
    #[error("Client ID does not match the entry's state")]
    ClientIdMismatch,
    #[error("Redirect URI does not match the entry's state")]
    RedirectUriMistmatch,
    #[error("User has not yet logged in")]
    NotLoggedIn,
    #[error("State table entry has expired")]
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_test() {
        intiailize_state_table();
        store_entry(
            "test1".into(),
            "test2".into(),
            "test3".into(),
            "test4".into(),
            "test5".into(),
        );
        let entry = STATE_TABLE.get().unwrap().entries.get("test1").unwrap();

        assert_eq!(entry.auth_token, "test1");
        assert_eq!(entry.state, "test2");
        assert_eq!(entry.scopes, "test3");
        assert_eq!(entry.client_id, "test4");
        assert_eq!(entry.redirect_uri, "test5");

        assert_eq!(
            *STATE_TABLE.get().unwrap().statemap.get("test2").unwrap(),
            "test1"
        );
    }
}
