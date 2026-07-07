//! auth 模块 refresh 相关测试。

use axum::http::StatusCode;

use crate::test_support::{login_request, raw_refresh_request, refresh_request, seeded_app};

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
