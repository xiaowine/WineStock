//! users 模块用户管理接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use tower::ServiceExt;

use crate::{
    persistence::repository::{RbacRepository, UserRepository},
    test_support::{
        error_code, json_body, login_request, raw_login_request, raw_refresh_request,
        seed_plain_user, seeded_app,
    },
    users::controller::{
        PermissionResponse, UserAdminResponse, UserPasswordChangeRequest, UserPasswordResetRequest,
        UserPermissionsUpdateRequest, UserStatus, UserStatusUpdateRequest,
    },
    users::service::PaginatedResponse,
    users::{
        DELETE_USER_PERMISSION, READ_USER_PERMISSION, READ_USER_PERMISSION_DEFINITION_PERMISSION,
        RESET_USER_PASSWORD_PERMISSION, UPDATE_USER_PERMISSIONS_PERMISSION,
        UPDATE_USER_STATUS_PERMISSION,
    },
};

#[tokio::test]
async fn user_management_reads_use_specific_permissions() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "staff-a", "password").await;
    seed_plain_user(app.state.database(), "user-reader", "password").await;
    seed_plain_user(app.state.database(), "permission-reader", "password").await;
    assign_single_permission(
        app.state.database(),
        "user-reader",
        "user-reader-only",
        READ_USER_PERMISSION,
    )
    .await;
    assign_single_permission(
        app.state.database(),
        "permission-reader",
        "permission-reader-only",
        READ_USER_PERMISSION_DEFINITION_PERMISSION,
    )
    .await;

    let user_reader_login = login_request(&app, "user-reader", "password").await;
    let users = authorized_empty_request(
        &app,
        "GET",
        "/api/users?search=staff&page_size=10",
        &user_reader_login.body.access_token,
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
        &user_reader_login.body.access_token,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);

    let removed_roles = authorized_empty_request(
        &app,
        "GET",
        "/api/roles",
        &user_reader_login.body.access_token,
    )
    .await;
    assert_eq!(removed_roles.status(), StatusCode::NOT_FOUND);

    let permission_reader_login = login_request(&app, "permission-reader", "password").await;
    let permissions = authorized_empty_request(
        &app,
        "GET",
        "/api/permissions",
        &permission_reader_login.body.access_token,
    )
    .await;
    assert_eq!(permissions.status(), StatusCode::OK);
    let permissions: Vec<PermissionResponse> = json_body(permissions).await;
    assert!(permissions
        .iter()
        .any(|permission| permission.code == READ_USER_PERMISSION));

    let staff_login = login_request(&app, "staff-a", "password").await;
    let forbidden =
        authorized_empty_request(&app, "GET", "/api/users", &staff_login.body.access_token).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn user_management_updates_status_permissions_password_and_writes_audit() {
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
    assert!(new_login.body.user.password_change_required);

    let permissions = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{managed_id}/permissions"),
        &admin_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![READ_USER_PERMISSION.to_owned()],
        },
    )
    .await;
    assert_eq!(permissions.status(), StatusCode::OK);
    let updated_permissions: UserAdminResponse = json_body(permissions).await;
    assert_eq!(
        updated_permissions.permissions,
        vec![READ_USER_PERMISSION.to_owned()]
    );

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
async fn user_soft_delete_requires_permission_and_invalidates_account() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "managed", "password").await;
    seed_plain_user(app.state.database(), "user-reader", "password").await;
    seed_plain_user(app.state.database(), "user-deleter", "password").await;
    let managed_id = user_id(&app, "managed").await;
    assign_single_permission(
        app.state.database(),
        "user-reader",
        "reader-only",
        READ_USER_PERMISSION,
    )
    .await;
    assign_single_permission(
        app.state.database(),
        "user-deleter",
        "deleter-only",
        DELETE_USER_PERMISSION,
    )
    .await;
    let managed_login = login_request(&app, "managed", "password").await;

    let reader_login = login_request(&app, "user-reader", "password").await;
    let forbidden = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/users/{managed_id}"),
        &reader_login.body.access_token,
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let deleter_login = login_request(&app, "user-deleter", "password").await;
    let deleted = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/users/{managed_id}"),
        &deleter_login.body.access_token,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT status, deleted_at FROM auth_users WHERE id = ?",
            [managed_id.into()],
        ))
        .await
        .expect("deleted user should query")
        .expect("soft-deleted row should remain");
    let status: String = row.try_get("", "status").expect("status should decode");
    let deleted_at: Option<String> = row
        .try_get("", "deleted_at")
        .expect("deleted_at should decode");
    assert_eq!(status, "disabled");
    assert!(deleted_at.is_some());

    let admin_login = login_request(&app, "admin", "password").await;
    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/users/{managed_id}"),
        &admin_login.body.access_token,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::NOT_FOUND);
    let list = authorized_empty_request(
        &app,
        "GET",
        "/api/users?search=managed",
        &admin_login.body.access_token,
    )
    .await;
    let list: PaginatedResponse<UserAdminResponse> = json_body(list).await;
    assert_eq!(list.total, 0);

    let login_after_delete = raw_login_request(&app, "managed", "password").await;
    assert_eq!(login_after_delete.status(), StatusCode::UNAUTHORIZED);
    let refresh_after_delete = raw_refresh_request(&app, &managed_login.body.refresh_token).await;
    assert_eq!(refresh_after_delete.status(), StatusCode::UNAUTHORIZED);

    let repeated = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/users/{managed_id}"),
        &deleter_login.body.access_token,
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::NOT_FOUND);
    assert_eq!(audit_action(&app, managed_id).await, "deleted");
}

