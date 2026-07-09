//! auth 模块登出相关测试。

use axum::http::StatusCode;
use tower::ServiceExt;

use crate::{
    auth::AuthLogoutRequest,
    test_support::{json_request, login_request, raw_refresh_request, seeded_app},
};

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
