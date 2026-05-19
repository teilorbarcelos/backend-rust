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
            let _: () = del_cmd.query_async(&mut conn).await.map_err(|e| {
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
