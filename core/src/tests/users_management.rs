//! users 模块用户管理接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    persistence::repository::{RbacRepository, UserRepository},
    rbac::ADMIN_ROLE_CODE,
    test_support::{
        json_body, login_request, raw_login_request, seed_plain_user, seeded_app, text_body,
    },
    users::controller::{
        PermissionResponse, RoleResponse, UserAdminResponse, UserPasswordResetRequest,
        UserRolesUpdateRequest, UserStatus, UserStatusUpdateRequest,
    },
    users::service::PaginatedResponse,
};

const STAFF_ROLE_CODE: &str = "staff";

#[tokio::test]
async fn user_management_lists_and_reads_rbac_with_manage_permission() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "staff-a", "password").await;
    let admin_login = login_request(&app, "admin", "password").await;

    let users = authorized_empty_request(
        &app,
        "GET",
        "/api/users?search=staff&page_size=10",
        &admin_login.body.access_token,
    )
    .await;
    assert_eq!(users.status(), StatusCode::OK);
    let users: PaginatedResponse<UserAdminResponse> = json_body(users).await;
    assert_eq!(users.total, 1);
    assert_eq!(users.items[0].username, "staff-a");
    assert_eq!(users.items[0].status, UserStatus::Active);

    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/users/{}", users.items[0].id),
        &admin_login.body.access_token,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);

    let roles =
        authorized_empty_request(&app, "GET", "/api/roles", &admin_login.body.access_token).await;
    assert_eq!(roles.status(), StatusCode::OK);
    let roles: Vec<RoleResponse> = json_body(roles).await;
    assert!(roles.iter().any(|role| role.code == ADMIN_ROLE_CODE));

    let permissions = authorized_empty_request(
        &app,
        "GET",
        "/api/permissions",
        &admin_login.body.access_token,
    )
    .await;
    assert_eq!(permissions.status(), StatusCode::OK);
    let permissions: Vec<PermissionResponse> = json_body(permissions).await;
    assert!(permissions
        .iter()
        .any(|permission| permission.code == "user.manage"));

    let staff_login = login_request(&app, "staff-a", "password").await;
    let forbidden =
        authorized_empty_request(&app, "GET", "/api/users", &staff_login.body.access_token).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn user_management_updates_status_roles_password_and_writes_audit() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "managed", "old-password").await;
    let managed_id = user_id(&app, "managed").await;
    let admin_login = login_request(&app, "admin", "password").await;

    let reset = authorized_json_request(
        &app,
        "POST",
        &format!("/api/users/{managed_id}/password"),
        &admin_login.body.access_token,
        &UserPasswordResetRequest {
            password: "new-password".to_owned(),
        },
    )
    .await;
    assert_eq!(reset.status(), StatusCode::NO_CONTENT);
    let new_login = login_request(&app, "managed", "new-password").await;
    assert_eq!(new_login.status, StatusCode::OK);

    let staff_role_id = RbacRepository::new(app.state.database())
        .ensure_role(STAFF_ROLE_CODE, "Staff", None)
        .await
        .expect("staff role should exist");
    assert!(staff_role_id > 0);
    let roles = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{managed_id}/roles"),
        &admin_login.body.access_token,
        &UserRolesUpdateRequest {
            roles: vec![STAFF_ROLE_CODE.to_owned()],
        },
    )
    .await;
    assert_eq!(roles.status(), StatusCode::OK);
    let updated_roles: UserAdminResponse = json_body(roles).await;
    assert_eq!(updated_roles.roles, vec![STAFF_ROLE_CODE.to_owned()]);

    let disabled = authorized_json_request(
        &app,
        "PATCH",
        &format!("/api/users/{managed_id}/status"),
        &admin_login.body.access_token,
        &UserStatusUpdateRequest {
            status: UserStatus::Disabled,
        },
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    let disabled: UserAdminResponse = json_body(disabled).await;
    assert_eq!(disabled.status, UserStatus::Disabled);

    let disabled_login = raw_login_request(&app, "managed", "new-password").await;
    assert_eq!(disabled_login.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(audit_count(&app, managed_id).await, 3);
}

#[tokio::test]
async fn user_management_protects_last_active_admin() {
    let app = seeded_app().await;
    let admin_login = login_request(&app, "admin", "password").await;
    let admin_id = user_id(&app, "admin").await;

    let disable_last_admin = authorized_json_request(
        &app,
        "PATCH",
        &format!("/api/users/{admin_id}/status"),
        &admin_login.body.access_token,
        &UserStatusUpdateRequest {
            status: UserStatus::Disabled,
        },
    )
    .await;
    assert_eq!(disable_last_admin.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(disable_last_admin).await, "last_admin_required");

    seed_plain_user(app.state.database(), "second-admin", "password").await;
    let second_id = user_id(&app, "second-admin").await;
    let rbac = RbacRepository::new(app.state.database());
    let admin_role_id = rbac
        .ensure_role(ADMIN_ROLE_CODE, "Admin", None)
        .await
        .expect("admin role should exist");
    rbac.assign_role_to_user(second_id, admin_role_id)
        .await
        .expect("second admin should assign");

    let remove_admin_role = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{admin_id}/roles"),
        &admin_login.body.access_token,
        &UserRolesUpdateRequest { roles: vec![] },
    )
    .await;
    assert_eq!(remove_admin_role.status(), StatusCode::OK);
}

async fn user_id(app: &crate::test_support::TestApp, username: &str) -> i64 {
    UserRepository::new(app.state.database())
        .find_by_username(username)
        .await
        .expect("user lookup should succeed")
        .expect("user should exist")
        .id
}

async fn audit_count(app: &crate::test_support::TestApp, user_id: i64) -> i64 {
    app.state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM audit_events WHERE entity_type = 'user' AND entity_id = ?",
            [user_id.into()],
        ))
        .await
        .expect("audit count should query")
        .expect("audit count row should exist")
        .try_get("", "count")
        .expect("audit count should decode")
}

async fn authorized_empty_request(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    access_token: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

async fn authorized_json_request<T: serde::Serialize>(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    access_token: &str,
    body: &T,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {access_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(body).expect("body should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}
