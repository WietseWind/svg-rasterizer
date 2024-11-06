use std::time::Duration;
use redis::AsyncCommands;
use crate::error::{ServiceResult, ServiceError};

#[derive(Clone)]
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    pub fn new(redis_url: &str) -> ServiceResult<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| ServiceError::CacheError(format!("Failed to create Redis client: {}", e)))?;
        Ok(Self { client })
    }

    pub async fn initialize(&self) -> ServiceResult<()> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to connect to Redis: {}", e)))?;
            
        // Try a PING to verify connection
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| ServiceError::CacheError(format!("Redis PING failed: {}", e)))?;
            
        Ok(())
    }

    pub async fn get(&self, key: &str) -> ServiceResult<Option<Vec<u8>>> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
            
        conn.get(key)
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get key {}: {}", key, e)))
    }

    pub async fn set(&self, key: &str, value: &[u8], expiry: Duration) -> ServiceResult<()> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
            
        conn.set_ex(key, value, expiry.as_secs() as usize)
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to set key {}: {}", key, e)))
    }

    pub async fn increment_counter(&self, key: &str, window: Duration) -> ServiceResult<i32> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to get Redis connection: {}", e)))?;
            
        let count: i32 = redis::pipe()
            .atomic()
            .incr(key, 1)
            .expire(key, window.as_secs() as usize)
            .query_async(&mut conn)
            .await
            .map_err(|e| ServiceError::CacheError(format!("Failed to increment counter {}: {}", key, e)))?;
            
        Ok(count)
    }

    pub async fn check_connection(&self) -> ServiceResult<()> {
        let mut conn = self.client.get_async_connection()
            .await
            .map_err(|e| ServiceError::CacheError(format!("Redis connection failed: {}", e)))?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| ServiceError::CacheError(format!("Redis PING failed: {}", e)))?;

        Ok(())
    }
}