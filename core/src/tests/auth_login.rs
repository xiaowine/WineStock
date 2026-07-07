//! auth 模块登录相关测试。

use axum::http::StatusCode;
use winestock_shared::AuthUserResponse;

use crate::test_support::{login_request, raw_login_request, seeded_app, text_body};

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
            roles: vec!["admin".to_owned()],
            permissions: vec![
                "stock.read".to_owned(),
                "stock.write".to_owned(),
                "user.manage".to_owned(),
                "user.register".to_owned(),
            ],
        }
    );
}

#[tokio::test]
async fn wrong_password_returns_uniform_unauthorized_error() {
    let app = seeded_app().await;
    let response = raw_login_request(&app, "admin", "wrong").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(text_body(response).await, "invalid_credentials");
}
