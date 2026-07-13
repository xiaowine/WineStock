//! 文件对象 repository。
//!
//! 本模块属于 core 持久化层，只管理 SQLite 中的文件元数据。
//! 文件内容读写属于 `StorageRuntime.files_dir` 对应的文件系统目录，不在 repository 中处理。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryResult, Set, Statement,
};

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

/// 文件读取授权所需的元数据和物品/入库可选业务绑定信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileAccessRecord {
    /// 文件对象元数据。
    pub file: file_object::Model,

    /// 已绑定入库明细 ID；为空表示仍是临时上传。
    pub inbound_order_item_id: Option<i64>,

    /// 已绑定入库单 ID；为空表示仍是临时上传。
    pub inbound_order_id: Option<i64>,

    /// 已绑定物品 ID；可能来自必选主图或扩展图片属性。
    pub item_id: Option<i64>,

    /// 已绑定字段名称；物品主图使用固定中文名称。
    pub field_name: Option<String>,
}

impl FileAccessRecord {
    /// 判断文件是否已经绑定物品主图、物品属性或入库属性。
    pub(crate) fn is_bound(&self) -> bool {
        self.item_id.is_some() || self.inbound_order_item_id.is_some()
    }
}

/// 超过保留期限且尚未绑定的临时文件对象。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaleFileObject {
    /// 文件对象 ID。
    pub id: i64,

    /// 文件在服务端文件目录下的相对路径。
    pub storage_path: String,
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
    #[allow(dead_code)]
    pub(crate) async fn find_by_sha256(
        &self,
        sha256: &str,
    ) -> Result<Vec<file_object::Model>, DbErr> {
        file_object::Entity::find()
            .filter(file_object::Column::Sha256.eq(sha256))
            .all(self.database)
            .await
    }

    /// 查询文件元数据及物品/入库绑定关系，供受控读取和删除授权判断。
    pub(crate) async fn find_access_record(
        &self,
        id: i64,
    ) -> Result<Option<FileAccessRecord>, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT f.id, f.sha256, f.mime_type, f.size_bytes, f.storage_path,
                       f.original_name, f.created_at, f.owner_user_id,
                       ia.inbound_order_item_id, i.order_id AS inbound_order_id,
                       COALESCE(main_item.id, item_attr.item_id) AS item_id,
                       COALESCE(ia.field_name, item_definition.field_name,
                         CASE WHEN main_item.id IS NOT NULL THEN '物品主图' END) AS field_name
                FROM storage_file_objects f
                LEFT JOIN storage_inbound_file_bindings inbound_binding ON inbound_binding.file_object_id = f.id
                LEFT JOIN stock_inbound_order_item_attributes ia ON ia.id = inbound_binding.inbound_order_item_attribute_id
                LEFT JOIN stock_inbound_order_items i ON i.id = ia.inbound_order_item_id
                LEFT JOIN storage_item_file_bindings item_binding ON item_binding.file_object_id = f.id
                LEFT JOIN stock_item_attributes item_attr ON item_attr.id = item_binding.item_attribute_id
                LEFT JOIN stock_item_attribute_definitions item_definition ON item_definition.id = item_attr.definition_id
                LEFT JOIN stock_items main_item ON main_item.image_file_id = f.id
                WHERE f.id = ?
                "#,
                [id.into()],
            ))
            .await?;

        row.map(decode_access_record).transpose()
    }

    /// 删除当前用户拥有且尚未绑定的临时文件元数据。
    ///
    /// 返回 false 表示记录不存在、所有者不匹配或已经被任一业务记录绑定。
    pub(crate) async fn delete_unbound_owned(
        &self,
        id: i64,
        owner_user_id: i64,
    ) -> Result<bool, DbErr> {
        let result = self
            .database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                DELETE FROM storage_file_objects
                WHERE id = ? AND owner_user_id = ?
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_inbound_file_bindings b
                      WHERE b.file_object_id = storage_file_objects.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_item_file_bindings b
                      WHERE b.file_object_id = storage_file_objects.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM stock_items item
                      WHERE item.image_file_id = storage_file_objects.id
                  )
                "#,
                [id.into(), owner_user_id.into()],
            ))
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// 查询超过指定小时数且仍未绑定的临时文件。
    pub(crate) async fn list_stale_unbound(
        &self,
        older_than_hours: i64,
    ) -> Result<Vec<StaleFileObject>, DbErr> {
        let modifier = format!("-{older_than_hours} hours");
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT f.id, f.storage_path
                FROM storage_file_objects f
                WHERE julianday(f.created_at) < julianday('now', ?)
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_inbound_file_bindings b
                      WHERE b.file_object_id = f.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_item_file_bindings b
                      WHERE b.file_object_id = f.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM stock_items item
                      WHERE item.image_file_id = f.id
                  )
                ORDER BY f.id
                "#,
                [modifier.into()],
            ))
            .await?;
        rows.into_iter().map(decode_stale_file).collect()
    }

    /// 以绑定不存在为条件删除指定临时文件元数据，避免清理竞态误删已绑定记录。
    pub(crate) async fn delete_stale_unbound(&self, id: i64) -> Result<bool, DbErr> {
        let result = self
            .database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                DELETE FROM storage_file_objects
                WHERE id = ?
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_inbound_file_bindings b
                      WHERE b.file_object_id = storage_file_objects.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM storage_item_file_bindings b
                      WHERE b.file_object_id = storage_file_objects.id
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM stock_items item
                      WHERE item.image_file_id = storage_file_objects.id
                  )
                "#,
                [id.into()],
            ))
            .await?;
        Ok(result.rows_affected() == 1)
    }

    /// 查询仍引用同一服务端相对路径的文件元数据数量。
    pub(crate) async fn count_by_storage_path(&self, path: &str) -> Result<i64, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM storage_file_objects WHERE storage_path = ?",
                [path.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("file path reference count".to_owned()))?;
        row.try_get("", "count")
    }
}

fn decode_access_record(row: QueryResult) -> Result<FileAccessRecord, DbErr> {
    Ok(FileAccessRecord {
        file: file_object::Model {
            id: row.try_get("", "id")?,
            sha256: row.try_get("", "sha256")?,
            mime_type: row.try_get("", "mime_type")?,
            size_bytes: row.try_get("", "size_bytes")?,
            storage_path: row.try_get("", "storage_path")?,
            original_name: row.try_get("", "original_name")?,
            created_at: row.try_get("", "created_at")?,
            owner_user_id: row.try_get("", "owner_user_id")?,
        },
        inbound_order_item_id: row.try_get("", "inbound_order_item_id")?,
        inbound_order_id: row.try_get("", "inbound_order_id")?,
        item_id: row.try_get("", "item_id")?,
        field_name: row.try_get("", "field_name")?,
    })
}

fn decode_stale_file(row: QueryResult) -> Result<StaleFileObject, DbErr> {
    Ok(StaleFileObject {
        id: row.try_get("", "id")?,
        storage_path: row.try_get("", "storage_path")?,
    })
}
