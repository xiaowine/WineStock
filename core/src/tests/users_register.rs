//! users 模块注册相关测试。

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use winestock_shared::{AuthRegisterRequest, AuthUserResponse};

use crate::{
    persistence::repository::{RbacRepository, UserRepository},
    security::{AuthApiError, CurrentUser},
    test_support::{
        empty_app, json_body, login_request, raw_register_request, seed_plain_user, text_body,
    },
    users::REGISTER_USER_PERMISSION,
};

#[tokio::test]
async fn first_registration_requires_no_token_and_becomes_admin() {
    let app = empty_app().await;

    let response = raw_register_request(&app, " first-admin ", "password", None).await;

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let user: AuthUserResponse = json_body(response).await;
    assert_eq!(user.username, "first-admin");
    assert!(!user.password_change_required);
    assert_eq!(
        user.permissions,
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
            "user.password.reset",
            "user.permission.read",
            "user.permissions.update",
            "user.read",
            "user.register",
            "user.status.update"
        ]
    );

    let login = login_request(&app, "first-admin", "password").await;
    assert_eq!(login.status, axum::http::StatusCode::OK);
    assert!(login
        .body
        .user
        .permissions
        .contains(&"user.permissions.update".to_owned()));

    let first_admin_id = user_id_by_username(&app, "first-admin").await;
    let audit_events = audit_events_for_user(&app, first_admin_id).await;
    assert_eq!(audit_events.len(), 1);
    assert_eq!(audit_events[0].action, "created");
    assert_eq!(audit_events[0].user_id, Some(first_admin_id));
    assert_eq!(audit_events[0].details["first_user"], true);
}

#[tokio::test]
async fn registration_requires_register_permission_after_first_user_exists() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);

    let missing_token = raw_register_request(&app, "staff", "password", None).await;
    assert_eq!(missing_token.status(), axum::http::StatusCode::UNAUTHORIZED);

    seed_plain_user(app.state.database(), "plain", "password").await;
    let plain_login = login_request(&app, "plain", "password").await;
    assert_eq!(plain_login.status, axum::http::StatusCode::OK);
    assert!(plain_login.body.user.permissions.is_empty());

    let forbidden = raw_register_request(
        &app,
        "staff",
        "password",
        Some(&plain_login.body.access_token),
    )
    .await;
    assert_eq!(forbidden.status(), axum::http::StatusCode::FORBIDDEN);

    let admin_login = login_request(&app, "admin", "password").await;
    let created = raw_register_request(
        &app,
        "staff",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(created.status(), axum::http::StatusCode::CREATED);
    let user: AuthUserResponse = json_body(created).await;
    assert_eq!(user.username, "staff");
    assert!(user.permissions.is_empty());
    assert!(!user.password_change_required);

    let admin_id = user_id_by_username(&app, "admin").await;
    let staff_id = user_id_by_username(&app, "staff").await;
    let audit_events = audit_events_for_user(&app, staff_id).await;
    assert_eq!(audit_events.len(), 1);
    assert_eq!(audit_events[0].action, "created");
    assert_eq!(audit_events[0].user_id, Some(admin_id));
    assert_eq!(audit_events[0].details["first_user"], false);
}

#[tokio::test]
async fn registration_service_rechecks_first_user_bypass_inside_transaction() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);

    let missing_current_user = super::service::register(
        &app.state,
        AuthRegisterRequest {
            username: "raced-user".to_owned(),
            password: "password".to_owned(),
        },
        None,
    )
    .await
    .expect_err("stale first-user bypass should be rejected");
    assert!(matches!(
        missing_current_user,
        AuthApiError::InvalidAccessToken
    ));

    let stale_current_user = CurrentUser {
        user_id: 1,
        access_token_id: "stale".to_owned(),
        permissions: vec![],
        password_change_required: false,
    };
    let missing_permission = super::service::register(
        &app.state,
        AuthRegisterRequest {
            username: "forbidden-user".to_owned(),
            password: "password".to_owned(),
        },
        Some(&stale_current_user),
    )
    .await
    .expect_err("stale permission snapshot should be rejected");
    assert!(matches!(missing_permission, AuthApiError::PermissionDenied));
}

#[tokio::test]
async fn registration_allows_user_with_register_permission() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);

    seed_plain_user(app.state.database(), "registrar", "password").await;
    let rbac = RbacRepository::new(app.state.database());
    let register_permission = rbac
        .ensure_permission(REGISTER_USER_PERMISSION, Some("注册新用户。"))
        .await
        .expect("register permission should exist");
    let users = UserRepository::new(app.state.database());
    let registrar = users
        .find_by_username("registrar")
        .await
        .expect("registrar lookup should succeed")
        .expect("registrar should exist");
    rbac.assign_permission_to_user(registrar.id, register_permission)
        .await
        .expect("registrar permission should assign");

    let registrar_login = login_request(&app, "registrar", "password").await;
    assert_eq!(registrar_login.status, axum::http::StatusCode::OK);
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

    assert_eq!(created.status(), axum::http::StatusCode::CREATED);
}

#[tokio::test]
async fn registration_checks_current_register_permission_in_database() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);
    let admin_login = login_request(&app, "admin", "password").await;

    app.state
        .database()
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
                DELETE FROM auth_user_permission_assignments
                WHERE user_id = (
                    SELECT id FROM auth_users WHERE username = 'admin'
                )
                "#
            .to_owned(),
        ))
        .await
        .expect("admin permission assignments should be removable");

    let stale_register_permission = raw_register_request(
        &app,
        "late-staff",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;

    assert_eq!(
        stale_register_permission.status(),
        axum::http::StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn registration_rejects_duplicate_or_invalid_usernames() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);
    let admin_login = login_request(&app, "admin", "password").await;

    let duplicate = raw_register_request(
        &app,
        "admin",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(duplicate.status(), axum::http::StatusCode::CONFLICT);
    assert_eq!(text_body(duplicate).await, "username_taken");

    let empty_username = raw_register_request(
        &app,
        "   ",
        "password",
        Some(&admin_login.body.access_token),
    )
    .await;
    assert_eq!(empty_username.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(text_body(empty_username).await, "invalid_request");
}

async fn user_id_by_username(app: &crate::test_support::TestApp, username: &str) -> i64 {
    UserRepository::new(app.state.database())
        .find_by_username(username)
        .await
        .expect("user lookup should query")
        .expect("user should exist")
        .id
}

#[derive(Debug, Clone, PartialEq)]
struct AuditEventRow {
    user_id: Option<i64>,
    action: String,
    details: serde_json::Value,
}

async fn audit_events_for_user(
    app: &crate::test_support::TestApp,
    user_id: i64,
) -> Vec<AuditEventRow> {
    app.state
        .database()
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT user_id, action, details_json
            FROM audit_events
            WHERE entity_type = 'user' AND entity_id = ?
            ORDER BY id ASC
            "#,
            [user_id.into()],
        ))
        .await
        .expect("audit events should query")
        .into_iter()
        .map(|row| AuditEventRow {
            user_id: row.try_get("", "user_id").expect("user id should decode"),
            action: row.try_get("", "action").expect("action should decode"),
            details: row
                .try_get::<Option<String>>("", "details_json")
                .expect("details should decode")
                .and_then(|details| serde_json::from_str(&details).ok())
                .expect("details should be json"),
        })
        .collect()
}
