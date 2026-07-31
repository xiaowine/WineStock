//! 库位分组、库位和移库仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装树形库位分组、具体库位、默认库位补齐和整批次移库事务。
//! 物品主数据不持有库位；当前库存位置由批次的 `location_id` 决定。

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionSession, TransactionTrait, Value,
};
use serde_json::json;

use super::{
    common::insert_audit_event_on_connection, CreateLocation, CreateLocationGroup,
    CreateLocationTransfer, StockLocationGroupRecord, StockLocationRecord,
    StockLocationTransferRecord, StockRepository, UpdateLocation, UpdateLocationGroup,
};
use crate::persistence::repository::{time::sqlite_now, validation::validate_repository_input};

#[derive(Debug, Clone, PartialEq)]
struct MovableBatchRecord {
    id: i64,
    item_id: i64,
    batch_no: String,
    location_id: i64,
    remaining_quantity: f64,
}

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 启动时补齐示例库区和示例库位；事务内二次确认，避免并发或旧数据导致重复根分组。
    /// 名称用"示例"而非"默认"，避免与入库预填用的全局默认库位（is_default）混淆。
    pub(crate) async fn ensure_default_location(&self) -> Result<(), DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        if has_active_locations_on_connection(&transaction).await? {
            transaction.commit().await?;
            return Ok(());
        }

        let now = sqlite_now(&transaction).await?;
        let group_id = if let Some(group_id) =
            find_active_root_location_group_id_by_name_on_connection(&transaction, "示例库区")
                .await?
        {
            group_id
        } else {
            let group_result = transaction
                .execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                        INSERT INTO stock_location_groups
                            (parent_id, name, sort_order, created_at, updated_at)
                        VALUES (NULL, '示例库区', 0, ?, ?)
                        "#,
                    [now.clone().into(), now.clone().into()],
                ))
                .await?;
            i64::try_from(group_result.last_insert_id())
                .map_err(|_| DbErr::Custom("location group id overflow".to_owned()))?
        };
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_locations
                    (group_id, name, notes, sort_order, created_at, updated_at)
                VALUES (?, '示例库位', NULL, 0, ?, ?)
                "#,
                vec![group_id.into(), now.clone().into(), now.into()],
            ))
            .await?;
        transaction.commit().await?;

        Ok(())
    }

    /// 查询全部未软删除库位分组，按父级和排序值返回，供服务层组装树。
    pub(crate) async fn list_active_location_groups(
        &self,
    ) -> Result<Vec<StockLocationGroupRecord>, DbErr> {
        let rows = self
            .database
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                SELECT id, parent_id, name, sort_order, created_at, updated_at
                FROM stock_location_groups
                WHERE deleted_at IS NULL
                ORDER BY parent_id IS NOT NULL ASC, parent_id ASC, sort_order ASC, id ASC
                "#
                .to_owned(),
            ))
            .await?;

        rows.into_iter().map(location_group_from_row).collect()
    }

    /// 按 ID 查询未软删除库位分组。
    pub(crate) async fn find_active_location_group_by_id(
        &self,
        id: i64,
    ) -> Result<Option<StockLocationGroupRecord>, DbErr> {
        find_active_location_group_by_id_on_connection(self.database, id).await
    }

    /// 判断同一上级分组下是否存在同名未软删除分组。
    pub(crate) async fn active_location_group_name_exists(
        &self,
        parent_id: Option<i64>,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let (where_clause, values) = location_group_name_filter(parent_id, name, except_id);
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM stock_location_groups WHERE {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location group name".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 判断目标上级是否是当前分组的子孙节点，避免移动后形成环。
    pub(crate) async fn location_group_has_descendant(
        &self,
        group_id: i64,
        descendant_id: i64,
    ) -> Result<bool, DbErr> {
        if group_id == descendant_id {
            return Ok(true);
        }
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH RECURSIVE descendants(id) AS (
                    SELECT id
                    FROM stock_location_groups
                    WHERE parent_id = ? AND deleted_at IS NULL
                    UNION ALL
                    SELECT groups.id
                    FROM stock_location_groups groups
                    JOIN descendants ON groups.parent_id = descendants.id
                    WHERE groups.deleted_at IS NULL
                )
                SELECT COUNT(*) AS count
                FROM descendants
                WHERE id = ?
                "#,
                [group_id.into(), descendant_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location descendants".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 创建库位分组并写入变更记录。
    pub(crate) async fn create_location_group(
        &self,
        input: CreateLocationGroup,
        audit_user_id: Option<i64>,
    ) -> Result<StockLocationGroupRecord, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let group = transaction
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_location_groups
                    (parent_id, name, sort_order, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                RETURNING id, parent_id, name, sort_order, created_at, updated_at
                "#,
                vec![
                    input.parent_id.into(),
                    input.name.clone().into(),
                    input.sort_order.into(),
                    now.clone().into(),
                    now.into(),
                ],
            ))
            .await?
            .map(location_group_from_row)
            .transpose()?
            .ok_or_else(|| DbErr::RecordNotFound("created stock location group".to_owned()))?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location_group",
                Some(group.id),
                "created",
                Some(location_group_audit_snapshot(&group).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(group)
    }

    /// 更新库位分组并记录普通更新或移动事件。
    pub(crate) async fn update_location_group(
        &self,
        id: i64,
        input: UpdateLocationGroup,
        audit_user_id: Option<i64>,
    ) -> Result<Option<StockLocationGroupRecord>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(previous) =
            find_active_location_group_by_id_on_connection(&transaction, id).await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        let updated = transaction
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_location_groups
                SET parent_id = ?, name = ?, sort_order = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                RETURNING id, parent_id, name, sort_order, created_at, updated_at
                "#,
                vec![
                    input.parent_id.into(),
                    input.name.into(),
                    input.sort_order.into(),
                    now.into(),
                    id.into(),
                ],
            ))
            .await?
            .map(location_group_from_row)
            .transpose()?
            .ok_or_else(|| DbErr::RecordNotFound("updated stock location group".to_owned()))?;
        if let Some(user_id) = audit_user_id {
            let action = if previous.parent_id != updated.parent_id {
                "moved"
            } else {
                "updated"
            };
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location_group",
                Some(updated.id),
                action,
                Some(location_group_update_details(&previous, &updated)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(Some(updated))
    }

    /// 软删除空库位分组并写入删除记录；调用方需先确认没有子分组和库位。
    pub(crate) async fn soft_delete_location_group(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(group) = find_active_location_group_by_id_on_connection(&transaction, id).await?
        else {
            transaction.commit().await?;
            return Ok(false);
        };
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_location_groups
                SET deleted_at = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
                vec![now.clone().into(), now.into(), id.into()],
            ))
            .await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location_group",
                Some(group.id),
                "deleted",
                Some(json!({ "previous": location_group_audit_snapshot(&group) }).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(true)
    }

    /// 查询未软删除子分组是否存在。
    pub(crate) async fn location_group_has_children(&self, id: i64) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(*) AS count
                FROM stock_location_groups
                WHERE parent_id = ? AND deleted_at IS NULL
                "#,
                [id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location group children".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 查询分组下是否仍有未软删除库位。
    pub(crate) async fn location_group_has_locations(&self, id: i64) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(*) AS count
                FROM stock_locations
                WHERE group_id = ? AND deleted_at IS NULL
                "#,
                [id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location group locations".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 查询未软删除库位列表，可按分组或自由文本筛选。
    pub(crate) async fn list_active_locations(
        &self,
        group_id: Option<i64>,
        search: Option<&str>,
    ) -> Result<Vec<StockLocationRecord>, DbErr> {
        let mut sql = r#"
            SELECT locations.id, locations.group_id, groups.name AS group_name,
                   locations.name, locations.notes, locations.sort_order,
                   locations.is_default, locations.created_at, locations.updated_at
            FROM stock_locations locations
            JOIN stock_location_groups groups
              ON groups.id = locations.group_id
             AND groups.deleted_at IS NULL
            WHERE locations.deleted_at IS NULL
        "#
        .to_owned();
        let mut values = Vec::<Value>::new();
        if let Some(group_id) = group_id {
            sql.push_str(" AND locations.group_id = ?");
            values.push(group_id.into());
        }
        if let Some(search) = search {
            sql.push_str(" AND (lower(locations.name) LIKE ? OR lower(COALESCE(locations.notes, '')) LIKE ?)");
            let like = format!("%{}%", search.to_lowercase());
            values.push(like.clone().into());
            values.push(like.into());
        }
        sql.push_str(" ORDER BY groups.sort_order ASC, groups.id ASC, locations.sort_order ASC, locations.id ASC");

        let rows = self
            .database
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                sql,
                values,
            ))
            .await?;

        rows.into_iter().map(location_from_row).collect()
    }

    /// 按 ID 查询未软删除库位。
    pub(crate) async fn find_active_location_by_id(
        &self,
        id: i64,
    ) -> Result<Option<StockLocationRecord>, DbErr> {
        find_active_location_by_id_on_connection(self.database, id).await
    }

    /// 判断未软删除库位名称是否已存在。
    pub(crate) async fn active_location_name_exists(
        &self,
        name: &str,
        except_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let mut sql =
            "SELECT COUNT(*) AS count FROM stock_locations WHERE deleted_at IS NULL AND name = ?"
                .to_owned();
        let mut values = vec![name.to_owned().into()];
        if let Some(except_id) = except_id {
            sql.push_str(" AND id != ?");
            values.push(except_id.into());
        }
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                sql,
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location name".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 创建库位并写入变更记录。
    pub(crate) async fn create_location(
        &self,
        input: CreateLocation,
        audit_user_id: Option<i64>,
    ) -> Result<StockLocationRecord, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let result = transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_locations
                    (group_id, name, notes, sort_order, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                vec![
                    input.group_id.into(),
                    input.name.clone().into(),
                    input.notes.clone().into(),
                    input.sort_order.into(),
                    now.clone().into(),
                    now.into(),
                ],
            ))
            .await?;
        let location_id = i64::try_from(result.last_insert_id())
            .map_err(|_| DbErr::Custom("location id overflow".to_owned()))?;
        let location = find_active_location_by_id_on_connection(&transaction, location_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created stock location".to_owned()))?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location",
                Some(location.id),
                "created",
                Some(location_audit_snapshot(&location).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(location)
    }

    /// 更新库位基础资料并写入变更记录。
    pub(crate) async fn update_location(
        &self,
        id: i64,
        input: UpdateLocation,
        audit_user_id: Option<i64>,
    ) -> Result<Option<StockLocationRecord>, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        let transaction = self.database.begin().await?;
        let Some(previous) = find_active_location_by_id_on_connection(&transaction, id).await?
        else {
            transaction.commit().await?;
            return Ok(None);
        };
        let now = sqlite_now(&transaction).await?;
        // “至多一个默认”由服务层事务保证：设为默认前先清除其它默认库位。
        if input.is_default == Some(true) && !previous.is_default {
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    "UPDATE stock_locations SET is_default = 0, updated_at = ? WHERE is_default = 1",
                    vec![now.clone().into()],
                ))
                .await?;
        }
        let next_is_default = input.is_default.unwrap_or(previous.is_default);
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_locations
                SET group_id = ?, name = ?, notes = ?, sort_order = ?, is_default = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
                vec![
                    input.group_id.into(),
                    input.name.into(),
                    input.notes.into(),
                    input.sort_order.into(),
                    i64::from(next_is_default).into(),
                    now.into(),
                    id.into(),
                ],
            ))
            .await?;
        let updated = find_active_location_by_id_on_connection(&transaction, id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("updated stock location".to_owned()))?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location",
                Some(updated.id),
                "updated",
                Some(location_update_details(&previous, &updated)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(Some(updated))
    }

    /// 查询库位是否仍有当前库存引用。
    pub(crate) async fn location_has_current_stock(&self, id: i64) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(*) AS count
                FROM stock_batches
                WHERE location_id = ? AND remaining_quantity > 0
                "#,
                [id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("stock location current stock".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count > 0)
    }

    /// 软删除库位并写入变更记录；调用方需先确认没有当前库存引用。
    pub(crate) async fn soft_delete_location(
        &self,
        id: i64,
        audit_user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let Some(location) = find_active_location_by_id_on_connection(&transaction, id).await?
        else {
            transaction.commit().await?;
            return Ok(false);
        };
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_locations
                SET deleted_at = ?, updated_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
                vec![now.clone().into(), now.into(), id.into()],
            ))
            .await?;
        if let Some(user_id) = audit_user_id {
            insert_audit_event_on_connection(
                &transaction,
                Some(user_id),
                "location",
                Some(location.id),
                "deleted",
                Some(json!({ "previous": location_audit_snapshot(&location) }).to_string()),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(true)
    }

    /// 整批次移库；更新批次库位、写入移库记录和变更记录必须在同一事务内完成。
    pub(crate) async fn create_location_transfer(
        &self,
        input: CreateLocationTransfer,
    ) -> Result<StockLocationTransferRecord, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        if input.from_location_id == input.to_location_id {
            return Err(DbErr::Custom(
                "location transfer target unchanged".to_owned(),
            ));
        }

        let transaction = self.database.begin().await?;
        if find_active_location_by_id_on_connection(&transaction, input.from_location_id)
            .await?
            .is_none()
            || find_active_location_by_id_on_connection(&transaction, input.to_location_id)
                .await?
                .is_none()
        {
            return Err(DbErr::Custom("stock location not found".to_owned()));
        }
        let batch = find_movable_batch_on_connection(&transaction, input.batch_id)
            .await?
            .ok_or_else(|| DbErr::Custom("stock batch not found".to_owned()))?;
        if batch.location_id != input.from_location_id {
            return Err(DbErr::Custom(
                "location transfer source mismatch".to_owned(),
            ));
        }

        let now = sqlite_now(&transaction).await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_batches
                SET location_id = ?, updated_at = ?
                WHERE id = ? AND location_id = ? AND remaining_quantity = ?
                "#,
                vec![
                    input.to_location_id.into(),
                    now.clone().into(),
                    batch.id.into(),
                    input.from_location_id.into(),
                    batch.remaining_quantity.into(),
                ],
            ))
            .await?;
        let transfer = transaction
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_location_transfers
                    (batch_id, item_id, from_location_id, to_location_id, quantity, notes, created_by_user_id, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING id, batch_id, item_id, from_location_id, to_location_id,
                          quantity, notes, created_by_user_id, created_at
                "#,
                vec![
                    batch.id.into(),
                    batch.item_id.into(),
                    input.from_location_id.into(),
                    input.to_location_id.into(),
                    batch.remaining_quantity.into(),
                    input.notes.clone().into(),
                    input.created_by_user_id.into(),
                    now.into(),
                ],
            ))
            .await?
            .map(location_transfer_from_row)
            .transpose()?
            .ok_or_else(|| DbErr::RecordNotFound("created stock location transfer".to_owned()))?;
        insert_audit_event_on_connection(
            &transaction,
            input.created_by_user_id,
            "location_transfer",
            Some(transfer.id),
            "created",
            Some(
                json!({
                    "batch_id": batch.id,
                    "batch_no": batch.batch_no,
                    "item_id": batch.item_id,
                    "from_location_id": input.from_location_id,
                    "to_location_id": input.to_location_id,
                    "quantity": batch.remaining_quantity,
                    "notes": input.notes
                })
                .to_string(),
            ),
        )
        .await?;
        transaction.commit().await?;

        Ok(transfer)
    }
}

