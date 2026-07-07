//! repository 写库输入校验工具。
//!
//! 本模块属于 `core` 持久化层，只校验 repository 内部输入实体的静态字段约束。
//! 它不读取请求体，也不替代数据库唯一约束、外键约束或事务校验。

use sea_orm::DbErr;

/// 校验 repository 写库输入，避免无效内部实体一路下沉到数据库错误。
pub(crate) fn validate_repository_input<T>(input: &T) -> Result<(), DbErr>
where
    T: garde::Validate<Context = ()>,
{
    input
        .validate()
        .map_err(|report| DbErr::Custom(format!("invalid repository input: {report}")))
}

/// 校验可选数据库 ID，存在时必须是正整数。
pub(crate) fn validate_optional_positive_id(value: &Option<i64>, _: &()) -> garde::Result {
    match value {
        Some(id) if *id < 1 => Err(garde::Error::new("must_be_positive")),
        _ => Ok(()),
    }
}
