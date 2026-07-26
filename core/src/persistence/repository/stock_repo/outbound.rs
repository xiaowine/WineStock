//! 出库单仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装出库单、出库明细、搜索、审批扣减、库存流水和审计写入。
//! 创建出库单不扣减库存，审批出库单才按指定批次或 FIFO 扣减。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionTrait, Value};

use super::{
    common::{current_item_quantity_on_connection, insert_audit_event_on_connection, json_string},
    search, CreateOutboundOrder, ListOutboundOrders, OutboundOrderDetail, OutboundOrderItemRecord,
    OutboundOrderRecord, Page, StockRepository,
};
use crate::persistence::repository::{time::sqlite_now, validation::validate_repository_input};

#[derive(Debug, Clone, PartialEq)]
struct StockBatchForDeduction {
    id: i64,
    location_id: i64,
    remaining_quantity: f64,
    unit_cost: f64,
}

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建 pending 出库单和明细；创建阶段不扣减库存。
    pub(crate) async fn create_outbound_order(
        &self,
        input: CreateOutboundOrder,
    ) -> Result<OutboundOrderDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        if input.items.is_empty() {
            return Err(DbErr::Custom(
                "outbound order items must not be empty".to_owned(),
            ));
        }

        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_outbound_orders
                    (destination, status, notes, created_by_user_id, created_at, updated_at)
                VALUES (?, 'pending', ?, ?, ?, ?)
                "#,
                vec![
                    input.destination.clone().into(),
                    input.notes.into(),
                    input.created_by_user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                ],
            ))
            .await?;
        let order_id = i64::try_from(result.last_insert_id())
            .map_err(|_| DbErr::Custom("outbound order id overflow".to_owned()))?;

        for item in &input.items {
            validate_repository_input(item)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_outbound_order_items
                        (order_id, item_id, quantity, batch_id, location_id, created_at)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        order_id.into(),
                        item.item_id.into(),
                        item.quantity.into(),
                        item.batch_id.into(),
                        item.location_id.into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            input.created_by_user_id,
            "outbound",
            Some(order_id),
            "created",
            Some(format!(
                r#"{{"destination":{},"item_count":{}}}"#,
                json_string(&input.destination),
                input.items.len()
            )),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(order_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created outbound order".to_owned()))
    }

    /// 查询出库单详情。
    pub(crate) async fn find_outbound_order_by_id(
        &self,
        id: i64,
    ) -> Result<Option<OutboundOrderDetail>, DbErr> {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        let items = list_outbound_items_on_connection(self.database, id).await?;

        Ok(Some(OutboundOrderDetail { order, items }))
    }

    /// 分页查询出库单，支持物品、创建时间和出库历史自由搜索筛选。
    pub(crate) async fn list_outbound_orders(
        &self,
        input: ListOutboundOrders,
    ) -> Result<Page<OutboundOrderDetail>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_outbound_orders(&input).await?;
        let rows = self.query_outbound_orders(&input, limit, offset).await?;
        let mut items = Vec::with_capacity(rows.len());
        for order in rows {
            let order_items = list_outbound_items_on_connection(self.database, order.id).await?;
            items.push(OutboundOrderDetail {
                order,
                items: order_items,
            });
        }

        Ok(Page { items, total })
    }

    /// 审批 pending 出库单；指定批次或 FIFO 扣减、库存流水和审计事件必须在同一事务内完成。
    pub(crate) async fn approve_outbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<OutboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("outbound order is not pending".to_owned()));
        }
        let order_items = list_outbound_items_on_connection(self.database, id).await?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_outbound_orders
                SET status = 'approved',
                    approved_by_user_id = ?,
                    approved_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;

        for item in &order_items {
            deduct_outbound_item_on_connection(&transaction, item, user_id, &now).await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "outbound",
            Some(id),
            "approved",
            Some(format!(r#"{{"item_count":{}}}"#, order_items.len())),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(id).await
    }

    /// 拒绝 pending 出库单；拒绝不扣减库存。
    pub(crate) async fn reject_outbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<OutboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_outbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("outbound order is not pending".to_owned()));
        }
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_outbound_orders
                SET status = 'rejected',
                    rejected_by_user_id = ?,
                    rejected_at = ?,
                    updated_at = ?
                WHERE id = ? AND status = 'pending'
                "#,
                vec![
                    user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                    id.into(),
                ],
            ))
            .await?;
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "outbound",
            Some(id),
            "rejected",
            Some(r#"{"reason":"rejected_by_user"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.find_outbound_order_by_id(id).await
    }

    async fn find_outbound_order_record_by_id(
        &self,
        id: i64,
    ) -> Result<Option<OutboundOrderRecord>, DbErr> {
        self.database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                outbound_order_select_sql("WHERE id = ?"),
                [id.into()],
            ))
            .await?
            .map(outbound_order_from_row)
            .transpose()
    }

    async fn count_outbound_orders(&self, input: &ListOutboundOrders) -> Result<u64, DbErr> {
        let (where_clause, values) = outbound_order_filters(input);
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM stock_outbound_orders {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("outbound order count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_outbound_orders(
        &self,
        input: &ListOutboundOrders,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OutboundOrderRecord>, DbErr> {
        let (where_clause, mut values) = outbound_order_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "{} {where_clause} ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
                    outbound_order_select_sql("")
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(outbound_order_from_row).collect()
    }
}
fn outbound_order_filters(input: &ListOutboundOrders) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(item_id) = input.item_id {
        clauses.push(
            "EXISTS (SELECT 1 FROM stock_outbound_order_items items WHERE items.order_id = stock_outbound_orders.id AND items.item_id = ?)".to_owned(),
        );
        values.push(item_id.into());
    }
    if let Some(status) = input.status.as_ref() {
        clauses.push("status = ?".to_owned());
        values.push(status.clone().into());
    }
    if let Some(date_from) = input.date_from.as_ref() {
        clauses.push("created_at >= ?".to_owned());
        values.push(date_from.clone().into());
    }
    if let Some(date_to) = input.date_to.as_ref() {
        clauses.push("created_at <= ?".to_owned());
        values.push(date_to.clone().into());
    }
    if let Some(search) = input.search.as_ref() {
        let search_like = format!("%{}%", search.to_lowercase());
        search::append_outbound_search_filter(&mut clauses, &mut values, &search_like);
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn outbound_order_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, destination, status, notes, created_by_user_id, approved_by_user_id,
               rejected_by_user_id, created_at, updated_at, approved_at, rejected_at
        FROM stock_outbound_orders
        {where_clause}
        "#
    )
}

fn outbound_order_from_row(row: sea_orm::QueryResult) -> Result<OutboundOrderRecord, DbErr> {
    Ok(OutboundOrderRecord {
        id: row.try_get("", "id")?,
        destination: row.try_get("", "destination")?,
        status: row.try_get("", "status")?,
        notes: row.try_get("", "notes")?,
        created_by_user_id: row.try_get("", "created_by_user_id")?,
        approved_by_user_id: row.try_get("", "approved_by_user_id")?,
        rejected_by_user_id: row.try_get("", "rejected_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
        approved_at: row.try_get("", "approved_at")?,
        rejected_at: row.try_get("", "rejected_at")?,
    })
}
async fn list_outbound_items_on_connection<C>(
    connection: &C,
    order_id: i64,
) -> Result<Vec<OutboundOrderItemRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT outbound_items.id,
                   outbound_items.order_id,
                   outbound_items.item_id,
                   items.name AS item_name, items.sku AS item_sku, items.unit AS item_unit, items.image_file_id AS item_image_file_id,
                   outbound_items.quantity,
                   outbound_items.batch_id,
                   outbound_items.location_id,
                   locations.name AS location_name,
                   outbound_items.created_at
            FROM stock_outbound_order_items outbound_items
            JOIN stock_items items ON items.id = outbound_items.item_id
            LEFT JOIN stock_locations locations ON locations.id = outbound_items.location_id
            WHERE outbound_items.order_id = ?
            ORDER BY outbound_items.id ASC
            "#,
            [order_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| {
            Ok(OutboundOrderItemRecord {
                id: row.try_get("", "id")?,
                order_id: row.try_get("", "order_id")?,
                item_id: row.try_get("", "item_id")?,
                item_name: row.try_get("", "item_name")?,
                item_sku: row.try_get("", "item_sku")?,
                item_unit: row.try_get("", "item_unit")?,
                item_image_file_id: row.try_get("", "item_image_file_id")?,
                quantity: row.try_get("", "quantity")?,
                batch_id: row.try_get("", "batch_id")?,
                location_id: row.try_get("", "location_id")?,
                location_name: row.try_get("", "location_name")?,
                created_at: row.try_get("", "created_at")?,
            })
        })
        .collect()
}

async fn deduct_outbound_item_on_connection<C>(
    connection: &C,
    item: &OutboundOrderItemRecord,
    user_id: Option<i64>,
    now: &str,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let batches = if let Some(batch_id) = item.batch_id {
        vec![find_batch_for_deduction_on_connection(
            connection,
            item.item_id,
            batch_id,
            item.location_id,
        )
        .await?
        .ok_or_else(|| DbErr::Custom("insufficient stock".to_owned()))?]
    } else {
        list_fifo_batches_for_deduction_on_connection(connection, item.item_id, item.location_id)
            .await?
    };

    let mut remaining_to_deduct = item.quantity;
    for batch in batches {
        if remaining_to_deduct <= 0.0 {
            break;
        }
        let deducted = remaining_to_deduct.min(batch.remaining_quantity);
        let new_remaining = batch.remaining_quantity - deducted;
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_batches
                SET remaining_quantity = ?, updated_at = ?
                WHERE id = ? AND remaining_quantity = ?
                "#,
                vec![
                    new_remaining.into(),
                    now.to_owned().into(),
                    batch.id.into(),
                    batch.remaining_quantity.into(),
                ],
            ))
            .await?;
        remaining_to_deduct -= deducted;
        let balance_after = current_item_quantity_on_connection(connection, item.item_id).await?;
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_movements
                    (item_id, batch_id, movement_type, quantity_delta, unit_cost, balance_after, location_id, outbound_order_item_id, created_by_user_id, created_at)
                VALUES (?, ?, 'outbound', ?, ?, ?, ?, ?, ?, ?)
                "#,
                vec![
                    item.item_id.into(),
                    batch.id.into(),
                    (-deducted).into(),
                    batch.unit_cost.into(),
                    balance_after.into(),
                    batch.location_id.into(),
                    item.id.into(),
                    user_id.into(),
                    now.to_owned().into(),
                ],
            ))
            .await?;
    }

    if remaining_to_deduct > 0.000_000_1 {
        Err(DbErr::Custom("insufficient stock".to_owned()))
    } else {
        Ok(())
    }
}

