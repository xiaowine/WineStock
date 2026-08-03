//! users 模块当前用户接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    auth::AuthUserResponse,
    test_support::{error_code, json_body, login_request, seeded_app},
};

#[tokio::test]
async fn me_requires_token_and_returns_latest_user_snapshot() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

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
    assert!(!current.password_change_required);
    assert_eq!(
        current.permissions,
        vec![
            "audit.read",
            "stock.dashboard.read",
            "stock.inbound.approve",
            "stock.inbound.create",
            "stock.inbound.read",
            "stock.item.manage",
            "stock.item.read",
            "stock.location.manage",
            "stock.location.read",
            "stock.outbound.approve",
            "stock.outbound.create",
            "stock.outbound.read",
            "stock.read",
            "stock.substitute.manage",
            "stock.substitute.read",
            "stock.template.manage",
            "stock.template.read",
            "stock.write",
            "user.delete",
            "user.password.reset",
            "user.permission.read",
            "user.permissions.update",
            "user.read",
            "user.register",
            "user.status.update",
            "user.username.update"
        ]
    );
}

#[tokio::test]
async fn password_change_uses_authenticated_user_without_username_field() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/me/password")
                .header(
                    "authorization",
                    format!("Bearer {}", login.body.access_token),
                )
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"current_password":"password","new_password":"new-password"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn password_change_rejects_username_field() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/me/password")
                .header(
                    "authorization",
                    format!("Bearer {}", login.body.access_token),
                )
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"admin","current_password":"password","new_password":"new-password"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "invalid_request");
}