async fn has_active_locations_on_connection<C>(connection: &C) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    let row = connection
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            SELECT COUNT(*) AS count
            FROM stock_locations
            WHERE deleted_at IS NULL
            "#
            .to_owned(),
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("stock location count".to_owned()))?;
    let count: i64 = row.try_get("", "count")?;

    Ok(count > 0)
}

async fn find_active_root_location_group_id_by_name_on_connection<C>(
    connection: &C,
    name: &str,
) -> Result<Option<i64>, DbErr>
where
    C: ConnectionTrait,
{
    connection
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id
            FROM stock_location_groups
            WHERE parent_id IS NULL AND name = ? AND deleted_at IS NULL
            "#,
            [name.into()],
        ))
        .await?
        .map(|row| row.try_get("", "id"))
        .transpose()
}

async fn find_active_location_group_by_id_on_connection<C>(
    connection: &C,
    id: i64,
) -> Result<Option<StockLocationGroupRecord>, DbErr>
where
    C: ConnectionTrait,
{
    connection
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, parent_id, name, sort_order, created_at, updated_at
            FROM stock_location_groups
            WHERE id = ? AND deleted_at IS NULL
            "#,
            [id.into()],
        ))
        .await?
        .map(location_group_from_row)
        .transpose()
}

async fn find_active_location_by_id_on_connection<C>(
    connection: &C,
    id: i64,
) -> Result<Option<StockLocationRecord>, DbErr>
where
    C: ConnectionTrait,
{
    connection
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT locations.id, locations.group_id, groups.name AS group_name,
                   locations.name, locations.notes, locations.sort_order,
                   locations.is_default, locations.created_at, locations.updated_at
            FROM stock_locations locations
            JOIN stock_location_groups groups
              ON groups.id = locations.group_id
             AND groups.deleted_at IS NULL
            WHERE locations.id = ? AND locations.deleted_at IS NULL
            "#,
            [id.into()],
        ))
        .await?
        .map(location_from_row)
        .transpose()
}

