use std::time::Duration;

use rand::Rng;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};

pub const CACHE_NULL_VALUE: &str = "__NULL__";
pub const CACHE_EXPIRED_VALUE: &str = "__EXPIRED__";

#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
    ttl_seconds: u64,
    null_ttl_seconds: u64,
    jitter_seconds: u64,
}

impl RedisCache {
    pub async fn new(
        url: &str,
        ttl_seconds: u64,
        null_ttl_seconds: u64,
        jitter_seconds: u64,
    ) -> Result<Self, redis::RedisError> {
        let client = Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self {
            manager,
            ttl_seconds,
            null_ttl_seconds,
            jitter_seconds,
        })
    }

    pub async fn get_string(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.manager.clone();
        conn.get(key).await
    }

    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let ttl = self.with_jitter(self.ttl_seconds);
        let payload = serde_json::to_string(value).unwrap_or_default();
        let _: () = redis::cmd("SETEX").arg(key).arg(ttl).arg(payload).query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn set_string(&self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let ttl = self.with_jitter(self.ttl_seconds);
        let _: () = redis::cmd("SETEX").arg(key).arg(ttl).arg(value).query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn set_null(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let ttl = self.with_jitter(self.null_ttl_seconds);
        let _: () = redis::cmd("SETEX")
            .arg(key)
            .arg(ttl)
            .arg(CACHE_NULL_VALUE)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn set_expired(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let ttl = self.with_jitter(self.null_ttl_seconds);
        let _: () = redis::cmd("SETEX")
            .arg(key)
            .arg(ttl)
            .arg(CACHE_EXPIRED_VALUE)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn del(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let _: () = conn.del(key).await?;
        Ok(())
    }

    pub async fn try_lock(&self, key: &str, ttl_ms: u64) -> Result<bool, redis::RedisError> {
        let mut conn = self.manager.clone();
        let result: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg("1")
            .arg("NX")
            .arg("PX")
            .arg(ttl_ms)
            .query_async(&mut conn)
            .await?;
        Ok(result.is_some())
    }

    pub async fn unlock(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.manager.clone();
        let _: () = conn.del(key).await?;
        Ok(())
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, redis::RedisError> {
        let mut conn = self.manager.clone();
        let cached: Option<String> = conn.get(key).await?;
        let Some(payload) = cached else {
            return Ok(None);
        };
        let value = serde_json::from_str::<T>(&payload).ok();
        Ok(value)
    }

    fn with_jitter(&self, base: u64) -> u64 {
        if self.jitter_seconds == 0 {
            return base;
        }
        let mut rng = rand::thread_rng();
        base + rng.gen_range(0..=self.jitter_seconds)
    }

    pub async fn sleep_backoff(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
}
