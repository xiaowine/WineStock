//! Desktop Shell 的客户端元数据解析与前端启动注入。
//!
//! 本模块只拥有平台客户端类型、设备名称和应用版本的运行时来源；不拥有
//! Shell Bridge 业务方法、运行配置或鉴权会话。

use serde_json::json;

const FALLBACK_DEVICE_NAME: &str = "WineStock Desktop";

/// 读取当前 Desktop 进程对应的系统用户名，失败时返回稳定的桌面端名称。
pub fn resolve_device_name() -> String {
    normalize_device_name(read_system_username()).unwrap_or_else(|| FALLBACK_DEVICE_NAME.to_owned())
}

/// 生成在前端脚本执行前运行的客户端元数据初始化脚本。
pub fn build_runtime_config_script(device_name: &str, app_version: &str) -> String {
    let metadata = json!({
        "clientKind": "desktop",
        "deviceName": device_name,
        "appVersion": app_version,
    });
    let metadata_json =
        serde_json::to_string(&metadata).expect("客户端元数据必须能够序列化为 JSON");

    format!(
        "window.__WINESTOCK_RUNTIME_CONFIG__ = Object.assign({{}}, window.__WINESTOCK_RUNTIME_CONFIG__ || {{}}, {metadata_json});"
    )
}

fn normalize_device_name(value: Option<String>) -> Option<String> {
    let value = value?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn read_system_username() -> Option<String> {
    whoami::username().ok()
}

#[cfg(test)]
mod tests {
    use super::{build_runtime_config_script, normalize_device_name, FALLBACK_DEVICE_NAME};

    #[test]
    fn blank_device_name_uses_no_value() {
        assert_eq!(normalize_device_name(Some("  ".to_owned())), None);
        assert_eq!(normalize_device_name(None), None);
    }

    #[test]
    fn runtime_script_serializes_metadata_without_script_injection() {
        let script = build_runtime_config_script("user\"\\测试", "1.2.3");

        assert!(script.contains("\"clientKind\":\"desktop\""));
        assert!(script.contains("\\\""));
        assert!(script.contains("测试"));
        assert!(script.contains("\"appVersion\":\"1.2.3\""));
    }

    #[test]
    fn fallback_device_name_is_stable() {
        assert_eq!(FALLBACK_DEVICE_NAME, "WineStock Desktop");
    }
}