async fn find_batch_for_deduction_on_connection<C>(
    connection: &C,
    item_id: i64,
    batch_id: i64,
    location_id: Option<i64>,
) -> Result<Option<StockBatchForDeduction>, DbErr>
where
    C: ConnectionTrait,
{
    let mut sql = r#"
            SELECT id, location_id, remaining_quantity, unit_cost
            FROM stock_batches
            WHERE id = ? AND item_id = ? AND remaining_quantity > 0
            "#
    .to_owned();
    let mut values = vec![batch_id.into(), item_id.into()];
    if let Some(location_id) = location_id {
        sql.push_str(" AND location_id = ?");
        values.push(location_id.into());
    }
    connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            values,
        ))
        .await?
        .map(batch_for_deduction_from_row)
        .transpose()
}

async fn list_fifo_batches_for_deduction_on_connection<C>(
    connection: &C,
    item_id: i64,
    location_id: Option<i64>,
) -> Result<Vec<StockBatchForDeduction>, DbErr>
where
    C: ConnectionTrait,
{
    let mut sql = r#"
            SELECT id, location_id, remaining_quantity, unit_cost
            FROM stock_batches
            WHERE item_id = ? AND remaining_quantity > 0
            "#
    .to_owned();
    let mut values = vec![item_id.into()];
    if let Some(location_id) = location_id {
        sql.push_str(" AND location_id = ?");
        values.push(location_id.into());
    }
    sql.push_str(" ORDER BY expires_at IS NULL ASC, expires_at ASC, received_at ASC, id ASC");
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            values,
        ))
        .await?;

    rows.into_iter().map(batch_for_deduction_from_row).collect()
}

fn batch_for_deduction_from_row(
    row: sea_orm::QueryResult,
) -> Result<StockBatchForDeduction, DbErr> {
    Ok(StockBatchForDeduction {
        id: row.try_get("", "id")?,
        location_id: row.try_get("", "location_id")?,
        remaining_quantity: row.try_get("", "remaining_quantity")?,
        unit_cost: row.try_get("", "unit_cost")?,
    })
}
