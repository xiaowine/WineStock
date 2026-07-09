//! core 统一错误响应契约。
//!
//! 本模块属于 `core axum library` 的全局 HTTP 外壳层，负责把非 2xx API 响应固定为
//! JSON 结构。它不决定具体业务错误，只提供稳定的响应格式和兜底路由错误。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// API 错误响应外层结构。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ApiErrorResponse {
    /// 错误主体；前端应优先使用 `code` 做分支和本地化。
    pub error: ApiErrorBody,
}

/// API 错误响应主体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ApiErrorBody {
    /// 稳定错误代码，使用英文 snake_case，不暴露内部异常文本。
    pub code: String,

    /// 安全的默认提示文本，可由前端按 `code` 覆盖为本地化文案。
    pub message: String,

    /// 预留给字段级校验或批量错误；当前普通错误固定为 `null`。
    pub details: Option<serde_json::Value>,
}

impl ApiErrorResponse {
    /// 构造不携带细节的稳定错误响应。
    pub(crate) fn new(code: &'static str, message: &'static str) -> Self {
        Self::with_details(code, message, None)
    }

    /// 构造可携带结构化细节的稳定错误响应。
    pub(crate) fn with_details(
        code: &'static str,
        message: &'static str,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            error: ApiErrorBody {
                code: code.to_string(),
                message: message.to_string(),
                details,
            },
        }
    }
}

/// 把领域错误映射为统一 JSON 响应体，同时保留 HTTP 状态码语义。
pub(crate) fn api_error_response(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> Response {
    (status, Json(ApiErrorResponse::new(code, message))).into_response()
}

/// 把带结构化细节的领域错误映射为统一 JSON 响应体。
pub(crate) fn api_error_response_with_details(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    details: serde_json::Value,
) -> Response {
    (
        status,
        Json(ApiErrorResponse::with_details(code, message, Some(details))),
    )
        .into_response()
}

/// 未匹配到任何 API 路由时返回统一 JSON 404。
pub(crate) async fn not_found() -> Response {
    api_error_response(StatusCode::NOT_FOUND, "not_found", "接口不存在")
}

/// 路径存在但 HTTP 方法不支持时返回统一 JSON 405。
pub(crate) async fn method_not_allowed() -> Response {
    api_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        "请求方法不支持",
    )
}
