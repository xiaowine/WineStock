//! Tauri 构建脚本：生成平台资源，并让正式 bin 与集成测试复用 Windows manifest。

fn main() {
    tauri_build::build();

    // Tauri 的 Windows manifest 资源默认只链接到正式 bin；集成测试也会加载
    // WebView/Windows 控件相关依赖，因此必须复用同一份资源，否则测试进程
    // 可能因系统 Common Controls 版本不匹配而无法启动。
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR must be set by Cargo");
        println!(
            "cargo:rustc-link-arg-tests={}",
            std::path::Path::new(&out_dir)
                .join("resource.lib")
                .display()
        );
    }
}
