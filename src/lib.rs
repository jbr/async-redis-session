//! # async-redis-session
//! ```rust
//! use async_redis_session::RedisSessionStore;
//! use async_session::{Session, SessionStore};
//!
//! # fn main() -> async_session::Result { async_std::task::block_on(async {
//! let store = RedisSessionStore::new("redis://127.0.0.1/")?;
//!
//! let session = Session::new();
//! session.insert("key".into(), "value".into());
//!
//! let cookie_value = store.store_session(session).await.unwrap();
//! let session = store.load_session(cookie_value).await.unwrap();
//! assert_eq!(session.get("key").unwrap(), "value");
//! # Ok(()) }) }
//! ```

#![forbid(unsafe_code, future_incompatible)]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    missing_docs,
    unreachable_pub,
    missing_copy_implementations,
    unused_qualifications
)]

use async_session::{async_trait, serde_json, Result, Session, SessionStore};
use redis::{aio::Connection, AsyncCommands, Client, IntoConnectionInfo, RedisResult};

/// # RedisSessionStore
#[derive(Clone, Debug)]
pub struct RedisSessionStore {
    client: Client,
    prefix: Option<String>,
}

impl RedisSessionStore {
    /// creates a redis store from an existing [`redis::Client`]
    /// ```rust
    /// # use async_redis_session::RedisSessionStore;
    /// let client = redis::Client::open("redis://127.0.0.1").unwrap();
    /// let store = RedisSessionStore::from_client(client);
    /// ```
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            prefix: None,
        }
    }

    /// creates a redis store from a [`redis::IntoConnectionInfo`]
    /// such as a [`String`], [`&str`](str), or [`Url`](../url/struct.Url.html)
    /// ```rust
    /// # use async_redis_session::RedisSessionStore;
    /// let store = RedisSessionStore::new("redis://127.0.0.1").unwrap();
    /// ```
    pub fn new(connection_info: impl IntoConnectionInfo) -> RedisResult<Self> {
        Ok(Self::from_client(Client::open(connection_info)?))
    }

    /// sets a key prefix for this session store
    ///
    /// ```rust
    /// # use async_redis_session::RedisSessionStore;
    /// let store = RedisSessionStore::new("redis://127.0.0.1").unwrap()
    ///     .with_prefix("async-sessions/");
    /// ```
    /// ```rust
    /// # use async_redis_session::RedisSessionStore;
    /// let client = redis::Client::open("redis://127.0.0.1").unwrap();
    /// let store = RedisSessionStore::from_client(client)
    ///     .with_prefix("async-sessions/");
    /// ```
    pub fn with_prefix(mut self, prefix: impl AsRef<str>) -> Self {
        self.prefix = Some(prefix.as_ref().to_owned());
        self
    }

    fn prefix_key(&self, key: impl AsRef<str>) -> String {
        if let Some(ref prefix) = self.prefix {
            format!("{}{}", prefix, key.as_ref())
        } else {
            key.as_ref().into()
        }
    }

    async fn connection(&self) -> RedisResult<Connection> {
        self.client.get_async_std_connection().await
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn load_session(&self, cookie_value: String) -> Option<Session> {
        let id = Session::id_from_cookie_value(&cookie_value).ok()?;
        let mut connection = self.connection().await.ok()?;
        let record: Option<String> = connection.get(id).await.ok()?;
        match record {
            Some(value) => serde_json::from_str(&value).ok()?,
            None => None,
        }
    }

    async fn store_session(&self, session: Session) -> Option<String> {
        let id = session.id();
        let string = serde_json::to_string(&session).ok()?;

        let mut connection = self.connection().await.ok()?;

        match session.expires_in() {
            None => connection.set(id, string).await.ok()?,

            Some(expiry) => connection
                .set_ex(id, string, expiry.as_secs() as usize)
                .await
                .ok()?,
        };

        session.into_cookie_value()
    }

    async fn destroy_session(&self, session: Session) -> Result {
        let mut connection = self.connection().await?;
        let key = self.prefix_key(session.id().to_string());
        connection.del(key).await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result {
        self.connection().await?.del(self.prefix_key("*")).await?;
        Ok(())
    }
}
