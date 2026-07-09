//! auth 模块登录相关测试。

use axum::http::StatusCode;
use garde::Validate;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::{
    auth::{
        AuthClientKind, AuthLoginRequest, AuthRefreshRequest, AuthRegisterRequest, AuthUserResponse,
    },
    security::{hash_refresh_token, CURRENT_REFRESH_TOKEN_VERSION},
    test_support::{error_code, login_request, raw_login_request, seeded_app},
};

#[test]
fn auth_request_validation_rejects_blank_or_oversized_fields() {
    let register = AuthRegisterRequest {
        username: "   ".to_owned(),
        password: "password".to_owned(),
    };
    assert!(register.validate().is_err());

    let login = AuthLoginRequest {
        username: "admin".to_owned(),
        password: "password".to_owned(),
        device_name: "   ".to_owned(),
        client_kind: AuthClientKind::Desktop,
        version: "1.0.0".to_owned(),
    };
    assert!(login.validate().is_err());

    let refresh = AuthRefreshRequest {
        refresh_token: String::new(),
    };
    assert!(refresh.validate().is_err());
}

#[test]
fn auth_login_client_kind_only_accepts_formal_platforms() {
    let json = r#"
    {
      "username": "admin",
      "password": "password",
      "device_name": "workstation",
      "client_kind": "web",
      "version": "1.0.0"
    }
    "#;

    assert!(serde_json::from_str::<AuthLoginRequest>(json).is_err());
}

#[tokio::test]
async fn login_returns_tokens_and_current_permissions() {
    let app = seeded_app().await;

    let login = login_request(&app, "admin", "password").await;

    assert_eq!(login.status, StatusCode::OK);
    assert!(!login.body.access_token.is_empty());
    assert!(!login.body.refresh_token.is_empty());
    assert_eq!(login.body.expires_in, 900);
    assert_eq!(
        login.body.user,
        AuthUserResponse {
            id: login.body.user.id.clone(),
            username: "admin".to_owned(),
            permissions: vec![
                "audit.read".to_owned(),
                "stock.dashboard.read".to_owned(),
                "stock.inbound.approve".to_owned(),
                "stock.inbound.create".to_owned(),
                "stock.inbound.read".to_owned(),
                "stock.item.manage".to_owned(),
                "stock.item.read".to_owned(),
                "stock.location.manage".to_owned(),
                "stock.location.read".to_owned(),
                "stock.outbound.approve".to_owned(),
                "stock.outbound.create".to_owned(),
                "stock.outbound.read".to_owned(),
                "stock.read".to_owned(),
                "stock.substitute.manage".to_owned(),
                "stock.substitute.read".to_owned(),
                "stock.template.manage".to_owned(),
                "stock.template.read".to_owned(),
                "stock.write".to_owned(),
                "user.password.reset".to_owned(),
                "user.permission.read".to_owned(),
                "user.permissions.update".to_owned(),
                "user.read".to_owned(),
                "user.register".to_owned(),
                "user.status.update".to_owned(),
            ],
            password_change_required: false,
        }
    );
}

#[tokio::test]
async fn login_stores_client_and_refresh_token_versions_separately() {
    let app = seeded_app().await;

    let login = login_request(&app, "admin", "password").await;
    assert_eq!(login.status, StatusCode::OK);
    let token_hash = hash_refresh_token(&login.body.refresh_token);
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT device_name, client_kind, app_version, refresh_token_version
            FROM auth_refresh_tokens
            WHERE token_hash = ?
            "#,
            [token_hash.into()],
        ))
        .await
        .expect("refresh token query should succeed")
        .expect("refresh token row should exist");

    let device_name: String = row
        .try_get("", "device_name")
        .expect("device should decode");
    let client_kind: String = row.try_get("", "client_kind").expect("kind should decode");
    let app_version: String = row
        .try_get("", "app_version")
        .expect("app version should decode");
    let refresh_token_version: String = row
        .try_get("", "refresh_token_version")
        .expect("refresh token version should decode");

    assert_eq!(device_name, "test-device");
    assert_eq!(client_kind, "desktop");
    assert_eq!(app_version, "0.1.0-test");
    assert_eq!(refresh_token_version, CURRENT_REFRESH_TOKEN_VERSION);
}

#[tokio::test]
async fn wrong_password_returns_uniform_unauthorized_error() {
    let app = seeded_app().await;
    let response = raw_login_request(&app, "admin", "wrong").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(response).await, "invalid_credentials");
}

#[tokio::test]
async fn invalid_login_payload_is_rejected_before_auth_service() {
    let app = seeded_app().await;
    let response = raw_login_request(&app, "   ", "password").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "invalid_request");
}
