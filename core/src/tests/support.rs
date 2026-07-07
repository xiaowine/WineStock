//! core 测试共享支持函数。
//!
//! 本文件属于测试辅助层，负责搭建本地 router、种子用户和常用 HTTP 请求助手。
//! 它不声明具体业务断言，具体行为由各测试文件自行验证。

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use sea_orm::DatabaseConnection;
use tempfile::{tempdir, TempDir};
use tower::ServiceExt;
use winestock_shared::{
    AppConfig, AuthClientKind, AuthLoginRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, RuntimeMode, ServerConfig, StorageConfig,
};

use crate::{
    bootstrap_from_config,
    persistence::repository::{CreateUser, RbacRepository, UserRepository},
    rbac::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME},
    state::CoreState,
};

pub(crate) struct TestApp {
    pub(crate) router: Router,
    pub(crate) state: CoreState,
    _temp: TempDir,
}

pub(crate) struct TokenResult {
    pub(crate) status: StatusCode,
    pub(crate) body: AuthTokenResponse,
}

pub(crate) async fn seeded_app() -> TestApp {
    let app = empty_app().await;
    let users = UserRepository::new(app.state.database());
    let rbac = RbacRepository::new(app.state.database());
    let user = users
        .create_user(CreateUser {
            username: "admin".to_owned(),
            password_hash: password_hash("password"),
            display_name: Some("Admin".to_owned()),
        })
        .await
        .expect("user should be created");
    let admin_role_id = rbac
        .ensure_role(ADMIN_ROLE_CODE, ADMIN_ROLE_NAME, None)
        .await
        .expect("admin role should exist");
    rbac.assign_role_to_user(user.id, admin_role_id)
        .await
        .expect("admin role should assign");

    app
}

pub(crate) async fn empty_app() -> TestApp {
    let temp = tempdir().expect("temp dir should exist");
    let config = AppConfig {
        server: ServerConfig {
            mode: RuntimeMode::SelfHosted,
            ..ServerConfig::default()
        },
        storage: StorageConfig {
            database_path: temp
                .path()
                .join("winestock.sqlite")
                .to_string_lossy()
                .into_owned(),
            files_dir: temp.path().join("files").to_string_lossy().into_owned(),
            auto_migrate: true,
        },
    };
    let bootstrap = bootstrap_from_config(&config)
        .await
        .expect("bootstrap should succeed");
    let local = bootstrap.local_service.expect("local service should exist");
    let state = CoreState::from_local_service(&local);
    let router = crate::build_router_with_local_service(&local);

    TestApp {
        router,
        state,
        _temp: temp,
    }
}

pub(crate) async fn seed_plain_user(database: &DatabaseConnection, username: &str, password: &str) {
    UserRepository::new(database)
        .create_user(CreateUser {
            username: username.to_owned(),
            password_hash: password_hash(password),
            display_name: None,
        })
        .await
        .expect("plain user should be created");
}

pub(crate) async fn login_request(app: &TestApp, username: &str, password: &str) -> TokenResult {
    let response = raw_login_request(app, username, password).await;
    let status = response.status();
    let body = json_body(response).await;

    TokenResult { status, body }
}

pub(crate) async fn raw_login_request(
    app: &TestApp,
    username: &str,
    password: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/login",
            &AuthLoginRequest {
                username: username.to_owned(),
                password: password.to_owned(),
                device_name: "test-device".to_owned(),
                client_kind: AuthClientKind::Desktop,
                version: "0.1.0-test".to_owned(),
            },
        ))
        .await
        .expect("request should complete")
}

pub(crate) async fn raw_register_request(
    app: &TestApp,
    username: &str,
    password: &str,
    access_token: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json");
    if let Some(access_token) = access_token {
        builder = builder.header("authorization", format!("Bearer {access_token}"));
    }

    app.router
        .clone()
        .oneshot(
            builder
                .body(Body::from(
                    serde_json::to_vec(&AuthRegisterRequest {
                        username: username.to_owned(),
                        password: password.to_owned(),
                    })
                    .expect("body should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

pub(crate) async fn refresh_request(app: &TestApp, refresh_token: &str) -> TokenResult {
    let response = raw_refresh_request(app, refresh_token).await;
    let status = response.status();
    let body = json_body(response).await;

    TokenResult { status, body }
}

pub(crate) async fn raw_refresh_request(
    app: &TestApp,
    refresh_token: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/refresh",
            &AuthRefreshRequest {
                refresh_token: refresh_token.to_owned(),
            },
        ))
        .await
        .expect("request should complete")
}

pub(crate) fn password_hash(password: &str) -> String {
    let salt = SaltString::from_b64("d2luZXN0b2NrX3Rlc3Rfc2FsdA").expect("salt should decode");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("password should hash")
        .to_string()
}

pub(crate) fn json_request<T: serde::Serialize>(
    method: &str,
    uri: &str,
    body: &T,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(body).expect("body should serialize"),
        ))
        .expect("request should build")
}

pub(crate) async fn json_body<T: for<'de> serde::Deserialize<'de>>(
    response: axum::response::Response,
) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    serde_json::from_slice(&bytes).expect("body should decode")
}

pub(crate) async fn text_body(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    String::from_utf8(bytes.to_vec()).expect("body should be utf8")
}
