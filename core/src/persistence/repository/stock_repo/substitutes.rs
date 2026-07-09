//! 替代料仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装替代料关系整体替换、查询、解绑和环路检测。
//! 替换操作必须在事务中同时完成关系写入和审计事件写入。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionTrait};
use std::collections::HashSet;

use super::{
    common::insert_audit_event_on_connection, BindStockSubstitute, StockRepository,
    StockSubstituteRecord,
};
use crate::persistence::repository::validation::validate_repository_input;

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 整体替换指定物品的替代料列表；替换、环路校验和审计事件必须在同一事务内完成。
    pub(crate) async fn replace_substitutes(
        &self,
        item_id: i64,
        substitutes: Vec<BindStockSubstitute>,
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

        let transaction = self.database.begin().await?;
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
            "linked",
            Some(r#"{"mode":"replace"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.list_substitutes(item_id).await.map(Some)
    }

    /// 查询指定物品的替代料列表；只返回未软删除的主物品和替代物品。
    pub(crate) async fn list_substitutes(
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
                    substitutes.substitute_item_id,
                    substitute_items.name AS substitute_item_name,
                    COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
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
                LEFT JOIN stock_batches batches
                    ON batches.item_id = substitute_items.id
                WHERE substitutes.item_id = ?
                GROUP BY
                    substitutes.item_id,
                    substitutes.substitute_item_id,
                    substitute_items.name,
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

    /// 解绑单个替代料关系；返回 false 表示关系原本不存在。
    pub(crate) async fn delete_substitute(
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

fn validate_substitute_inputs(
    item_id: i64,
    substitutes: &[BindStockSubstitute],
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
        substitute_item_id: row.try_get("", "substitute_item_id")?,
        substitute_item_name: row.try_get("", "substitute_item_name")?,
        quantity: row.try_get("", "quantity")?,
        priority: row.try_get("", "priority")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
    })
}
