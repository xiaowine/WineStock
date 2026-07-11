# `shared/src/error.rs`

本文件定义共享配置解析和配置文件加载错误。

## `ConfigParseError`

| 变体           | 限制或含义                              |
|--------------|------------------------------------|
| `Json`       | JSON 语法、字段类型或未知字段错误                |
| `Validation` | JSON 已能反序列化，但字段值不满足 `garde` 静态字段约束 |

该错误只描述平台无关配置解析结果。

## `ConfigFileError`

| 变体                       | 限制或含义                         |
|--------------------------|-------------------------------|
| `ReadConfig`             | 读取已有配置失败，保留配置路径和底层 IO 错误     |
| `ParseConfig`            | 已有文件无法解析或不满足共享字段约束           |
| `CreateConfigDirectory`  | 缺失配置需要初始化，但父目录创建失败           |
| `SerializeDefaultConfig` | 调用方提供的默认配置无法序列化为 JSON         |
| `WriteDefaultConfig`     | 默认配置文件无法以非覆盖方式创建或无法完整写入      |

该错误保留调用方传入的路径和 source 链，但配置位置、默认值、相对路径解析和启动策略仍由平台 shell 决定。