#[tokio::test]
async fn user_management_writes_use_specific_permissions() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "managed", "password").await;
    seed_plain_user(app.state.database(), "status-updater", "password").await;
    seed_plain_user(app.state.database(), "permissions-updater", "password").await;
    let managed_id = user_id(&app, "managed").await;
    assign_single_permission(
        app.state.database(),
        "status-updater",
        "status-updater-only",
        UPDATE_USER_STATUS_PERMISSION,
    )
    .await;
    assign_single_permission(
        app.state.database(),
        "permissions-updater",
        "permissions-updater-only",
        UPDATE_USER_PERMISSIONS_PERMISSION,
    )
    .await;

    let status_login = login_request(&app, "status-updater", "password").await;
    let forbidden_permissions = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{managed_id}/permissions"),
        &status_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![],
        },
    )
    .await;
    assert_eq!(forbidden_permissions.status(), StatusCode::FORBIDDEN);
    let disabled = authorized_json_request(
        &app,
        "PATCH",
        &format!("/api/users/{managed_id}/status"),
        &status_login.body.access_token,
        &UserStatusUpdateRequest {
            status: UserStatus::Disabled,
        },
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);

    let permissions_login = login_request(&app, "permissions-updater", "password").await;
    let forbidden_status = authorized_json_request(
        &app,
        "PATCH",
        &format!("/api/users/{managed_id}/status"),
        &permissions_login.body.access_token,
        &UserStatusUpdateRequest {
            status: UserStatus::Active,
        },
    )
    .await;
    assert_eq!(forbidden_status.status(), StatusCode::FORBIDDEN);
    let permissions = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{managed_id}/permissions"),
        &permissions_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![],
        },
    )
    .await;
    assert_eq!(permissions.status(), StatusCode::OK);
}

#[tokio::test]
async fn user_permissions_update_rejects_unknown_codes_without_partial_write() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "managed", "password").await;
    let managed_id = user_id(&app, "managed").await;
    assign_single_permission(
        app.state.database(),
        "managed",
        "managed-reader",
        READ_USER_PERMISSION,
    )
    .await;
    let admin_login = login_request(&app, "admin", "password").await;

    let rejected = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{managed_id}/permissions"),
        &admin_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![
                READ_USER_PERMISSION.to_owned(),
                "user.permission.does-not-exist".to_owned(),
            ],
        },
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(rejected).await, "permission_not_found");

    let permissions = RbacRepository::new(app.state.database())
        .list_user_permissions(managed_id)
        .await
        .expect("user permissions should query");
    assert_eq!(permissions, vec![READ_USER_PERMISSION.to_owned()]);
}