async fn find_movable_batch_on_connection<C>(
    connection: &C,
    batch_id: i64,
) -> Result<Option<MovableBatchRecord>, DbErr>
where
    C: ConnectionTrait,
{
    connection
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, item_id, batch_no, location_id, remaining_quantity
            FROM stock_batches
            WHERE id = ? AND remaining_quantity > 0
            "#,
            [batch_id.into()],
        ))
        .await?
        .map(|row| {
            Ok(MovableBatchRecord {
                id: row.try_get("", "id")?,
                item_id: row.try_get("", "item_id")?,
                batch_no: row.try_get("", "batch_no")?,
                location_id: row.try_get("", "location_id")?,
                remaining_quantity: row.try_get("", "remaining_quantity")?,
            })
        })
        .transpose()
}

fn location_group_name_filter(
    parent_id: Option<i64>,
    name: &str,
    except_id: Option<i64>,
) -> (String, Vec<Value>) {
    let mut clauses = vec!["deleted_at IS NULL".to_owned(), "name = ?".to_owned()];
    let mut values = vec![name.to_owned().into()];
    if let Some(parent_id) = parent_id {
        clauses.push("parent_id = ?".to_owned());
        values.push(parent_id.into());
    } else {
        clauses.push("parent_id IS NULL".to_owned());
    }
    if let Some(except_id) = except_id {
        clauses.push("id != ?".to_owned());
        values.push(except_id.into());
    }

    (clauses.join(" AND "), values)
}

