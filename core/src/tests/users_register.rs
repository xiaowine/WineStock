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
    assert_eq!(user.roles, vec!["admin"]);
    assert_eq!(
        user.permissions,
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
            "user.manage",
            "user.register"
        ]
    );

    let login = login_request(&app, "first-admin", "password").await;
    assert_eq!(login.status, axum::http::StatusCode::OK);
    assert_eq!(login.body.user.roles, vec!["admin"]);
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
    assert!(plain_login.body.user.roles.is_empty());

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
    assert!(user.roles.is_empty());
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
        roles: vec![],
        permissions: vec![],
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
async fn registration_allows_non_admin_role_with_register_permission() {
    let app = empty_app().await;
    let first = raw_register_request(&app, "admin", "password", None).await;
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);

    seed_plain_user(app.state.database(), "registrar", "password").await;
    let rbac = RbacRepository::new(app.state.database());
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
    let users = UserRepository::new(app.state.database());
    let registrar = users
        .find_by_username("registrar")
        .await
        .expect("registrar lookup should succeed")
        .expect("registrar should exist");
    rbac.assign_role_to_user(registrar.id, registrar_role)
        .await
        .expect("registrar role should assign");

    let registrar_login = login_request(&app, "registrar", "password").await;
    assert_eq!(registrar_login.status, axum::http::StatusCode::OK);
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