#[tokio::test]
async fn user_password_reset_requires_reset_permission() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "managed", "old-password").await;
    seed_plain_user(app.state.database(), "user-reader", "password").await;
    seed_plain_user(app.state.database(), "password-resetter", "password").await;
    let managed_id = user_id(&app, "managed").await;
    assign_single_permission(
        app.state.database(),
        "user-reader",
        "reader-only",
        READ_USER_PERMISSION,
    )
    .await;
    assign_single_permission(
        app.state.database(),
        "password-resetter",
        "password-resetter-only",
        RESET_USER_PASSWORD_PERMISSION,
    )
    .await;
    assign_single_permission(
        app.state.database(),
        "managed",
        "managed-reader",
        READ_USER_PERMISSION,
    )
    .await;
    let old_login = login_request(&app, "managed", "old-password").await;
    assert_eq!(old_login.status, StatusCode::OK);
    assert!(!old_login.body.user.password_change_required);

    let reader_login = login_request(&app, "user-reader", "password").await;
    let forbidden = authorized_json_request(
        &app,
        "POST",
        &format!("/api/users/{managed_id}/password"),
        &reader_login.body.access_token,
        &UserPasswordResetRequest {
            password: "new-password".to_owned(),
        },
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let resetter_login = login_request(&app, "password-resetter", "password").await;
    let reset = authorized_json_request(
        &app,
        "POST",
        &format!("/api/users/{managed_id}/password"),
        &resetter_login.body.access_token,
        &UserPasswordResetRequest {
            password: "new-password".to_owned(),
        },
    )
    .await;
    assert_eq!(reset.status(), StatusCode::NO_CONTENT);
    let old_refresh = raw_refresh_request(&app, &old_login.body.refresh_token).await;
    assert_eq!(old_refresh.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(old_refresh).await, "invalid_refresh_token");

    let temporary_login = login_request(&app, "managed", "new-password").await;
    assert_eq!(temporary_login.status, StatusCode::OK);
    assert!(temporary_login.body.user.password_change_required);

    let blocked = authorized_empty_request(
        &app,
        "GET",
        "/api/users",
        &temporary_login.body.access_token,
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
    assert_eq!(error_code(blocked).await, "password_change_required");

    let allowed_me = authorized_empty_request(
        &app,
        "GET",
        "/api/auth/me",
        &temporary_login.body.access_token,
    )
    .await;
    assert_eq!(allowed_me.status(), StatusCode::OK);
    let current: crate::auth::AuthUserResponse = json_body(allowed_me).await;
    assert!(current.password_change_required);

    let changed = authorized_json_request(
        &app,
        "POST",
        "/api/auth/me/password",
        &temporary_login.body.access_token,
        &UserPasswordChangeRequest {
            current_password: "new-password".to_owned(),
            new_password: "final-password".to_owned(),
        },
    )
    .await;
    assert_eq!(changed.status(), StatusCode::NO_CONTENT);

    let temporary_password = raw_login_request(&app, "managed", "new-password").await;
    assert_eq!(temporary_password.status(), StatusCode::UNAUTHORIZED);
    let final_login = login_request(&app, "managed", "final-password").await;
    assert_eq!(final_login.status, StatusCode::OK);
    assert!(!final_login.body.user.password_change_required);

    let allowed_after_change =
        authorized_empty_request(&app, "GET", "/api/users", &final_login.body.access_token).await;
    assert_eq!(allowed_after_change.status(), StatusCode::OK);
}

#[tokio::test]
async fn current_user_changes_only_own_password_with_current_password() {
    let app = seeded_app().await;
    seed_plain_user(app.state.database(), "self-user", "old-password").await;
    let login = login_request(&app, "self-user", "old-password").await;

    let wrong_current = authorized_json_request(
        &app,
        "POST",
        "/api/auth/me/password",
        &login.body.access_token,
        &UserPasswordChangeRequest {
            current_password: "wrong-password".to_owned(),
            new_password: "new-password".to_owned(),
        },
    )
    .await;
    assert_eq!(wrong_current.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(wrong_current).await, "invalid_credentials");

    let changed = authorized_json_request(
        &app,
        "POST",
        "/api/auth/me/password",
        &login.body.access_token,
        &UserPasswordChangeRequest {
            current_password: "old-password".to_owned(),
            new_password: "new-password".to_owned(),
        },
    )
    .await;
    assert_eq!(changed.status(), StatusCode::NO_CONTENT);

    let old_login = raw_login_request(&app, "self-user", "old-password").await;
    assert_eq!(old_login.status(), StatusCode::UNAUTHORIZED);
    let new_login = login_request(&app, "self-user", "new-password").await;
    assert_eq!(new_login.status, StatusCode::OK);
}

#[tokio::test]
async fn user_management_protects_last_active_permission_manager() {
    let app = seeded_app().await;
    let admin_id = user_id(&app, "admin").await;
    seed_plain_user(app.state.database(), "management-operator", "password").await;
    let management_operator = crate::security::CurrentUser {
        user_id: user_id(&app, "management-operator").await,
        access_token_id: "management-test".to_owned(),
        permissions: vec![],
        password_change_required: false,
    };

    let disable_last_manager = crate::users::service::update_user_status(
        &app.state,
        &management_operator,
        admin_id,
        UserStatusUpdateRequest {
            status: UserStatus::Disabled,
        },
    )
    .await;
    assert!(matches!(
        disable_last_manager,
        Err(crate::security::AuthApiError::LastPermissionManagerRequired)
    ));

    let delete_last_manager =
        crate::users::service::delete_user(&app.state, &management_operator, admin_id).await;
    assert!(matches!(
        delete_last_manager,
        Err(crate::security::AuthApiError::LastPermissionManagerRequired)
    ));

    seed_plain_user(app.state.database(), "second-manager", "password").await;
    let second_id = user_id(&app, "second-manager").await;
    let rbac = RbacRepository::new(app.state.database());
    let permission_id = rbac
        .ensure_permission(UPDATE_USER_PERMISSIONS_PERMISSION, None)
        .await
        .expect("permission should exist");
    rbac.assign_permission_to_user(second_id, permission_id)
        .await
        .expect("second manager should assign");

    let remove_manage_permission = crate::users::service::update_user_permissions(
        &app.state,
        &management_operator,
        admin_id,
        UserPermissionsUpdateRequest {
            permissions: vec![],
        },
    )
    .await;
    assert!(remove_manage_permission.is_ok());
}

#[tokio::test]
async fn user_management_rejects_changes_to_own_protected_permissions() {
    let app = seeded_app().await;
    let admin_login = login_request(&app, "admin", "password").await;
    let admin_id = user_id(&app, "admin").await;
    let audit_count_before = audit_count(&app, admin_id).await;

    let remove_permission_definition_read = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{admin_id}/permissions"),
        &admin_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![UPDATE_USER_PERMISSIONS_PERMISSION.to_owned()],
        },
    )
    .await;
    assert_eq!(
        remove_permission_definition_read.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        error_code(remove_permission_definition_read).await,
        "self_protected_permissions_update_forbidden"
    );

    let remove_permissions_update = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{admin_id}/permissions"),
        &admin_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![READ_USER_PERMISSION_DEFINITION_PERMISSION.to_owned()],
        },
    )
    .await;
    assert_eq!(remove_permissions_update.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        error_code(remove_permissions_update).await,
        "self_protected_permissions_update_forbidden"
    );
    assert_eq!(audit_count(&app, admin_id).await, audit_count_before);

    let update_other_own_permissions = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{admin_id}/permissions"),
        &admin_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![
                READ_USER_PERMISSION_DEFINITION_PERMISSION.to_owned(),
                UPDATE_USER_PERMISSIONS_PERMISSION.to_owned(),
            ],
        },
    )
    .await;
    assert_eq!(update_other_own_permissions.status(), StatusCode::OK);
    let admin: UserAdminResponse = json_body(update_other_own_permissions).await;
    assert_eq!(
        admin.permissions,
        vec![
            READ_USER_PERMISSION_DEFINITION_PERMISSION.to_owned(),
            UPDATE_USER_PERMISSIONS_PERMISSION.to_owned(),
        ]
    );

    seed_plain_user(app.state.database(), "self-manager", "password").await;
    let self_manager_id = user_id(&app, "self-manager").await;
    assign_single_permission(
        app.state.database(),
        "self-manager",
        "self-manager-only",
        UPDATE_USER_PERMISSIONS_PERMISSION,
    )
    .await;
    let self_manager_login = login_request(&app, "self-manager", "password").await;
    let add_permission_definition_read = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/users/{self_manager_id}/permissions"),
        &self_manager_login.body.access_token,
        &UserPermissionsUpdateRequest {
            permissions: vec![
                READ_USER_PERMISSION_DEFINITION_PERMISSION.to_owned(),
                UPDATE_USER_PERMISSIONS_PERMISSION.to_owned(),
            ],
        },
    )
    .await;
    assert_eq!(
        add_permission_definition_read.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        error_code(add_permission_definition_read).await,
        "self_protected_permissions_update_forbidden"
    );
}

