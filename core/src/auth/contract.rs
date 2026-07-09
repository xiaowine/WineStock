//! auth HTTP 契约实体。
//!
//! 本模块属于 `core axum library` 的 auth 业务边界，定义会话认证请求和响应 DTO。
//! 它不处理密码哈希、令牌持久化、权限查询或 Axum 路由。

use serde::{Deserialize, Serialize};

use crate::validation::{validate_code_list, validate_not_blank};

/// 登录客户端类型，限制登录请求只能来自正式平台端。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AuthClientKind {
    /// 桌面端客户端。
    Desktop,

    /// Android 端客户端。
    Android,

    /// Web 前端客户端，用于浏览器调试或后续正式 Web 外壳。
    Web,
}

impl AuthClientKind {
    /// 返回入库和日志中使用的稳定客户端类型代码。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Android => "android",
            Self::Web => "web",
        }
    }
}

/// 用户名密码登录请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub struct AuthLoginRequest {
    /// 登录用户名。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub username: String,

    /// 明文密码只允许出现在登录请求中，服务端不得持久化该值。
    #[garde(length(min = 1, max = 256), custom(validate_not_blank))]
    pub password: String,

    /// 设备名称，用于标识 refresh token 来源。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub device_name: String,

    /// 客户端类型，仅允许桌面端、Android 端或 Web 前端。
    #[garde(skip)]
    pub client_kind: AuthClientKind,

    /// 客户端版本号，用于记录 refresh token 来源版本。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub version: String,
}

/// 注册用户请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub struct AuthRegisterRequest {
    /// 登录用户名，服务端会去除首尾空白并要求非空。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub username: String,

    /// 明文密码只允许出现在注册请求中，服务端会保存 Argon2 哈希。
    #[garde(length(min = 1, max = 256), custom(validate_not_blank))]
    pub password: String,
}

/// 刷新访问令牌请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub struct AuthRefreshRequest {
    /// 客户端持有的 opaque refresh token 明文；服务端只保存其哈希。
    #[garde(length(min = 1, max = 512), custom(validate_not_blank))]
    pub refresh_token: String,
}

/// 登出请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub struct AuthLogoutRequest {
    /// 需要吊销的 opaque refresh token 明文。
    #[garde(length(min = 1, max = 512), custom(validate_not_blank))]
    pub refresh_token: String,
}

/// 鉴权接口返回的用户摘要。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub struct AuthUserResponse {
    /// 用户 ID，作为字符串返回，避免前端运行时整数精度差异。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub id: String,

    /// 登录用户名。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub username: String,

    /// 用户权限代码列表。
    #[garde(inner(length(min = 1, max = 128)), custom(validate_code_list))]
    pub permissions: Vec<String>,

    /// 是否必须先修改密码；临时密码登录后应只进入改密流程。
    #[garde(skip)]
    pub password_change_required: bool,
}

/// 登录和刷新接口返回的 token 包。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub struct AuthTokenResponse {
    /// JWT access token，用于 `Authorization: Bearer` 请求头。
    #[garde(length(min = 1, max = 8192), custom(validate_not_blank))]
    pub access_token: String,

    /// opaque refresh token 明文，只在本响应中返回一次。
    #[garde(length(min = 1, max = 512), custom(validate_not_blank))]
    pub refresh_token: String,

    /// access token 剩余有效期，单位秒。
    #[garde(range(min = 1))]
    pub expires_in: u64,

    /// 当前登录用户摘要。
    #[garde(dive)]
    pub user: AuthUserResponse,
}
