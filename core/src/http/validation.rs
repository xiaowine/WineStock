//! HTTP 请求体校验提取器。
//!
//! 本模块属于 `core` 的全局 HTTP 外壳层，负责在业务 handler 之前完成 JSON 解析
//! 和 `garde` 静态字段校验。它不访问数据库，也不执行依赖当前业务状态的校验。

use axum::{
    extract::{FromRequest, FromRequestParts, Json, Path, Query, Request},
    http::request::Parts,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use garde::Validate;
use serde::de::DeserializeOwned;

use super::api_error_response;

/// 先解析 JSON 再执行 `garde` 校验的请求体提取器。
pub(crate) struct ValidatedJson<T>(pub(crate) T);

/// 把路径参数解析失败收敛为统一 JSON 错误响应的提取器。
pub(crate) struct ValidatedPath<T>(pub(crate) T);

/// 把查询参数解析失败收敛为统一 JSON 错误响应的提取器。
pub(crate) struct ValidatedQuery<T>(pub(crate) T);

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

impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = RequestValidationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|_| RequestValidationError::Path)?;

        Ok(Self(value))
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = RequestValidationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|_| RequestValidationError::Query)?;

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

    /// 路径参数缺失、格式错误或类型不匹配。
    Path,

    /// 查询参数格式错误或类型不匹配。
    Query,
}

impl IntoResponse for RequestValidationError {
    fn into_response(self) -> Response {
        // 对客户端统一暴露稳定 JSON 错误码，避免把字段规则实现细节写入公开 API。
        api_error_response(StatusCode::BAD_REQUEST, "invalid_request", "请求参数无效")
    }
}
