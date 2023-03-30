//! # async-redis-session
//! ```rust
//! use async_redis_session::{RedisSessionStore, Error};
//! use async_session::{Session, SessionStore};
//!
//! # fn main() -> Result<(), Error> { async_std::task::block_on(async {
//! let store = RedisSessionStore::new("redis://127.0.0.1/")?;
//!
//! let mut session = Session::new();
//! session.insert("key", "value")?;
//!
//! let cookie_value = store.store_session(&mut session).await?.unwrap();
//! let session = store.load_session(&cookie_value).await?.unwrap();
//! assert_eq!(&session.get::<String>("key").unwrap(), "value");
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

use async_session::{async_trait, Session, SessionStore};
use redis::{aio::Connection, AsyncCommands, Client, IntoConnectionInfo, RedisResult};

/// Errors that can arise in the operation of the session stores
/// included in this crate
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// an error that comes from sqlx
    #[error(transparent)]
    Redis(#[from] redis::RedisError),

    /// an error that comes from serde_json
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// an error that comes from base64
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
}

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

    async fn ids(&self) -> Result<Vec<String>, Error> {
        Ok(self.connection().await?.keys(self.prefix_key("*")).await?)
    }

    /// returns the number of sessions in this store
    pub async fn count(&self) -> Result<usize, Error> {
        if self.prefix.is_none() {
            let mut connection = self.connection().await?;
            Ok(redis::cmd("DBSIZE").query_async(&mut connection).await?)
        } else {
            Ok(self.ids().await?.len())
        }
    }

    #[cfg(test)]
    async fn ttl_for_session(&self, session: &Session) -> Result<usize, Error> {
        Ok(self
            .connection()
            .await?
            .ttl(self.prefix_key(session.id()))
            .await?)
    }

    fn prefix_key(&self, key: impl AsRef<str>) -> String {
        let key = key.as_ref();
        if let Some(ref prefix) = self.prefix {
            format!("{prefix}{key}")
        } else {
            key.to_string()
        }
    }

    async fn connection(&self) -> RedisResult<Connection> {
        self.client.get_async_connection().await
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    type Error = Error;

    async fn load_session(&self, cookie_value: &str) -> Result<Option<Session>, Self::Error> {
        let id = Session::id_from_cookie_value(cookie_value)?;
        let mut connection = self.connection().await?;
        let record: Option<String> = connection.get(self.prefix_key(id)).await?;
        match record {
            Some(value) => Ok(serde_json::from_str(&value)?),
            None => Ok(None),
        }
    }

    async fn store_session(&self, session: &mut Session) -> Result<Option<String>, Self::Error> {
        let id = self.prefix_key(session.id());
        let string = serde_json::to_string(&session)?;

        let mut connection = self.connection().await?;

        match session.expires_in() {
            None => connection.set(id, string).await?,

            Some(expiry) => {
                connection
                    .set_ex(id, string, expiry.as_secs() as usize)
                    .await?
            }
        };

        Ok(session.take_cookie_value())
    }

    async fn destroy_session(&self, session: &mut Session) -> Result<(), Self::Error> {
        let mut connection = self.connection().await?;
        let key = self.prefix_key(session.id());
        connection.del(key).await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result<(), Self::Error> {
        let mut connection = self.connection().await?;

        if self.prefix.is_none() {
            let _: () = redis::cmd("FLUSHDB").query_async(&mut connection).await?;
        } else {
            let ids = self.ids().await?;
            if !ids.is_empty() {
                connection.del(ids).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use std::time::Duration;

    async fn test_store() -> RedisSessionStore {
        let store = RedisSessionStore::new("redis://127.0.0.1").unwrap();
        store.clear_store().await.unwrap();
        store
    }

    #[async_std::test]
    async fn creating_a_new_session_with_no_expiry() -> Result<(), Error> {
        let store = test_store().await;
        let mut session = Session::new();
        session.insert("key", "value")?;
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let loaded_session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(session.id(), loaded_session.id());
        assert_eq!("value", &loaded_session.get::<String>("key").unwrap());

        assert!(!loaded_session.is_expired());
        Ok(())
    }

    #[async_std::test]
    async fn updating_a_session() -> Result<(), Error> {
        let store = test_store().await;
        let mut session = Session::new();

        session.insert("key", "value")?;
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let mut session = store.load_session(&cookie_value).await?.unwrap();
        session.insert("key", "other value")?;
        assert_eq!(None, store.store_session(&mut session).await?);

        let session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(&session.get::<String>("key").unwrap(), "other value");

        assert_eq!(1, store.count().await.unwrap());
        Ok(())
    }

    #[async_std::test]
    async fn updating_a_session_extending_expiry() -> Result<(), Error> {
        let store = test_store().await;
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(5));
        let original_expires = *session.expiry().unwrap();
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let mut session = store.load_session(&cookie_value).await?.unwrap();
        let ttl = store.ttl_for_session(&session).await?;
        assert!(ttl > 3 && ttl < 5);

        assert_eq!(session.expiry().unwrap(), &original_expires);
        session.expire_in(Duration::from_secs(10));
        let new_expires = *session.expiry().unwrap();
        store.store_session(&mut session).await?;

        let session = store.load_session(&cookie_value).await?.unwrap();
        let ttl = store.ttl_for_session(&session).await?;
        assert!(ttl > 8 && ttl < 10);
        assert_eq!(session.expiry().unwrap(), &new_expires);

        assert_eq!(1, store.count().await.unwrap());

        task::sleep(Duration::from_secs(10)).await;
        assert_eq!(0, store.count().await.unwrap());

        Ok(())
    }

    #[async_std::test]
    async fn creating_a_new_session_with_expiry() -> Result<(), Error> {
        let store = test_store().await;
        let mut session = Session::new();
        session.expire_in(Duration::from_secs(3));
        session.insert("key", "value")?;

        let cookie_value = store.store_session(&mut session).await?.unwrap();

        assert!(store.ttl_for_session(&session).await? > 1);

        let loaded_session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(session.id(), loaded_session.id());
        assert_eq!("value", &loaded_session.get::<String>("key").unwrap());

        assert!(!loaded_session.is_expired());

        task::sleep(Duration::from_secs(2)).await;
        assert_eq!(None, store.load_session(&cookie_value).await?);

        Ok(())
    }

    #[async_std::test]
    async fn destroying_a_single_session() -> Result<(), Error> {
        let store = test_store().await;
        for _ in 0..3i8 {
            store.store_session(&mut Session::new()).await?;
        }

        let cookie = store.store_session(&mut Session::new()).await?.unwrap();
        assert_eq!(4, store.count().await?);
        let mut session = store.load_session(&cookie).await?.unwrap();
        store.destroy_session(&mut session).await.unwrap();
        assert_eq!(None, store.load_session(&cookie).await?);
        assert_eq!(3, store.count().await?);

        // attempting to destroy the session again is not an error
        assert!(store.destroy_session(&mut session).await.is_ok());
        Ok(())
    }

    #[async_std::test]
    async fn clearing_the_whole_store() -> Result<(), Error> {
        let store = test_store().await;
        for _ in 0..3i8 {
            store.store_session(&mut Session::new()).await?;
        }

        assert_eq!(3, store.count().await?);
        store.clear_store().await.unwrap();
        assert_eq!(0, store.count().await?);

        Ok(())
    }

    #[async_std::test]
    async fn prefixes() -> Result<(), Error> {
        test_store().await; // clear the db

        let store = RedisSessionStore::new("redis://127.0.0.1")?.with_prefix("sessions/");
        store.clear_store().await?;

        for _ in 0..3i8 {
            store.store_session(&mut Session::new()).await?;
        }

        let mut session = Session::new();

        session.insert("key", "value")?;
        let cookie_value = store.store_session(&mut session).await?.unwrap();

        let mut session = store.load_session(&cookie_value).await?.unwrap();
        session.insert("key", "other value")?;
        assert_eq!(None, store.store_session(&mut session).await?);

        let session = store.load_session(&cookie_value).await?.unwrap();
        assert_eq!(&session.get::<String>("key").unwrap(), "other value");

        assert_eq!(4, store.count().await.unwrap());

        let other_store =
            RedisSessionStore::new("redis://127.0.0.1")?.with_prefix("other-namespace/");

        assert_eq!(0, other_store.count().await.unwrap());
        for _ in 0..3i8 {
            other_store.store_session(&mut Session::new()).await?;
        }

        other_store.clear_store().await?;

        assert_eq!(0, other_store.count().await?);
        assert_eq!(4, store.count().await?);

        Ok(())
    }
}
