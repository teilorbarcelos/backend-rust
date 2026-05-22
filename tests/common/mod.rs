use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    Router,
};
use backend_rust::{
    config::AppConfig,
    infra::{bootstrap::bootstrap_database, cache::Cache, database},
    middleware, modules,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use tower::ServiceExt;

pub struct TestContext {
    pub db: DatabaseConnection,
    pub cache: Cache,
    pub config: AppConfig,
    pub router: Router,
}

impl TestContext {
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();

        config.jwt_expires_in = 3600;

        let db = database::connect(&config.database_url)
            .await
            .expect("Failed to connect to test Postgres database");

        bootstrap_database(&db)
            .await
            .expect("Failed to bootstrap test database");

        let cache = Cache::new(&config.redis_url);

        let mut test_config = config.clone();
        test_config.messaging_enabled = false;
        test_config.environment = "development".to_string();

        let _ = backend_rust::infra::messaging::MessagingProvider::init(&test_config).await;

        let api_router = modules::app_router(db.clone(), cache.clone(), test_config.clone());
        let obs_router = modules::observability::router(db.clone(), cache.clone());

        let router = Router::new()
            .merge(api_router)
            .merge(obs_router)
            .layer(axum::middleware::from_fn(
                modules::observability::track_metrics_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                db.clone(),
                middleware::error_log::error_logging_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                db.clone(),
                middleware::audit::audit_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                cache.clone(),
                middleware::rate_limit::rate_limit_middleware,
            ))
            .layer(axum::middleware::from_fn(
                middleware::request_log::request_logging_middleware,
            ));

        Self {
            db,
            cache,
            config: test_config,
            router,
        }
    }

    pub async fn clear_database(&self) {
        let statements = vec![
            "TRUNCATE TABLE public.\"Product\", audit.tb_audit, audit.tb_error_log CASCADE;",
            "DELETE FROM public.\"RoleFeature\" WHERE id_role != 'administrator';",
            "DELETE FROM public.\"User\" WHERE email != 'admin@email.com';",
            "DELETE FROM public.\"Auth\" WHERE id != 'auth-admin-uuid-00000000000000000001';",
            "DELETE FROM public.\"Role\" WHERE id != 'administrator';",

            "UPDATE public.\"RoleFeature\" SET \"create\" = true, \"view\" = true, \"activate\" = true, \"delete\" = true WHERE id_role = 'administrator';",
            "UPDATE public.\"Role\" SET active = true, name = 'Administrador', description = 'Perfil com acesso total ao sistema', is_deleted = false, deleted_at = NULL WHERE id = 'administrator';",
            "UPDATE public.\"User\" SET active = true, is_deleted = false, deleted_at = NULL, name = 'Supreme Administrator' WHERE email = 'admin@email.com';",
            "UPDATE public.\"Auth\" SET active = true, is_deleted = false, deleted_at = NULL WHERE id = 'auth-admin-uuid-00000000000000000001';",
        ];

        for stmt in statements {
            let res = self
                .db
                .execute(Statement::from_string(
                    self.db.get_database_backend(),
                    stmt.to_string(),
                ))
                .await;
            if let Err(e) = res {
                eprintln!(
                    "Warning: database cleanup statement failed: {}. Query: {}",
                    e, stmt
                );
            }
        }
    }

    pub async fn reset_rate_limiter(&self) {
        let mut conn = self
            .cache
            .pool
            .get()
            .await
            .expect("Failed to connect to Redis for reset");
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("rate_limit:*")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        let keys_no_underscore: Vec<String> = redis::cmd("KEYS")
            .arg("ratelimit:*")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        let all_keys = [keys, keys_no_underscore].concat();
        if !all_keys.is_empty() {
            let mut del_cmd = redis::cmd("DEL");
            for key in &all_keys {
                del_cmd.arg(key);
            }
            let _: () = del_cmd.query_async(&mut conn).await.unwrap_or_default();
        }
    }
}

pub struct TestClient {
    router: Router,
    token: Option<String>,
}

impl TestClient {
    pub fn new(router: Router) -> Self {
        Self {
            router,
            token: None,
        }
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub async fn request(
        &mut self,
        method: &str,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
    ) -> (StatusCode, Response<Body>) {
        let mut req = Request::builder().method(method).uri(uri);

        if let Some(ref t) = self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
        }

        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }

        let req = req.body(body).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        (resp.status(), resp)
    }

    pub async fn request_with_headers(
        &mut self,
        method: &str,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
        extra_headers: Vec<(&str, &str)>,
    ) -> (StatusCode, Response<Body>) {
        let mut req = Request::builder().method(method).uri(uri);

        if let Some(ref t) = self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
        }

        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }

        for (k, v) in extra_headers {
            req = req.header(k, v);
        }

        let req = req.body(body).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        (resp.status(), resp)
    }

    pub async fn get(&mut self, uri: &str) -> (StatusCode, Response<Body>) {
        self.request("GET", uri, Body::empty(), None).await
    }

    pub async fn post_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request(
            "POST",
            uri,
            Body::from(body_bytes),
            Some("application/json"),
        )
        .await
    }

    pub async fn put_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request("PUT", uri, Body::from(body_bytes), Some("application/json"))
            .await
    }

    pub async fn patch_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request(
            "PATCH",
            uri,
            Body::from(body_bytes),
            Some("application/json"),
        )
        .await
    }

    pub async fn delete(&mut self, uri: &str) -> (StatusCode, Response<Body>) {
        self.request("DELETE", uri, Body::empty(), None).await
    }
}

pub async fn read_body_json(resp: Response<Body>) -> serde_json::Value {
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null)
}

pub async fn read_body_string(resp: Response<Body>) -> String {
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body_bytes.to_vec()).unwrap()
}
