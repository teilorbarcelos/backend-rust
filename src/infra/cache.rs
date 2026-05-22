use crate::errors::AppError;
use deadpool_redis::{Config, Connection, Pool, Runtime};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Cache {
    pub pool: Pool,
}

impl Cache {
    pub fn new(redis_url: &str) -> Self {
        let cfg = Config::from_url(redis_url.to_string());

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Falha ao criar o pool do Redis");

        Self { pool }
    }

    async fn get_conn(&self) -> Result<Connection, AppError> {
        self.pool
            .get()
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao obter conexão do Redis: {}", e)))
    }

    pub async fn create_session(
        &self,
        user_id: &str,
        token: &str,
        expires_sec: i64,
    ) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let key = format!("session:{}:{}", user_id, token);

        redis::cmd("SET")
            .arg(&key)
            .arg("active")
            .arg("EX")
            .arg(expires_sec)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao salvar sessão: {}", e)))?;

        Ok(())
    }

    pub async fn validate_session(&self, user_id: &str, token: &str) -> Result<bool, AppError> {
        let mut conn = self.get_conn().await?;
        let key = format!("session:{}:{}", user_id, token);

        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);

        Ok(exists)
    }

    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let pattern = format!("session:{}*", user_id);

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .unwrap_or_default();

        if !keys.is_empty() {
            let mut del_cmd = redis::cmd("DEL");
            for key in keys {
                del_cmd.arg(key);
            }
            #[cfg(test)]
            let res = if user_id.contains("FORCE_DEL_ERROR") {
                Err(redis::RedisError::from((
                    redis::ErrorKind::ResponseError,
                    "Forced DEL error",
                )))
            } else {
                del_cmd.query_async::<_, ()>(&mut conn).await
            };
            #[cfg(not(test))]
            let res = del_cmd.query_async::<_, ()>(&mut conn).await;

            let _: () = res.map_err(|e| {
                AppError::Internal(format!("Erro ao expirar sessões antigas: {}", e))
            })?;
        }

        Ok(())
    }

    pub async fn delete_session(&self, user_id: &str, token: &str) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let key = format!("session:{}:{}", user_id, token);
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao deletar sessão: {}", e)))?;
        Ok(())
    }

    pub async fn check_rate_limit(
        &self,
        rate_key: &str,
        limit: i64,
        window_sec: i64,
    ) -> Result<(bool, i64, i64), AppError> {
        let mut conn = self.get_conn().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let clear_before = now - (window_sec * 1000);
        let redis_key = format!("ratelimit:{}", rate_key);

        let _: () = redis::pipe()
            .atomic()
            .cmd("ZREMRANGEBYSCORE")
            .arg(&redis_key)
            .arg("-inf")
            .arg(clear_before)
            .cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro no Rate Limiter: {}", e)))?;

        let count: i64 = redis::cmd("ZCARD")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        if count >= limit {
            return Ok((false, 0, limit));
        }

        let _: () = redis::pipe()
            .atomic()
            .cmd("ZADD")
            .arg(&redis_key)
            .arg(now)
            .arg(now)
            .cmd("EXPIRE")
            .arg(&redis_key)
            .arg(window_sec)
            .query_async(&mut conn)
            .await
            .unwrap_or(());

        let remaining = limit - count - 1;
        Ok((true, remaining.max(0), limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_exceeded() {
        dotenvy::dotenv().ok();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let cache = Cache::new(&redis_url);

        let key = format!("test_rate_limit_exceeded_key_{}", uuid::Uuid::new_v4());

        let (allowed1, remaining1, limit1) = cache.check_rate_limit(&key, 1, 10).await.unwrap();
        assert!(allowed1);
        assert_eq!(remaining1, 0);
        assert_eq!(limit1, 1);

        let (allowed2, remaining2, limit2) = cache.check_rate_limit(&key, 1, 10).await.unwrap();
        assert!(!allowed2);
        assert_eq!(remaining2, 0);
        assert_eq!(limit2, 1);
    }

    #[tokio::test]
    async fn test_invalidate_user_sessions_error() {
        let dead_cache = Cache::new("redis://127.0.0.1:9999");
        let user_id = format!("test-err-{}", uuid::Uuid::new_v4());
        let res = dead_cache.invalidate_user_sessions(&user_id).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_invalidate_user_sessions_del_error() {
        dotenvy::dotenv().ok();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let cache = Cache::new(&redis_url);

        let user_id = format!("test-del-err-FORCE_DEL_ERROR-{}", uuid::Uuid::new_v4());
        cache
            .create_session(&user_id, "token123", 10)
            .await
            .unwrap();

        let res = cache.invalidate_user_sessions(&user_id).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .message()
            .contains("Erro ao expirar sessões antigas"));
    }
}
