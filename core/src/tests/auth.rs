//! 鉴权 HTTP 流程和 token 行为测试。

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tempfile::{tempdir, TempDir};
use tower::ServiceExt;
use winestock_shared::{
    AppConfig, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse, RuntimeMode, ServerConfig, StorageConfig,
};

use crate::{
    bootstrap_from_config,
    persistence::repository::{CreateUser, RbacRepository, UserRepository},
    rbac::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME},
};

use super::*;
use super::{runtime::AccessClaims, security::unix_timestamp};

struct TestApp {
    router: Router,
    state: AuthRuntime,
    _temp: TempDir,
}

#[tokio::test]
async fn login_returns_tokens_and_access_token_reads_current_user() {
    let app = seeded_app().await;

    let login = login_request(&app, "admin", "password").await;

    assert_eq!(login.status, StatusCode::OK);
    assert!(!login.body.access_token.is_empty());
    assert!(!login.body.refresh_token.is_empty());
    assert_eq!(login.body.expires_in, 900);
    assert_eq!(login.body.user.username, "admin");
    assert_eq!(login.body.user.roles, vec!["admin"]);
    assert_eq!(
        login.body.user.permissions,
        vec!["stock.read", "stock.write", "user.manage", "user.register"]
    );

    let missing = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let me = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header(
                    "authorization",
                    format!("Bearer {}", login.body.access_token),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(me.status(), StatusCode::OK);
    let current: AuthUserResponse = json_body(me).await;
    assert_eq!(current.username, "admin");
    assert_eq!(
        current.permissions,
        vec!["stock.read", "stock.write", "user.manage", "user.register"]
    );
}

#[tokio::test]
async fn wrong_password_returns_uniform_unauthorized_error() {
    let app = seeded_app().await;
    let response = raw_login_request(&app, "admin", "wrong").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(text_body(response).await, "invalid_credentials");
}

#[tokio::test]
async fn first_registration_requires_no_token_and_becomes_admin() {
    let app = empty_app().await;

    let response = raw_register_request(&app, " first-admin ", "password", None).await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let user: AuthUserResponse = json_body(response).await;
    assert_eq!(user.username, "first-admin");
    assert_eq!(user.roles, vec!["admin"]);
    assert_eq!(
        user.permissions,
        vec!["stock.read", "stock.write", "user.manage", "user.register"]
    );

    let login = login_request(&app, "first-admin", "password").await;
    assert_eq!(login.status, StatusCode::OK);
    assert_eq!(login.body.user.roles, vec!["admin"]);
}

#[tokio::test]
async fn registration_requires_register_permission_after_first_user_exists() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    let missing_token = raw_register_request(&app, "staff", "password", None).await;
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    seed_plain_user(&app.state.database, "plain", "password").await;
    let plain_login = login_request(&app, "plain", "password").await;
    assert_eq!(plain_login.status, StatusCode::OK);
    assert!(plain_login.body.user.roles.is_empty());

    let forbidden = raw_register_request(
        &app,
        "staff",
        "password",
        Some(&plain_login.body.access_token),
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let admin_login = login_request(&app, "admin", "password").await;
    let created = raw_register_request(
        &app,
        "staff",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let user: AuthUserResponse = json_body(created).await;
    assert_eq!(user.username, "staff");
    assert!(user.roles.is_empty());
}

#[tokio::test]
async fn registration_allows_non_admin_role_with_register_permission() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    seed_plain_user(&app.state.database, "registrar", "password").await;
    let rbac = RbacRepository::new(&app.state.database);
    let registrar_role = rbac
        .ensure_role("registrar", "Registrar", Some("允许注册用户的业务角色。"))
        .await
        .expect("registrar role should exist");
    let register_permission = rbac
        .ensure_permission(REGISTER_USER_PERMISSION, Some("注册新用户。"))
        .await
        .expect("register permission should exist");
    rbac.assign_permission_to_role(registrar_role, register_permission)
        .await
        .expect("register permission should assign");
    let users = UserRepository::new(&app.state.database);
    let registrar = users
        .find_by_username("registrar")
        .await
        .expect("registrar lookup should succeed")
        .expect("registrar should exist");
    rbac.assign_role_to_user(registrar.id, registrar_role)
        .await
        .expect("registrar role should assign");

    let registrar_login = login_request(&app, "registrar", "password").await;
    assert_eq!(registrar_login.status, StatusCode::OK);
    assert_eq!(registrar_login.body.user.roles, vec!["registrar"]);
    assert_eq!(
        registrar_login.body.user.permissions,
        vec![REGISTER_USER_PERMISSION.to_owned()]
    );

    let created = raw_register_request(
        &app,
        "created-by-registrar",
        "password",
        Some(&registrar_login.body.access_token),
    )
    .await;

    assert_eq!(created.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn registration_checks_current_register_permission_in_database() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let admin_login = login_request(&app, "admin", "password").await;

    app.state
        .database
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
                DELETE FROM auth_user_role_assignments
                WHERE user_id = (
                    SELECT id FROM auth_users WHERE username = 'admin'
                )
                "#
            .to_owned(),
        ))
        .await
        .expect("admin role assignment should be removable");

    let stale_register_permission = raw_register_request(
        &app,
        "late-staff",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;

    assert_eq!(stale_register_permission.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn registration_rejects_duplicate_or_invalid_usernames() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let admin_login = login_request(&app, "admin", "password").await;

    let duplicate = raw_register_request(
        &app,
        "admin",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(duplicate).await, "username_taken");

    let empty_username = raw_register_request(
        &app,
        "   ",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(empty_username.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn refresh_rotates_token_and_rejects_reused_old_token() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let first_refresh = refresh_request(&app, &login.body.refresh_token).await;
    assert_eq!(first_refresh.status, StatusCode::OK);
    assert_ne!(first_refresh.body.refresh_token, login.body.refresh_token);

    let reused_old = raw_refresh_request(&app, &login.body.refresh_token).await;
    assert_eq!(reused_old.status(), StatusCode::UNAUTHORIZED);

    let second_refresh = refresh_request(&app, &first_refresh.body.refresh_token).await;
    assert_eq!(second_refresh.status, StatusCode::OK);
    assert_ne!(
        second_refresh.body.refresh_token,
        first_refresh.body.refresh_token
    );
}

#[tokio::test]
async fn refresh_rotation_keeps_other_device_tokens_active() {
    let app = seeded_app().await;
    let desktop_login = login_request(&app, "admin", "password").await;
    let android_login = login_request(&app, "admin", "password").await;
    assert_ne!(
        desktop_login.body.refresh_token,
        android_login.body.refresh_token
    );

    let desktop_refresh = refresh_request(&app, &desktop_login.body.refresh_token).await;
    assert_eq!(desktop_refresh.status, StatusCode::OK);

    let android_refresh = refresh_request(&app, &android_login.body.refresh_token).await;
    assert_eq!(android_refresh.status, StatusCode::OK);
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let logout = app
        .router
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/logout",
            &AuthLogoutRequest {
                refresh_token: login.body.refresh_token.clone(),
            },
        ))
        .await
        .expect("request should complete");
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let refresh = raw_refresh_request(&app, &login.body.refresh_token).await;
    assert_eq!(refresh.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_and_expired_access_tokens_are_rejected() {
    let app = seeded_app().await;

    let invalid = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", "Bearer not-a-jwt")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(app.state.active_signing_key.key_id.clone());
    let expired = encode(
        &header,
        &AccessClaims {
            sub: "1".to_owned(),
            jti: "expired".to_owned(),
            iat: 1,
            exp: 1,
            roles: vec![],
            permissions: vec![],
        },
        &EncodingKey::from_secret(app.state.active_signing_key.key_material.as_bytes()),
    )
    .expect("expired token should encode");

    let expired_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {expired}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);

    let mut wrong_key_header = Header::new(Algorithm::HS256);
    wrong_key_header.kid = Some(app.state.active_signing_key.key_id.clone());
    let wrong_signature = encode(
        &wrong_key_header,
        &AccessClaims {
            sub: "1".to_owned(),
            jti: "wrong-signature".to_owned(),
            iat: unix_timestamp().expect("time should be available") as usize,
            exp: (unix_timestamp().expect("time should be available") + 900) as usize,
            roles: vec![],
            permissions: vec![],
        },
        &EncodingKey::from_secret(b"wrong-signing-key"),
    )
    .expect("wrong-signature token should encode");

    let wrong_signature_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {wrong_signature}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(wrong_signature_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn permission_middleware_blocks_before_business_handler() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let handler_called = Arc::new(AtomicBool::new(false));
    let restricted_called = Arc::clone(&handler_called);
    let router = Router::new()
        .route(
            "/restricted",
            require_permission(
                get(move || {
                    let restricted_called = Arc::clone(&restricted_called);
                    async move {
                        restricted_called.store(true, Ordering::SeqCst);
                        StatusCode::NO_CONTENT
                    }
                }),
                app.state.clone(),
                "admin.manage",
            ),
        )
        .with_state(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/restricted")
                .header(
                    "authorization",
                    format!("Bearer {}", login.body.access_token),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!handler_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn openapi_includes_bearer_auth_and_auth_paths() {
    let app = seeded_app().await;
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(crate::OPENAPI_JSON_PATH)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value = json_body(response).await;
    assert!(value["components"]["securitySchemes"]["bearerAuth"].is_object());
    assert!(value["paths"]["/api/auth/register"].is_object());
    assert!(value["paths"]["/api/auth/login"].is_object());
    assert!(value["paths"]["/api/auth/refresh"].is_object());
    assert!(value["paths"]["/api/auth/logout"].is_object());
    assert!(value["paths"]["/api/auth/me"].is_object());
}

async fn seeded_app() -> TestApp {
    let app = empty_app().await;
    let users = UserRepository::new(&app.state.database);
    let rbac = RbacRepository::new(&app.state.database);
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

async fn empty_app() -> TestApp {
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
    let state = AuthRuntime::from_local_service(&local);
    let router = crate::build_router_with_local_service(&local);

    TestApp {
        router,
        state,
        _temp: temp,
    }
}

async fn seed_plain_user(database: &DatabaseConnection, username: &str, password: &str) {
    UserRepository::new(database)
        .create_user(CreateUser {
            username: username.to_owned(),
            password_hash: password_hash(password),
            display_name: None,
        })
        .await
        .expect("plain user should be created");
}

fn password_hash(password: &str) -> String {
    let salt = SaltString::from_b64("d2luZXN0b2NrX3Rlc3Rfc2FsdA").expect("salt should decode");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("password should hash")
        .to_string()
}

async fn login_request(app: &TestApp, username: &str, password: &str) -> TokenResult {
    let response = raw_login_request(app, username, password).await;
    let status = response.status();
    let body = json_body(response).await;

    TokenResult { status, body }
}

async fn raw_login_request(
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
                device_name: Some("test-device".to_owned()),
                client_kind: Some("test".to_owned()),
            },
        ))
        .await
        .expect("request should complete")
}

async fn raw_register_request(
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

async fn refresh_request(app: &TestApp, refresh_token: &str) -> TokenResult {
    let response = raw_refresh_request(app, refresh_token).await;
    let status = response.status();
    let body = json_body(response).await;

    TokenResult { status, body }
}

async fn raw_refresh_request(app: &TestApp, refresh_token: &str) -> axum::response::Response {
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

fn json_request<T: serde::Serialize>(method: &str, uri: &str, body: &T) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(body).expect("body should serialize"),
        ))
        .expect("request should build")
}

async fn json_body<T: for<'de> serde::Deserialize<'de>>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    serde_json::from_slice(&bytes).expect("body should decode")
}

async fn text_body(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    String::from_utf8(bytes.to_vec()).expect("body should be utf8")
}

struct TokenResult {
    status: StatusCode,
    body: AuthTokenResponse,
}
