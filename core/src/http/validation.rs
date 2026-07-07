//! HTTP 请求体校验提取器。
//!
//! 本模块属于 `core` 的全局 HTTP 外壳层，负责在业务 handler 之前完成 JSON 解析
//! 和 `garde` 静态字段校验。它不访问数据库，也不执行依赖当前业务状态的校验。

use axum::{
    extract::{FromRequest, Json, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use garde::Validate;
use serde::de::DeserializeOwned;

/// 先解析 JSON 再执行 `garde` 校验的请求体提取器。
pub(crate) struct ValidatedJson<T>(pub(crate) T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate<Context = ()>,
{
    type Rejection = RequestValidationError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(|_| RequestValidationError::Json)?;
        value
            .validate()
            .map_err(|_| RequestValidationError::Validation)?;

        Ok(Self(value))
    }
}

/// 请求体解析或静态字段校验失败。
#[derive(Debug)]
pub(crate) enum RequestValidationError {
    /// JSON 语法、Content-Type、字段类型或未知字段错误。
    Json,

    /// JSON 已能反序列化，但字段值不满足 DTO 约束。
    Validation,
}

impl IntoResponse for RequestValidationError {
    fn into_response(self) -> Response {
        // 对客户端统一暴露稳定错误码，避免把字段规则实现细节写入公开 API。
        (StatusCode::BAD_REQUEST, "invalid_request").into_response()
    }
}