#[tokio::test]
async fn user_management_rejects_self_disable_and_self_temporary_password_reset() {
    let app = seeded_app().await;
    let admin_login = login_request(&app, "admin", "password").await;
    let admin_id = user_id(&app, "admin").await;
    let audit_count_before = audit_count(&app, admin_id).await;

    let disable_self = authorized_json_request(
        &app,
        "PATCH",
        &format!("/api/users/{admin_id}/status"),
        &admin_login.body.access_token,
        &UserStatusUpdateRequest {
            status: UserStatus::Disabled,
        },
    )
    .await;
    assert_eq!(disable_self.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        error_code(disable_self).await,
        "self_status_update_forbidden"
    );

    let delete_self = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/users/{admin_id}"),
        &admin_login.body.access_token,
    )
    .await;
    assert_eq!(delete_self.status(), StatusCode::FORBIDDEN);
    assert_eq!(error_code(delete_self).await, "self_user_delete_forbidden");

    let reset_self = authorized_json_request(
        &app,
        "POST",
        &format!("/api/users/{admin_id}/password"),
        &admin_login.body.access_token,
        &UserPasswordResetRequest {
            password: "temporary-password".to_owned(),
        },
    )
    .await;
    assert_eq!(reset_self.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        error_code(reset_self).await,
        "self_password_reset_forbidden"
    );

    let admin = UserRepository::new(app.state.database())
        .find_by_id(admin_id)
        .await
        .expect("admin lookup should succeed")
        .expect("admin should exist");
    assert_eq!(admin.status, "active");
    assert!(!admin.password_change_required);
    assert_eq!(audit_count(&app, admin_id).await, audit_count_before);

    let refresh = raw_refresh_request(&app, &admin_login.body.refresh_token).await;
    assert_eq!(refresh.status(), StatusCode::OK);
    let original_password = raw_login_request(&app, "admin", "password").await;
    assert_eq!(original_password.status(), StatusCode::OK);
    let rejected_password = raw_login_request(&app, "admin", "temporary-password").await;
    assert_eq!(rejected_password.status(), StatusCode::UNAUTHORIZED);
}

async fn user_id(app: &crate::test_support::TestApp, username: &str) -> i64 {
    UserRepository::new(app.state.database())
        .find_by_username(username)
        .await
        .expect("user lookup should succeed")
        .expect("user should exist")
        .id
}

async fn assign_single_permission(
    database: &DatabaseConnection,
    username: &str,
    _label: &str,
    permission_code: &str,
) {
    let user = UserRepository::new(database)
        .find_by_username(username)
        .await
        .expect("user lookup should succeed")
        .expect("user should exist");
    let rbac = RbacRepository::new(database);
    let permission_id = rbac
        .ensure_permission(permission_code, None)
        .await
        .expect("permission should exist");
    rbac.assign_permission_to_user(user.id, permission_id)
        .await
        .expect("permission should assign");
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

async fn audit_action(app: &crate::test_support::TestApp, user_id: i64) -> String {
    app.state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT action FROM audit_events WHERE entity_type = 'user' AND entity_id = ? ORDER BY id DESC LIMIT 1",
            [user_id.into()],
        ))
        .await
        .expect("audit action should query")
        .expect("audit action row should exist")
        .try_get("", "action")
        .expect("audit action should decode")
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
