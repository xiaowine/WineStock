//! security 前置层鉴权与权限 middleware 测试。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use tower::ServiceExt;

use crate::{
    security::{require_permission, unix_timestamp, AccessClaims},
    test_support::{login_request, seeded_app},
};

#[tokio::test]
async fn invalid_and_expired_access_tokens_are_rejected() {
    let app = seeded_app().await;

    let invalid = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", "Bearer invalid")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(app.state.security().active_signing_key().key_id.clone());
    let expired = encode(
        &header,
        &AccessClaims {
            sub: "1".to_owned(),
            jti: "expired".to_owned(),
            iat: 1,
            exp: 1,
            roles: vec![],
            permissions: vec![],
        },
        &EncodingKey::from_secret(
            app.state
                .security()
                .active_signing_key()
                .key_material
                .as_bytes(),
        ),
    )
    .expect("expired token should encode");

    let expired_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {expired}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);

    let mut wrong_key_header = Header::new(Algorithm::HS256);
    wrong_key_header.kid = Some(app.state.security().active_signing_key().key_id.clone());
    let wrong_signature = encode(
        &wrong_key_header,
        &AccessClaims {
            sub: "1".to_owned(),
            jti: "wrong-signature".to_owned(),
            iat: unix_timestamp().expect("time should be available") as usize,
            exp: (unix_timestamp().expect("time should be available") + 900) as usize,
            roles: vec![],
            permissions: vec![],
        },
        &EncodingKey::from_secret(b"wrong-signing-key"),
    )
    .expect("wrong-signature token should encode");

    let wrong_signature_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("authorization", format!("Bearer {wrong_signature}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(wrong_signature_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn permission_middleware_blocks_before_business_handler() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let handler_called = Arc::new(AtomicBool::new(false));
    let restricted_called = Arc::clone(&handler_called);
    let router = Router::new()
        .route(
            "/restricted",
            require_permission(
                get(move || {
                    let restricted_called = Arc::clone(&restricted_called);
                    async move {
                        restricted_called.store(true, Ordering::SeqCst);
                        StatusCode::NO_CONTENT
                    }
                }),
                app.state.clone(),
                "admin.manage",
            ),
        )
        .with_state(app.state.clone());

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/restricted")
                .header(
                    "authorization",
                    format!("Bearer {}", login.body.access_token),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!handler_called.load(Ordering::SeqCst));
}
