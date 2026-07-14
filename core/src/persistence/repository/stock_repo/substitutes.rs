//! 替代料仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装替代料关系整体替换、查询、删除和环路检测。
//! 替换操作必须在事务中同时完成关系写入和包含前后列表的审计事件写入。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionTrait};
use serde_json::json;
use std::collections::HashSet;

use super::{
    common::insert_audit_event_on_connection, StockRepository, StockSubstituteInput,
    StockSubstituteRecord,
};
use crate::persistence::repository::validation::validate_repository_input;

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 整体替换指定物品的替代料列表；替换、环路校验和前后列表审计事件必须在同一事务内完成。
    pub(crate) async fn replace_substitutes(
        &self,
        item_id: i64,
        substitutes: Vec<StockSubstituteInput>,
        user_id: Option<i64>,
    ) -> Result<Option<Vec<StockSubstituteRecord>>, DbErr>
    where
        C: TransactionTrait,
    {
        if self.find_active_item_by_id(item_id).await?.is_none() {
            return Ok(None);
        }
        validate_substitute_inputs(item_id, &substitutes)?;
        for substitute in &substitutes {
            if self
                .find_active_item_by_id(substitute.substitute_item_id)
                .await?
                .is_none()
            {
                return Err(DbErr::Custom("substitute item not found".to_owned()));
            }
            if self
                .substitute_would_create_cycle(item_id, substitute.substitute_item_id)
                .await?
            {
                return Err(DbErr::Custom("substitute cycle".to_owned()));
            }
        }

        let new_substitute_item_ids = substitutes
            .iter()
            .map(|substitute| substitute.substitute_item_id)
            .collect::<Vec<_>>();
        let transaction = self.database.begin().await?;
        let previous_substitute_item_ids =
            list_substitute_item_ids_on_connection(&transaction, item_id).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM stock_substitutes WHERE item_id = ?",
                [item_id.into()],
            ))
            .await?;
        for substitute in substitutes {
            validate_repository_input(&substitute)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_substitutes
                        (item_id, substitute_item_id, priority, notes, created_by_user_id)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item_id.into(),
                        substitute.substitute_item_id.into(),
                        substitute.priority.into(),
                        substitute.notes.into(),
                        user_id.into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "substitute",
            Some(item_id),
            if new_substitute_item_ids.is_empty() {
                "unlinked"
            } else {
                "linked"
            },
            Some(substitute_replace_details(
                &previous_substitute_item_ids,
                &new_substitute_item_ids,
            )),
        )
        .await?;
        transaction.commit().await?;

        self.list_item_substitutes(item_id).await.map(Some)
    }

    /// 查询指定物品的替代料列表；只返回未软删除的主物品和替代物品。
    pub(crate) async fn list_item_substitutes(
        &self,
        item_id: i64,
    ) -> Result<Vec<StockSubstituteRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    substitutes.item_id,
                    items.name AS item_name,
                    items.sku AS item_sku,
                    substitutes.substitute_item_id,
                    substitute_items.name AS substitute_item_name,
                    substitute_items.sku AS substitute_item_sku,
                    substitute_categories.name AS substitute_item_category_name,
                    substitute_items.image_file_id AS substitute_item_image_file_id,
                    substitute_items.unit AS substitute_item_unit,
                    substitute_items.reorder_point AS substitute_item_reorder_point,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                    CASE
                        WHEN COALESCE(SUM(batches.remaining_quantity), 0.0) <= 0 THEN 'out_of_stock'
                        WHEN substitute_items.reorder_point IS NOT NULL
                             AND COALESCE(SUM(batches.remaining_quantity), 0.0) <= substitute_items.reorder_point THEN 'reorder_due'
                        WHEN substitute_items.reorder_point IS NULL THEN 'needs_configuration'
                        ELSE 'normal'
                    END AS substitute_item_stock_state,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                FROM stock_substitutes substitutes
                JOIN stock_items items
                    ON items.id = substitutes.item_id
                   AND items.deleted_at IS NULL
                JOIN stock_items substitute_items
                    ON substitute_items.id = substitutes.substitute_item_id
                   AND substitute_items.deleted_at IS NULL
                LEFT JOIN stock_item_categories substitute_categories
                    ON substitute_categories.id = substitute_items.category_id
                   AND substitute_categories.deleted_at IS NULL
                LEFT JOIN stock_batches batches
                    ON batches.item_id = substitute_items.id
                   AND batches.remaining_quantity > 0
                WHERE substitutes.item_id = ?
                GROUP BY
                    substitutes.item_id,
                    items.name,
                    items.sku,
                    substitutes.substitute_item_id,
                    substitute_items.name,
                    substitute_items.sku,
                    substitute_categories.name,
                    substitute_items.image_file_id,
                    substitute_items.unit,
                    substitute_items.reorder_point,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                ORDER BY substitutes.priority ASC, substitutes.substitute_item_id ASC
                "#,
                [item_id.into()],
            ))
            .await?;

        rows.into_iter().map(substitute_from_row).collect()
    }

    /// 查询全部替代料关系；只返回未软删除的主物品和替代物品。
    pub(crate) async fn list_substitute_relations(
        &self,
    ) -> Result<Vec<StockSubstituteRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    substitutes.item_id,
                    items.name AS item_name,
                    items.sku AS item_sku,
                    substitutes.substitute_item_id,
                    substitute_items.name AS substitute_item_name,
                    substitute_items.sku AS substitute_item_sku,
                    substitute_categories.name AS substitute_item_category_name,
                    substitute_items.image_file_id AS substitute_item_image_file_id,
                    substitute_items.unit AS substitute_item_unit,
                    substitute_items.reorder_point AS substitute_item_reorder_point,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                    CASE
                        WHEN COALESCE(SUM(batches.remaining_quantity), 0.0) <= 0 THEN 'out_of_stock'
                        WHEN substitute_items.reorder_point IS NOT NULL
                             AND COALESCE(SUM(batches.remaining_quantity), 0.0) <= substitute_items.reorder_point THEN 'reorder_due'
                        WHEN substitute_items.reorder_point IS NULL THEN 'needs_configuration'
                        ELSE 'normal'
                    END AS substitute_item_stock_state,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                FROM stock_substitutes substitutes
                JOIN stock_items items
                    ON items.id = substitutes.item_id
                   AND items.deleted_at IS NULL
                JOIN stock_items substitute_items
                    ON substitute_items.id = substitutes.substitute_item_id
                   AND substitute_items.deleted_at IS NULL
                LEFT JOIN stock_item_categories substitute_categories
                    ON substitute_categories.id = substitute_items.category_id
                   AND substitute_categories.deleted_at IS NULL
                LEFT JOIN stock_batches batches
                    ON batches.item_id = substitute_items.id
                   AND batches.remaining_quantity > 0
                GROUP BY
                    substitutes.item_id,
                    items.name,
                    items.sku,
                    substitutes.substitute_item_id,
                    substitute_items.name,
                    substitute_items.sku,
                    substitute_categories.name,
                    substitute_items.image_file_id,
                    substitute_items.unit,
                    substitute_items.reorder_point,
                    substitutes.priority,
                    substitutes.notes,
                    substitutes.created_by_user_id,
                    substitutes.created_at
                ORDER BY items.name ASC, items.id ASC, substitutes.priority ASC, substitutes.substitute_item_id ASC
                "#
                .to_owned(),
            ))
            .await?;

        rows.into_iter().map(substitute_from_row).collect()
    }

    /// 删除单个替代料关系；返回 false 表示关系原本不存在。
    pub(crate) async fn delete_substitute_relation(
        &self,
        item_id: i64,
        substitute_item_id: i64,
        user_id: Option<i64>,
    ) -> Result<bool, DbErr>
    where
        C: TransactionTrait,
    {
        let transaction = self.database.begin().await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                DELETE FROM stock_substitutes
                WHERE item_id = ? AND substitute_item_id = ?
                "#,
                vec![item_id.into(), substitute_item_id.into()],
            ))
            .await?;
        let deleted = result.rows_affected() > 0;
        if deleted {
            insert_audit_event_on_connection(
                &transaction,
                user_id,
                "substitute",
                Some(item_id),
                "unlinked",
                Some(format!(r#"{{"substitute_item_id":{substitute_item_id}}}"#)),
            )
            .await?;
        }
        transaction.commit().await?;

        Ok(deleted)
    }

    async fn substitute_would_create_cycle(
        &self,
        item_id: i64,
        substitute_item_id: i64,
    ) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH RECURSIVE substitute_path(current_item_id) AS (
                    SELECT ?
                    UNION
                    SELECT substitutes.substitute_item_id
                    FROM stock_substitutes substitutes
                    JOIN substitute_path path
                        ON substitutes.item_id = path.current_item_id
                    JOIN stock_items items
                        ON items.id = substitutes.substitute_item_id
                       AND items.deleted_at IS NULL
                    WHERE substitutes.item_id != ?
                )
                SELECT EXISTS(
                    SELECT 1
                    FROM substitute_path
                    WHERE current_item_id = ?
                ) AS has_cycle
                "#,
                vec![substitute_item_id.into(), item_id.into(), item_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("substitute cycle check".to_owned()))?;
        let has_cycle: i64 = row.try_get("", "has_cycle")?;

        Ok(has_cycle != 0)
    }
}