fn location_group_from_row(row: sea_orm::QueryResult) -> Result<StockLocationGroupRecord, DbErr> {
    Ok(StockLocationGroupRecord {
        id: row.try_get("", "id")?,
        parent_id: row.try_get("", "parent_id")?,
        name: row.try_get("", "name")?,
        sort_order: row.try_get("", "sort_order")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

fn location_from_row(row: sea_orm::QueryResult) -> Result<StockLocationRecord, DbErr> {
    Ok(StockLocationRecord {
        id: row.try_get("", "id")?,
        group_id: row.try_get("", "group_id")?,
        group_name: row.try_get("", "group_name")?,
        name: row.try_get("", "name")?,
        notes: row.try_get("", "notes")?,
        sort_order: row.try_get("", "sort_order")?,
        is_default: row.try_get::<i64>("", "is_default")? != 0,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

fn location_transfer_from_row(
    row: sea_orm::QueryResult,
) -> Result<StockLocationTransferRecord, DbErr> {
    Ok(StockLocationTransferRecord {
        id: row.try_get("", "id")?,
        batch_id: row.try_get("", "batch_id")?,
        item_id: row.try_get("", "item_id")?,
        from_location_id: row.try_get("", "from_location_id")?,
        to_location_id: row.try_get("", "to_location_id")?,
        quantity: row.try_get("", "quantity")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
    })
}

fn location_group_audit_snapshot(group: &StockLocationGroupRecord) -> serde_json::Value {
    json!({
        "parent_id": group.parent_id,
        "name": group.name,
        "sort_order": group.sort_order
    })
}

fn location_audit_snapshot(location: &StockLocationRecord) -> serde_json::Value {
    json!({
        "group_id": location.group_id,
        "name": location.name,
        "notes": location.notes,
        "sort_order": location.sort_order,
        "is_default": location.is_default
    })
}

fn location_group_update_details(
    previous: &StockLocationGroupRecord,
    updated: &StockLocationGroupRecord,
) -> String {
    json!({
        "changed_fields": location_group_changed_fields(previous, updated),
        "previous": location_group_audit_snapshot(previous),
        "new": location_group_audit_snapshot(updated)
    })
    .to_string()
}

fn location_update_details(
    previous: &StockLocationRecord,
    updated: &StockLocationRecord,
) -> String {
    json!({
        "changed_fields": location_changed_fields(previous, updated),
        "previous": location_audit_snapshot(previous),
        "new": location_audit_snapshot(updated)
    })
    .to_string()
}

fn location_group_changed_fields(
    previous: &StockLocationGroupRecord,
    updated: &StockLocationGroupRecord,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if previous.parent_id != updated.parent_id {
        fields.push("parent_id");
    }
    if previous.name != updated.name {
        fields.push("name");
    }
    if previous.sort_order != updated.sort_order {
        fields.push("sort_order");
    }

    fields
}

fn location_changed_fields(
    previous: &StockLocationRecord,
    updated: &StockLocationRecord,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if previous.group_id != updated.group_id {
        fields.push("group_id");
    }
    if previous.name != updated.name {
        fields.push("name");
    }
    if previous.notes != updated.notes {
        fields.push("notes");
    }
    if previous.is_default != updated.is_default {
        fields.push("is_default");
    }
    if previous.sort_order != updated.sort_order {
        fields.push("sort_order");
    }

    fields
}
