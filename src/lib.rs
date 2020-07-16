use async_session::{async_trait, base64, serde_json, Session, SessionStore};
use redis::{AsyncCommands, Client, RedisError};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RedisSessionStore {
    client: Client,
    ttl: Duration,
    prefix: Option<String>,
}

impl RedisSessionStore {
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            ttl: Duration::from_secs(86400),
            prefix: None,
        }
    }

    pub fn new(connection_info: impl redis::IntoConnectionInfo) -> Result<Self, RedisError> {
        Ok(Self::from_client(Client::open(connection_info)?))
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    fn prefix_key(&self, key: impl AsRef<str>) -> String {
        if let Some(ref prefix) = self.prefix {
            format!("{}{}", prefix, key.as_ref())
        } else {
            key.as_ref().into()
        }
    }

    async fn connection(&self) -> redis::RedisResult<redis::aio::Connection> {
        self.client.get_async_std_connection().await
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    type Error = Error;

    async fn load_session(&self, cookie_value: String) -> Option<Session> {
        let id = Session::id_from_cookie_value(&cookie_value).ok()?;
        let mut connection = self.connection().await.ok()?;
        match connection.get::<_, Option<String>>(id).await.ok()? {
            Some(value) => serde_json::from_str(&value).ok()?,
            None => None,
        }
    }

    async fn store_session(&self, mut session: Session) -> Option<String> {
        let id = session.id();
        let string = serde_json::to_string(&session).ok()?;

        let mut connection = self.connection().await.ok()?;
        connection
            .set_ex::<_, _, ()>(id, string, self.ttl.as_secs() as usize)
            .await
            .ok()?;

        session.take_cookie_value()
    }

    async fn destroy_session(&self, session: Session) -> Result<(), Self::Error> {
        self.connection()
            .await?
            .del::<_, ()>(self.prefix_key(session.id().to_string()))
            .await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result<(), Self::Error> {
        self.connection().await?.del(self.prefix_key("*")).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    RedisError(RedisError),
    SerdeError(serde_json::Error),
    Base64Error(base64::DecodeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RedisError(e) => e.fmt(f),
            Error::SerdeError(e) => e.fmt(f),
            Error::Base64Error(e) => e.fmt(f),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Self::Base64Error(e)
    }
}

impl From<RedisError> for Error {
    fn from(e: RedisError) -> Self {
        Self::RedisError(e)
    }
}

impl std::error::Error for Error {}
