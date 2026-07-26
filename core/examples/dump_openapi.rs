//! 开发期契约导出工具：把 Debug OpenAPI 文档写为 JSON 文件，供前端 `pnpm gen:api-types` 生成 TypeScript 类型。
//!
//! 输出是不入库的中间产物；仓库仍不提交静态 `openapi.json`。
//! example 默认按 dev profile 编译，天然满足 `openapi_document_json` 的 `debug_assertions` 门控。

use std::path::PathBuf;

fn main() {
    // 默认写到工作区 target 下的固定位置，与调用方当前目录无关；允许用第一个参数覆盖。
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/openapi/openapi.json")
        });

    let document = winestock_core::openapi_document_json().expect("序列化 OpenAPI 文档失败");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).expect("创建 OpenAPI 输出目录失败");
    }
    // 末尾补换行，保持与其它生成文件一致的文本约定。
    std::fs::write(&output, format!("{document}\n")).expect("写入 OpenAPI JSON 失败");
    println!("OpenAPI 文档已写入 {}", output.display());
}
