//! core 全量 API 错误码注册表。
//!
//! 本模块属于 `core axum library` 的 HTTP 外壳层，集中登记所有经统一错误响应契约
//! 返回的稳定错误码（英文 snake_case）。它是前端 `error.*` 本地化键覆盖的机械依据：
//! 前端 i18n 扫描测试逐码核对语言包（见 `frontend/tests/i18nScan.test.mjs`），
//! 本模块测试逐码核对源码发射点。新增错误码时必须在注册表追加，否则前端无法按码本地化。

/// 当前公开的全部稳定 API 错误码（按字母序维护）。
///
/// 该清单在非测试构建中不被引用，但仍是权威登记：core 测试逐码核对源码发射点，
/// 前端 `i18nScan` 测试逐码核对 `error.*` 语言包键覆盖。
#[allow(dead_code)]
pub(crate) const API_ERROR_CODES: &[&str] = &[
    "category_name_taken",
    "category_not_found",
    "file_already_bound",
    "file_not_found",
    "file_storage_error",
    "image_too_large",
    "inbound_direct_approval_forbidden",
    "inbound_order_not_found",
    "initial_user_already_exists",
    "insufficient_stock",
    "internal_auth_error",
    "internal_file_error",
    "internal_stock_error",
    "invalid_access_token",
    "invalid_credentials",
    "invalid_image_type",
    "invalid_image_upload",
    "invalid_lcsc_product_code",
    "invalid_refresh_token",
    "invalid_register_request",
    "invalid_request",
    "item_image_unavailable",
    "item_not_found",
    "last_permission_manager_required",
    "lcsc_invalid_response",
    "lcsc_lookup_busy",
    "lcsc_lookup_failed",
    "lcsc_lookup_timeout",
    "lcsc_product_not_found",
    "local_initial_user_required",
    "local_session_unavailable",
    "location_group_cycle",
    "location_group_depth_exceeded",
    "location_group_in_use",
    "location_group_name_taken",
    "location_group_not_found",
    "location_in_use",
    "location_name_taken",
    "location_not_found",
    "method_not_allowed",
    "not_found",
    "order_not_pending",
    "outbound_order_not_found",
    "password_change_required",
    "permission_denied",
    "permission_not_found",
    "self_password_reset_forbidden",
    "self_protected_permissions_update_forbidden",
    "self_status_update_forbidden",
    "self_user_delete_forbidden",
    "sku_taken",
    "stock_batch_not_found",
    "substitute_not_found",
    "template_name_taken",
    "template_not_found",
    "user_not_found",
    "username_taken",
];

#[cfg(test)]
mod tests {
    use super::API_ERROR_CODES;
    use std::path::Path;

    /// 注册表必须与错误码实际发射点一致：每个注册码都应作为字面量出现在 core 源码中。
    #[test]
    fn registry_codes_are_emitted_in_source() {
        let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut sources = String::new();
        collect_sources(&src_dir, &mut sources);

        for code in API_ERROR_CODES {
            assert!(
                sources.contains(&format!("\"{code}\"")),
                "错误码 {code} 已注册但未在 core 源码中作为字面量发射，请核对映射或删除注册"
            );
        }
    }

    /// 注册表不应包含重复码；按字母序断言去重后的长度一致。
    #[test]
    fn registry_has_no_duplicates() {
        let mut sorted = API_ERROR_CODES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), API_ERROR_CODES.len());
    }

    fn collect_sources(dir: &Path, output: &mut String) {
        for entry in std::fs::read_dir(dir).expect("core src 目录必须可读") {
            let entry = entry.expect("目录项必须可读");
            let path = entry.path();
            if path.is_dir() {
                collect_sources(&path, output);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let content =
                    std::fs::read_to_string(&path).expect("core 源码必须可读为 UTF-8");
                output.push_str(&content);
            }
        }
    }
}
