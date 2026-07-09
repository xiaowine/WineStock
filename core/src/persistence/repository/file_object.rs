//! 文件对象 repository。
//!
//! 本模块属于 core 持久化层，只管理 SQLite 中的文件元数据。
//! 文件内容读写属于 `StorageRuntime.files_dir` 对应的文件系统目录，不在 repository 中处理。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};

use crate::persistence::entity::file_object;

use super::{
    time::sqlite_now,
    validation::{validate_optional_positive_id, validate_repository_input},
};

/// 创建文件元数据时的输入；文件内容必须已经由调用方写入 files/ 目录。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateFileObject {
    /// 文件内容的 SHA-256 摘要，必须由调用方在写入元数据前计算。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub sha256: String,

    /// 文件 MIME 类型；调用方无法判断时允许为空。
    #[garde(length(min = 1, max = 255), custom(validate_optional_not_blank))]
    pub mime_type: Option<String>,

    /// 文件大小，单位字节。
    #[garde(range(min = 0))]
    pub size_bytes: i64,

    /// 文件在 `files/` 目录下的相对存储路径。
    #[garde(length(min = 1, max = 4096), custom(validate_not_blank))]
    pub storage_path: String,

    /// 上传或导入时的原始文件名。
    #[garde(length(min = 1, max = 255), custom(validate_optional_not_blank))]
    pub original_name: Option<String>,

    /// 文件所有者用户 ID；系统级文件或未归属文件允许为空。
    #[garde(custom(validate_optional_positive_id))]
    pub owner_user_id: Option<i64>,
}

/// 文件对象仓储层只管理 SQLite 元数据，不读写大对象文件内容。
pub(crate) struct FileObjectRepository<'db> {
    database: &'db DatabaseConnection,
}

impl<'db> FileObjectRepository<'db> {
    /// 创建绑定到同一个 SeaORM 连接的文件对象仓储。
    pub(crate) fn new(database: &'db DatabaseConnection) -> Self {
        Self { database }
    }

    /// 写入文件元数据；文件内容写入由调用方在文件系统层完成。
    pub(crate) async fn create_metadata(
        &self,
        input: CreateFileObject,
    ) -> Result<file_object::Model, DbErr> {
        validate_repository_input(&input)?;
        let active_file = file_object::ActiveModel {
            sha256: Set(input.sha256),
            mime_type: Set(input.mime_type),
            size_bytes: Set(input.size_bytes),
            storage_path: Set(input.storage_path),
            original_name: Set(input.original_name),
            created_at: Set(sqlite_now(self.database).await?),
            owner_user_id: Set(input.owner_user_id),
            ..Default::default()
        };
        let result = file_object::Entity::insert(active_file)
            .exec(self.database)
            .await?;

        self.find_by_id(result.last_insert_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created file object".to_owned()))
    }

    /// 按数据库主键查询文件元数据。
    pub(crate) async fn find_by_id(&self, id: i64) -> Result<Option<file_object::Model>, DbErr> {
        file_object::Entity::find_by_id(id).one(self.database).await
    }

    /// 按 SHA-256 摘要查询文件元数据，可能返回多个不同 owner 或路径的记录。
    pub(crate) async fn find_by_sha256(
        &self,
        sha256: &str,
    ) -> Result<Vec<file_object::Model>, DbErr> {
        file_object::Entity::find()
            .filter(file_object::Column::Sha256.eq(sha256))
            .all(self.database)
            .await
    }
}
