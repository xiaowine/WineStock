//! self-hosted 本机静默会话换取端点行为测试。
//!
//! 覆盖空库自动开通、凭据校验、非本机模式与存量库拒绝、标记用户自愈、
//! 占位密码免旧密码改密与占位标记清除。

use axum::http::StatusCode;
use tower::ServiceExt;

use crate::{
    auth::{AuthClientKind, AuthLocalSessionRequest, AuthTokenResponse},
    persistence::repository::{AuthRepository, RbacRepository, UserRepository},
    rbac::builtin_permission_codes,
    test_support::{
        empty_app, error_code, json_body, json_request, login_request, seeded_app, server_mode_app,
        TestApp,
    },
};

const MARKER_SETTING: &str = "local_auto_login_user_id";
const PLACEHOLDER_SETTING: &str = "local_auto_login_password_placeholder";

fn exchange_request(app: &TestApp) -> AuthLocalSessionRequest {
    let secret = app
        .state
        .local_session_secret()
        .expect("self-hosted app should hold exchange secret")
        .expose()
        .to_owned();

    AuthLocalSessionRequest {
        exchange_token: secret,
        device_name: "test-local-shell".to_owned(),
        client_kind: AuthClientKind::Android,
        version: "0.1.0-test".to_owned(),
    }
}

async fn raw_exchange(
    app: &TestApp,
    request: &AuthLocalSessionRequest,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(json_request("POST", "/api/auth/local-session", request))
        .await
        .expect("request should complete")
}

async fn setting(app: &TestApp, key: &str) -> Option<String> {
    AuthRepository::new(app.state.database())
        .get_setting_value(key)
        .await
        .expect("setting should query")
}

#[tokio::test]
async fn local_session_provisions_admin_on_empty_self_hosted_db() {
    let app = empty_app().await;

    let response = raw_exchange(&app, &exchange_request(&app)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: AuthTokenResponse = json_body(response).await;
    assert_eq!(body.user.username, "admin");
    assert!(!body.user.password_change_required);

    // 自动开通授予全部内置权限。
    let mut expected = builtin_permission_codes();
    expected.sort();
    let mut granted = body.user.permissions.clone();
    granted.sort();
    assert_eq!(granted, expected);

    // 标记与占位设置写入；首用户注册入口随之关闭。
    let marker = setting(&app, MARKER_SETTING).await;
    assert_eq!(marker, Some(body.user.id.clone()));
    assert_eq!(
        setting(&app, PLACEHOLDER_SETTING).await.as_deref(),
        Some("true")
    );

    // 二次换取复用同一标记用户，不产生重复账号。
    let second = raw_exchange(&app, &exchange_request(&app)).await;
    assert_eq!(second.status(), StatusCode::OK);
    let second_body: AuthTokenResponse = json_body(second).await;
    assert_eq!(second_body.user.id, body.user.id);
}

#[tokio::test]
async fn local_session_rejects_wrong_exchange_token() {
    let app = empty_app().await;
    let mut request = exchange_request(&app);
    request.exchange_token = "not-the-real-secret".to_owned();

    let response = raw_exchange(&app, &request).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(response).await, "invalid_credentials");

    // 凭据校验失败不得触发自动开通。
    assert_eq!(setting(&app, MARKER_SETTING).await, None);
}

#[tokio::test]
async fn local_session_unavailable_outside_self_hosted() {
    let app = server_mode_app().await;
    let request = AuthLocalSessionRequest {
        exchange_token: "anything".to_owned(),
        device_name: "test-local-shell".to_owned(),
        client_kind: AuthClientKind::Android,
        version: "0.1.0-test".to_owned(),
    };

    assert!(app.state.local_session_secret().is_none());
    let response = raw_exchange(&app, &request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(response).await, "local_session_unavailable");
}

#[tokio::test]
async fn local_session_unavailable_for_unmarked_existing_users() {
    // 存量库：已有用户但没有标记设置行，拒绝而不做启发式绑定。
    let app = seeded_app().await;

    let response = raw_exchange(&app, &exchange_request(&app)).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(response).await, "local_session_unavailable");
}

#[tokio::test]
async fn local_session_heals_disabled_and_stripped_marked_user() {
    let app = empty_app().await;
    let first: AuthTokenResponse =
        json_body(raw_exchange(&app, &exchange_request(&app)).await).await;
    let user_id: i64 = first.user.id.parse().expect("user id should parse");

    // 模拟 server-mode 期间标记用户被停用并清空权限。
    let users = UserRepository::new(app.state.database());
    let rbac = RbacRepository::new(app.state.database());
    let user = users
        .find_by_id(user_id)
        .await
        .expect("user should query")
        .expect("user should exist");
    users
        .update_status(user, "disabled".to_owned())
        .await
        .expect("user should disable");
    rbac.replace_user_permissions(user_id, &[])
        .await
        .expect("permissions should clear");

    let healed = raw_exchange(&app, &exchange_request(&app)).await;
    assert_eq!(healed.status(), StatusCode::OK);
    let healed_body: AuthTokenResponse = json_body(healed).await;
    assert_eq!(healed_body.user.id, first.user.id);
    assert_eq!(
        healed_body.user.permissions.len(),
        builtin_permission_codes().len()
    );
    let restored = users
        .find_by_id(user_id)
        .await
        .expect("user should query")
        .expect("user should exist");
    assert_eq!(restored.status, "active");
}

#[tokio::test]
async fn placeholder_allows_password_set_without_current_and_clears_flag() {
    let app = empty_app().await;
    let session: AuthTokenResponse =
        json_body(raw_exchange(&app, &exchange_request(&app)).await).await;

    // 占位状态：免旧密码直接设置新密码。
    let change = app
        .router
        .clone()
        .oneshot({
            let mut request = json_request(
                "POST",
                "/api/auth/me/password",
                &serde_json::json!({
                    "current_password": "",
                    "new_password": "real-password-123"
                }),
            );
            request.headers_mut().insert(
                "authorization",
                format!("Bearer {}", session.access_token)
                    .parse()
                    .expect("header should parse"),
            );
            request
        })
        .await
        .expect("request should complete");
    assert_eq!(change.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        setting(&app, PLACEHOLDER_SETTING).await.as_deref(),
        Some("false")
    );

    // 真实密码生效：普通登录可用，静默换取也不受影响。
    let login = login_request(&app, "admin", "real-password-123").await;
    assert_eq!(login.status, StatusCode::OK);

    // 占位清除后，免旧密码路径关闭。
    let denied = app
        .router
        .clone()
        .oneshot({
            let mut request = json_request(
                "POST",
                "/api/auth/me/password",
                &serde_json::json!({
                    "current_password": "",
                    "new_password": "another-password-123"
                }),
            );
            request.headers_mut().insert(
                "authorization",
                format!("Bearer {}", login.body.access_token)
                    .parse()
                    .expect("header should parse"),
            );
            request
        })
        .await
        .expect("request should complete");
    assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn local_session_status_reflects_placeholder() {
    let app = empty_app().await;
    let session: AuthTokenResponse =
        json_body(raw_exchange(&app, &exchange_request(&app)).await).await;

    let status = app
        .router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/auth/local-session/status")
                .header("authorization", format!("Bearer {}", session.access_token))
                .body(axum::body::Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(status.status(), StatusCode::OK);
    let value: serde_json::Value = json_body(status).await;
    assert_eq!(value["password_placeholder"], serde_json::json!(true));
}
