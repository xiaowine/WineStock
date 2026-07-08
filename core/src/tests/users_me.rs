//! users 模块当前用户接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use winestock_shared::AuthUserResponse;

use crate::test_support::{json_body, login_request, seeded_app};

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
            "stock.inbound.approve",
            "stock.inbound.create",
            "stock.item.manage",
            "stock.outbound.approve",
            "stock.outbound.create",
            "stock.read",
            "stock.substitute.manage",
            "stock.template.manage",
            "stock.write",
            "user.password.reset",
            "user.permission.read",
            "user.permissions.update",
            "user.read",
            "user.register",
            "user.status.update"
        ]
    );
}