async fn list_substitute_item_ids_on_connection<C>(
    connection: &C,
    item_id: i64,
) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT substitute_item_id
            FROM stock_substitutes
            WHERE item_id = ?
            ORDER BY priority ASC, substitute_item_id ASC
            "#,
            [item_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get("", "substitute_item_id"))
        .collect()
}

fn substitute_replace_details(previous: &[i64], new: &[i64]) -> String {
    let previous_set = previous.iter().copied().collect::<HashSet<_>>();
    let new_set = new.iter().copied().collect::<HashSet<_>>();
    let added = new
        .iter()
        .copied()
        .filter(|id| !previous_set.contains(id))
        .collect::<Vec<_>>();
    let removed = previous
        .iter()
        .copied()
        .filter(|id| !new_set.contains(id))
        .collect::<Vec<_>>();

    json!({
        "mode": "replace",
        "previous_substitute_item_ids": previous,
        "new_substitute_item_ids": new,
        "added_substitute_item_ids": added,
        "removed_substitute_item_ids": removed
    })
    .to_string()
}

fn validate_substitute_inputs(
    item_id: i64,
    substitutes: &[StockSubstituteInput],
) -> Result<(), DbErr> {
    let mut ids = HashSet::with_capacity(substitutes.len());
    let mut priorities = HashSet::with_capacity(substitutes.len());
    for substitute in substitutes {
        validate_repository_input(substitute)?;
        if substitute.substitute_item_id == item_id {
            return Err(DbErr::Custom("substitute self reference".to_owned()));
        }
        if !ids.insert(substitute.substitute_item_id) {
            return Err(DbErr::Custom("duplicate substitute item".to_owned()));
        }
        if !priorities.insert(substitute.priority) {
            return Err(DbErr::Custom("duplicate substitute priority".to_owned()));
        }
    }

    Ok(())
}

fn substitute_from_row(row: sea_orm::QueryResult) -> Result<StockSubstituteRecord, DbErr> {
    Ok(StockSubstituteRecord {
        item_id: row.try_get("", "item_id")?,
        item_name: row.try_get("", "item_name")?,
        item_sku: row.try_get("", "item_sku")?,
        substitute_item_id: row.try_get("", "substitute_item_id")?,
        substitute_item_name: row.try_get("", "substitute_item_name")?,
        substitute_item_sku: row.try_get("", "substitute_item_sku")?,
        substitute_item_category_name: row.try_get("", "substitute_item_category_name")?,
        substitute_item_image_file_id: row.try_get("", "substitute_item_image_file_id")?,
        substitute_item_unit: row.try_get("", "substitute_item_unit")?,
        substitute_item_reorder_point: row.try_get("", "substitute_item_reorder_point")?,
        quantity: row.try_get("", "quantity")?,
        substitute_item_stock_state: row.try_get("", "substitute_item_stock_state")?,
        priority: row.try_get("", "priority")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
    })
}
