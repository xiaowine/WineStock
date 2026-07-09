//! 入库单仓储操作。
//!
//! 本模块属于 `core` 持久化层，封装入库单、入库明细、审批批次、库存流水和审计写入。
//! 创建入库单不改变库存，审批入库单才写入批次和流水。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionTrait, Value};

use super::{
    common::{current_item_quantity_on_connection, insert_audit_event_on_connection, json_string},
    search, CreateInboundOrder, InboundOrderDetail, InboundOrderItemRecord, InboundOrderRecord,
    ListInboundOrders, Page, StockRepository,
};
use crate::persistence::repository::{time::sqlite_now, validation::validate_repository_input};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建 pending 入库单和明细；创建阶段不改变库存。
    pub(crate) async fn create_inbound_order(
        &self,
        input: CreateInboundOrder,
    ) -> Result<InboundOrderDetail, DbErr>
    where
        C: TransactionTrait,
    {
        validate_repository_input(&input)?;
        if input.items.is_empty() {
            return Err(DbErr::Custom(
                "inbound order items must not be empty".to_owned(),
            ));
        }

        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        let result = transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO stock_inbound_orders
                    (source, status, notes, created_by_user_id, created_at, updated_at)
                VALUES (?, 'pending', ?, ?, ?, ?)
                "#,
                vec![
                    input.source.clone().into(),
                    input.notes.into(),
                    input.created_by_user_id.into(),
                    now.clone().into(),
                    now.clone().into(),
                ],
            ))
            .await?;
        let order_id = i64::try_from(result.last_insert_id())
            .map_err(|_| DbErr::Custom("inbound order id overflow".to_owned()))?;

        for item in &input.items {
            validate_repository_input(item)?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_inbound_order_items
                        (order_id, item_id, quantity, unit_price, location, batch_no, expires_at, ext_attributes_json, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        order_id.into(),
                        item.item_id.into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        item.location.clone().into(),
                        item.batch_no.clone().into(),
                        item.expires_at.clone().into(),
                        item.ext_attributes_json.clone().into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            input.created_by_user_id,
            "inbound",
            Some(order_id),
            "created",
            Some(format!(
                r#"{{"source":{},"item_count":{}}}"#,
                json_string(&input.source),
                input.items.len()
            )),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(order_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created inbound order".to_owned()))
    }

    /// 查询入库单详情。
    pub(crate) async fn find_inbound_order_by_id(
        &self,
        id: i64,
    ) -> Result<Option<InboundOrderDetail>, DbErr> {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        let items = list_inbound_items_on_connection(self.database, id).await?;

        Ok(Some(InboundOrderDetail { order, items }))
    }

    /// 分页查询入库单，支持物品、创建时间和自由搜索筛选。
    pub(crate) async fn list_inbound_orders(
        &self,
        input: ListInboundOrders,
    ) -> Result<Page<InboundOrderDetail>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_inbound_orders(&input).await?;
        let rows = self.query_inbound_orders(&input, limit, offset).await?;
        let mut items = Vec::with_capacity(rows.len());
        for order in rows {
            let order_items = list_inbound_items_on_connection(self.database, order.id).await?;
            items.push(InboundOrderDetail {
                order,
                items: order_items,
            });
        }

        Ok(Page { items, total })
    }

    /// 审批 pending 入库单；状态、批次、库存流水和审计事件必须在同一事务内完成。
    pub(crate) async fn approve_inbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<InboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("inbound order is not pending".to_owned()));
        }
        let order_items = list_inbound_items_on_connection(self.database, id).await?;
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_inbound_orders
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
            let batch_no = item
                .batch_no
                .clone()
                .unwrap_or_else(|| format!("IN-{id}-{}", item.id));
            let batch_result = transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_batches
                        (item_id, inbound_order_item_id, batch_no, location, initial_quantity, remaining_quantity, unit_cost, received_at, expires_at, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item.item_id.into(),
                        item.id.into(),
                        batch_no.into(),
                        item.location.clone().into(),
                        item.quantity.into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        now.clone().into(),
                        item.expires_at.clone().into(),
                        now.clone().into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
            let balance_after =
                current_item_quantity_on_connection(&transaction, item.item_id).await?;
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    r#"
                    INSERT INTO stock_movements
                        (item_id, batch_id, movement_type, quantity_delta, unit_cost, balance_after, inbound_order_item_id, created_by_user_id, created_at)
                    VALUES (?, ?, 'inbound', ?, ?, ?, ?, ?, ?)
                    "#,
                    vec![
                        item.item_id.into(),
                        i64::try_from(batch_result.last_insert_id())
                            .map_err(|_| DbErr::Custom("stock batch id overflow".to_owned()))?
                            .into(),
                        item.quantity.into(),
                        item.unit_price.into(),
                        balance_after.into(),
                        item.id.into(),
                        user_id.into(),
                        now.clone().into(),
                    ],
                ))
                .await?;
        }
        insert_audit_event_on_connection(
            &transaction,
            user_id,
            "inbound",
            Some(id),
            "approved",
            Some(format!(r#"{{"item_count":{}}}"#, order_items.len())),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(id).await
    }

    /// 拒绝 pending 入库单；拒绝不改变库存。
    pub(crate) async fn reject_inbound_order(
        &self,
        id: i64,
        user_id: Option<i64>,
    ) -> Result<Option<InboundOrderDetail>, DbErr>
    where
        C: TransactionTrait,
    {
        let Some(order) = self.find_inbound_order_record_by_id(id).await? else {
            return Ok(None);
        };
        if order.status != "pending" {
            return Err(DbErr::Custom("inbound order is not pending".to_owned()));
        }
        let transaction = self.database.begin().await?;
        let now = sqlite_now(&transaction).await?;
        transaction
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                UPDATE stock_inbound_orders
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
            "inbound",
            Some(id),
            "rejected",
            Some(r#"{"reason":"rejected_by_user"}"#.to_owned()),
        )
        .await?;
        transaction.commit().await?;

        self.find_inbound_order_by_id(id).await
    }

    async fn find_inbound_order_record_by_id(
        &self,
        id: i64,
    ) -> Result<Option<InboundOrderRecord>, DbErr> {
        self.database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                inbound_order_select_sql("WHERE id = ?"),
                [id.into()],
            ))
            .await?
            .map(inbound_order_from_row)
            .transpose()
    }

    async fn count_inbound_orders(&self, input: &ListInboundOrders) -> Result<u64, DbErr> {
        let (where_clause, values) = inbound_order_filters(input);
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM stock_inbound_orders {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("inbound order count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_inbound_orders(
        &self,
        input: &ListInboundOrders,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<InboundOrderRecord>, DbErr> {
        let (where_clause, mut values) = inbound_order_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "{} {where_clause} ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
                    inbound_order_select_sql("")
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(inbound_order_from_row).collect()
    }
}

fn inbound_order_filters(input: &ListInboundOrders) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(item_id) = input.item_id {
        clauses.push(
            "EXISTS (SELECT 1 FROM stock_inbound_order_items items WHERE items.order_id = stock_inbound_orders.id AND items.item_id = ?)"
                .to_owned(),
        );
        values.push(item_id.into());
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
        search::append_inbound_search_filter(&mut clauses, &mut values, &search_like);
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn inbound_order_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, source, status, notes, created_by_user_id, approved_by_user_id,
               rejected_by_user_id, created_at, updated_at, approved_at, rejected_at
        FROM stock_inbound_orders
        {where_clause}
        "#
    )
}

fn inbound_order_from_row(row: sea_orm::QueryResult) -> Result<InboundOrderRecord, DbErr> {
    Ok(InboundOrderRecord {
        id: row.try_get("", "id")?,
        source: row.try_get("", "source")?,
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
async fn list_inbound_items_on_connection<C>(
    connection: &C,
    order_id: i64,
) -> Result<Vec<InboundOrderItemRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = connection
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT id, order_id, item_id, quantity, unit_price, location, batch_no,
                   expires_at, ext_attributes_json, created_at
            FROM stock_inbound_order_items
            WHERE order_id = ?
            ORDER BY id ASC
            "#,
            [order_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| {
            Ok(InboundOrderItemRecord {
                id: row.try_get("", "id")?,
                order_id: row.try_get("", "order_id")?,
                item_id: row.try_get("", "item_id")?,
                quantity: row.try_get("", "quantity")?,
                unit_price: row.try_get("", "unit_price")?,
                location: row.try_get("", "location")?,
                batch_no: row.try_get("", "batch_no")?,
                expires_at: row.try_get("", "expires_at")?,
                ext_attributes_json: row.try_get("", "ext_attributes_json")?,
                created_at: row.try_get("", "created_at")?,
            })
        })
        .collect()
}
