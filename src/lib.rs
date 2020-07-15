use async_session::{uuid::Uuid, Session, SessionStore};
use http_types::cookies::Cookie;
use redis::AsyncCommands;
use redis::{Client, RedisError};
use std::time::Duration;

#[derive(Clone)]
pub struct RedisSessionStore {
    client: Client,
    ttl: Duration,
}

impl RedisSessionStore {
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            ttl: Duration::from_secs(86400),
        }
    }

    pub fn new(connection_info: impl redis::IntoConnectionInfo) -> Result<Self, RedisError> {
        Ok(Self::from_client(Client::open(connection_info)?))
    }
}

#[async_trait::async_trait]
impl SessionStore for RedisSessionStore {
    type Error = Error;

    async fn load_session(&self, cookie: Cookie<'_>) -> Result<Option<Session>, Self::Error> {
        let mut connection = self.client.get_async_std_connection().await?;
        let value: String = connection.get(cookie.value()).await?;
        let session: Session = serde_json::from_str(&value)?;
        Ok(Some(session))
    }

    async fn store_session(&self, session: Session) -> Result<String, Self::Error> {
        let id = session.id();
        let mut connection = self.client.get_async_std_connection().await?;
        let string = serde_json::to_string(&session)?;

        let _: () = connection
            .set_ex(&id, string, self.ttl.as_secs() as usize)
            .await?;

        Ok(id)
    }

    async fn create_session(&self) -> Result<Session, Self::Error> {
        let sess = Session::new();
        sess.insert("id".to_string(), Uuid::new_v4().to_string());
        Ok(sess)
    }
}

#[derive(Debug)]
pub enum Error {
    RedisError(RedisError),
    SerdeError(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RedisError(e) => e.fmt(f),
            Error::SerdeError(e) => e.fmt(f),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl From<RedisError> for Error {
    fn from(e: RedisError) -> Self {
        Self::RedisError(e)
    }
}

impl std::error::Error for Error {}
