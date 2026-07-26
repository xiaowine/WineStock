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
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use tempfile::{tempdir, TempDir};
use tower::ServiceExt;
use winestock_shared::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};

use crate::{
    auth::{
        AuthClientKind, AuthLoginRequest, AuthRefreshRequest, AuthRegisterRequest,
        AuthTokenResponse,
    },
    bootstrap_from_config,
    persistence::repository::{CreateUser, RbacRepository, UserRepository},
    rbac::builtin_permission_codes,
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
        })
        .await
        .expect("user should be created");
    let permission_ids = rbac
        .find_permission_ids_by_codes(&builtin_permission_codes())
        .await
        .expect("permissions should query")
        .expect("built-in permissions should exist");
    rbac.replace_user_permissions(user.id, &permission_ids)
        .await
        .expect("admin permissions should assign");

    app
}

pub(crate) async fn empty_app() -> TestApp {
    empty_app_with_lcsc_endpoint(None).await
}

pub(crate) async fn empty_app_with_mock_lcsc(endpoint: String) -> TestApp {
    let price_endpoint = endpoint.replace("/api/devices/search", "/api/components/getSmtPartInfo");
    empty_app_with_lcsc_endpoint(Some((endpoint, price_endpoint))).await
}

async fn empty_app_with_lcsc_endpoint(lcsc_endpoints: Option<(String, String)>) -> TestApp {
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
    let mut local = bootstrap.local_service.expect("local service should exist");
    if let Some((search_endpoint, price_endpoint)) = lcsc_endpoints {
        local.external_catalog = crate::external::ExternalCatalogRuntime::with_lcsc_endpoints(
            search_endpoint,
            price_endpoint,
        )
        .expect("mock LCSC client should build");
    }
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
        })
        .await
        .expect("plain user should be created");
}

pub(crate) async fn bootstrap_location_id(app: &TestApp) -> i64 {
    query_location_id_by_name(app, "默认库位").await
}

pub(crate) async fn seed_stock_location(app: &TestApp, name: &str) -> i64 {
    let group_id = query_default_location_group_id(app).await;
    app.state
        .database()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO stock_locations (group_id, name, notes, sort_order)
            VALUES (?, ?, NULL, 0)
            "#,
            vec![group_id.into(), name.to_owned().into()],
        ))
        .await
        .expect("test location should insert");

    query_location_id_by_name(app, name).await
}

async fn query_default_location_group_id(app: &TestApp) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id FROM stock_location_groups WHERE name = '默认库区' AND deleted_at IS NULL",
            [],
        ))
        .await
        .expect("default location group query should succeed")
        .expect("default location group should exist");

    row.try_get("", "id")
        .expect("default location group id should decode")
}

async fn query_location_id_by_name(app: &TestApp, name: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id FROM stock_locations WHERE name = ? AND deleted_at IS NULL",
            [name.into()],
        ))
        .await
        .expect("location query should succeed")
        .expect("location should exist");

    row.try_get("", "id").expect("location id should decode")
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

pub(crate) async fn error_code(response: axum::response::Response) -> String {
    let value: serde_json::Value = json_body(response).await;
    value["error"]["code"]
        .as_str()
        .expect("error code should be a string")
        .to_owned()
}

/// 上传一张最小测试 PNG，并返回可用于物品主图的临时文件 ID。
pub(crate) async fn upload_test_image(app: &TestApp, access_token: &str) -> i64 {
    const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nwinestock";
    let boundary = "winestock-shared-test-image";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"item.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(PNG_BYTES);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/files/images")
                .header("authorization", format!("Bearer {access_token}"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("test image request should build"),
        )
        .await
        .expect("test image upload should complete");
    assert_eq!(response.status(), StatusCode::CREATED);
    let value: serde_json::Value = json_body(response).await;
    value["id"].as_i64().expect("test image id should exist")
}
