//! 库存搜索和筛选值查询。
//!
//! 本模块属于 `stock` repository 的查询子模块，负责库存物品、入库历史和出库历史的自由搜索 SQL、
//! 筛选值聚合 SQL。它不拥有 HTTP DTO，也不直接处理权限或分页默认值。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, Value};

use super::StockRepository;

/// 筛选字段聚合记录，供 stock 服务层投影为 HTTP 响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockFilterFieldRecord {
    /// 前端使用的稳定字段 key，例如 `base:unit` 或 `template:品牌`。
    pub key: String,

    /// 字段展示名称。
    pub label: String,

    /// 字段来源，当前为 `base` 或 `template`。
    pub source: String,

    /// 字段值类型；同名模板字段跨模板类型不一致时为 `mixed`。
    pub value_type: String,

    /// 当前字段下可选值及命中计数。
    pub values: Vec<StockFilterValueRecord>,
}

/// 单个筛选值聚合记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockFilterValueRecord {
    /// 已归一化为字符串的筛选值。
    pub value: String,

    /// 命中数量；物品筛选值按去重物品计数，入库/出库筛选值按去重单据计数。
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawFilterValueRow {
    field_key: String,
    field_label: String,
    field_source: String,
    field_value_type: String,
    field_value: String,
    value_count: u64,
}

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 查询当前库存视角下的物品列表筛选值；只统计仍有余额的批次。
    pub(crate) async fn list_item_filter_values(
        &self,
    ) -> Result<Vec<StockFilterFieldRecord>, DbErr> {
        let mut rows =
            query_filter_value_rows(self.database, item_base_filter_values_sql()).await?;
        rows.extend(
            query_filter_value_rows(self.database, item_template_filter_values_sql()).await?,
        );

        Ok(group_filter_rows(rows))
    }

    /// 查询入库历史视角下的筛选值；不受当前库存余额影响。
    pub(crate) async fn list_inbound_filter_values(
        &self,
    ) -> Result<Vec<StockFilterFieldRecord>, DbErr> {
        let mut rows =
            query_filter_value_rows(self.database, inbound_base_filter_values_sql()).await?;
        rows.extend(
            query_filter_value_rows(self.database, inbound_template_filter_values_sql()).await?,
        );

        Ok(group_filter_rows(rows))
    }

    /// 查询出库历史视角下的筛选值；批次属性从指定批次或审批流水反查。
    pub(crate) async fn list_outbound_filter_values(
        &self,
    ) -> Result<Vec<StockFilterFieldRecord>, DbErr> {
        let mut rows =
            query_filter_value_rows(self.database, outbound_base_filter_values_sql()).await?;
        rows.extend(
            query_filter_value_rows(self.database, outbound_template_filter_values_sql()).await?,
        );

        Ok(group_filter_rows(rows))
    }
}

/// 给物品列表追加自由搜索条件；模板值只从当前有库存批次追溯。
pub(super) fn append_item_search_filter(
    sql: &mut String,
    values: &mut Vec<Value>,
    search_like: &str,
) {
    let json_each = json_each_object("inbound_items.ext_attributes_json");
    let json_predicate = json_scalar_predicate("json_values");
    let json_value = json_scalar_value("json_values");
    sql.push_str(&format!(
        r#"
        AND (
            lower(stock_items.name) LIKE ?
            OR lower(stock_items.sku) LIKE ?
            OR lower(stock_items.unit) LIKE ?
            OR lower(COALESCE(stock_items.description, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_templates templates
                WHERE templates.id = stock_items.category_id
                  AND templates.deleted_at IS NULL
                  AND (
                      lower(templates.name) LIKE ?
                      OR lower(COALESCE(templates.description, '')) LIKE ?
                  )
            )
            OR EXISTS (
                SELECT 1
                FROM stock_batches batches
                JOIN stock_inbound_order_items inbound_items
                  ON inbound_items.id = batches.inbound_order_item_id
                JOIN {json_each} AS json_values
                WHERE batches.item_id = stock_items.id
                  AND batches.remaining_quantity > 0
                  AND {json_predicate}
                  AND lower({json_value}) LIKE ?
            )
        )
        "#
    ));
    for _ in 0..7 {
        values.push(search_like.into());
    }
}

/// 给入库历史追加自由搜索条件；使用 EXISTS 避免多明细把入库单放大。
pub(super) fn append_inbound_search_filter(
    clauses: &mut Vec<String>,
    values: &mut Vec<Value>,
    search_like: &str,
) {
    let json_each = json_each_object("inbound_items.ext_attributes_json");
    let json_predicate = json_scalar_predicate("json_values");
    let json_value = json_scalar_value("json_values");
    clauses.push(format!(
        r#"
        (
            lower(stock_inbound_orders.source) LIKE ?
            OR lower(stock_inbound_orders.status) LIKE ?
            OR lower(COALESCE(stock_inbound_orders.notes, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_inbound_order_items inbound_items
                LEFT JOIN stock_items matched_items ON matched_items.id = inbound_items.item_id
                WHERE inbound_items.order_id = stock_inbound_orders.id
                  AND (
                      lower(COALESCE(inbound_items.location, '')) LIKE ?
                      OR lower(COALESCE(inbound_items.batch_no, '')) LIKE ?
                      OR lower(COALESCE(inbound_items.expires_at, '')) LIKE ?
                      OR lower(COALESCE(matched_items.name, '')) LIKE ?
                      OR lower(COALESCE(matched_items.sku, '')) LIKE ?
                      OR lower(COALESCE(matched_items.unit, '')) LIKE ?
                      OR lower(COALESCE(matched_items.description, '')) LIKE ?
                      OR EXISTS (
                          SELECT 1
                          FROM {json_each} AS json_values
                          WHERE {json_predicate}
                            AND lower({json_value}) LIKE ?
                      )
                  )
            )
        )
        "#
    ));
    for _ in 0..11 {
        values.push(search_like.into());
    }
}

/// 给出库历史追加自由搜索条件；批次和模板值从指定批次或审批流水反查。
pub(super) fn append_outbound_search_filter(
    clauses: &mut Vec<String>,
    values: &mut Vec<Value>,
    search_like: &str,
) {
    let json_each = json_each_object("inbound_items.ext_attributes_json");
    let json_predicate = json_scalar_predicate("json_values");
    let json_value = json_scalar_value("json_values");
    clauses.push(format!(
        r#"
        (
            lower(stock_outbound_orders.destination) LIKE ?
            OR lower(stock_outbound_orders.status) LIKE ?
            OR lower(COALESCE(stock_outbound_orders.notes, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_outbound_order_items outbound_items
                LEFT JOIN stock_items matched_items ON matched_items.id = outbound_items.item_id
                WHERE outbound_items.order_id = stock_outbound_orders.id
                  AND (
                      lower(COALESCE(outbound_items.location, '')) LIKE ?
                      OR lower(COALESCE(matched_items.name, '')) LIKE ?
                      OR lower(COALESCE(matched_items.sku, '')) LIKE ?
                      OR lower(COALESCE(matched_items.unit, '')) LIKE ?
                      OR lower(COALESCE(matched_items.description, '')) LIKE ?
                      OR EXISTS (
                          SELECT 1
                          FROM stock_batches batches
                          LEFT JOIN stock_inbound_order_items inbound_items
                            ON inbound_items.id = batches.inbound_order_item_id
                          WHERE (
                              batches.id = outbound_items.batch_id
                              OR EXISTS (
                                  SELECT 1
                                  FROM stock_movements movements
                                  WHERE movements.outbound_order_item_id = outbound_items.id
                                    AND movements.batch_id = batches.id
                                    AND movements.movement_type = 'outbound'
                              )
                          )
                            AND (
                                lower(COALESCE(batches.batch_no, '')) LIKE ?
                                OR lower(COALESCE(batches.expires_at, '')) LIKE ?
                                OR EXISTS (
                                    SELECT 1
                                    FROM {json_each} AS json_values
                                    WHERE {json_predicate}
                                      AND lower({json_value}) LIKE ?
                                )
                            )
                      )
                  )
            )
        )
        "#
    ));
    for _ in 0..11 {
        values.push(search_like.into());
    }
}

async fn query_filter_value_rows<C>(
    database: &C,
    sql: String,
) -> Result<Vec<RawFilterValueRow>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = database
        .query_all(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await?;

    rows.into_iter()
        .map(|row| {
            let value_count: i64 = row.try_get("", "value_count")?;
            Ok(RawFilterValueRow {
                field_key: row.try_get("", "field_key")?,
                field_label: row.try_get("", "field_label")?,
                field_source: row.try_get("", "field_source")?,
                field_value_type: row.try_get("", "field_value_type")?,
                field_value: row.try_get("", "field_value")?,
                value_count: value_count as u64,
            })
        })
        .collect()
}

fn group_filter_rows(rows: Vec<RawFilterValueRow>) -> Vec<StockFilterFieldRecord> {
    let mut fields: Vec<StockFilterFieldRecord> = Vec::new();
    for row in rows {
        let value = StockFilterValueRecord {
            value: row.field_value,
            count: row.value_count,
        };
        if let Some(field) = fields.last_mut().filter(|field| field.key == row.field_key) {
            field.values.push(value);
            continue;
        }

        fields.push(StockFilterFieldRecord {
            key: row.field_key,
            label: row.field_label,
            source: row.field_source,
            value_type: row.field_value_type,
            values: vec![value],
        });
    }

    fields
}

fn item_base_filter_values_sql() -> String {
    r#"
    SELECT 'base:category' AS field_key,
           '所属模板' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           templates.name AS field_value,
           COUNT(DISTINCT items.id) AS value_count,
           10 AS field_order
    FROM stock_batches batches
    JOIN stock_items items ON items.id = batches.item_id AND items.deleted_at IS NULL
    JOIN stock_templates templates ON templates.id = items.category_id AND templates.deleted_at IS NULL
    WHERE batches.remaining_quantity > 0
      AND trim(templates.name) <> ''
    GROUP BY templates.name

    UNION ALL

    SELECT 'base:unit' AS field_key,
           '计量单位' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           items.unit AS field_value,
           COUNT(DISTINCT items.id) AS value_count,
           20 AS field_order
    FROM stock_batches batches
    JOIN stock_items items ON items.id = batches.item_id AND items.deleted_at IS NULL
    WHERE batches.remaining_quantity > 0
      AND trim(items.unit) <> ''
    GROUP BY items.unit

    UNION ALL

    SELECT 'base:location' AS field_key,
           '库位' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           COALESCE(inbound_items.location, batches.location) AS field_value,
           COUNT(DISTINCT items.id) AS value_count,
           30 AS field_order
    FROM stock_batches batches
    JOIN stock_items items ON items.id = batches.item_id AND items.deleted_at IS NULL
    LEFT JOIN stock_inbound_order_items inbound_items ON inbound_items.id = batches.inbound_order_item_id
    WHERE batches.remaining_quantity > 0
      AND COALESCE(inbound_items.location, batches.location) IS NOT NULL
      AND trim(COALESCE(inbound_items.location, batches.location)) <> ''
    GROUP BY COALESCE(inbound_items.location, batches.location)

    ORDER BY field_order ASC, field_label ASC, value_count DESC, field_value ASC
    "#
    .to_owned()
}

fn item_template_filter_values_sql() -> String {
    template_filter_values_sql(
        "items.id",
        r#"
        FROM stock_batches batches
        JOIN stock_items items ON items.id = batches.item_id AND items.deleted_at IS NULL
        JOIN stock_template_fields fields
          ON fields.template_id = items.category_id
         AND fields.searchable = 1
        JOIN stock_inbound_order_items inbound_items
          ON inbound_items.id = batches.inbound_order_item_id
        "#,
        "batches.remaining_quantity > 0",
    )
}

fn inbound_base_filter_values_sql() -> String {
    r#"
    SELECT 'base:source' AS field_key,
           '入库来源' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           orders.source AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           10 AS field_order
    FROM stock_inbound_orders orders
    WHERE trim(orders.source) <> ''
    GROUP BY orders.source

    UNION ALL

    SELECT 'base:status' AS field_key,
           '入库状态' AS field_label,
           'base' AS field_source,
           'select' AS field_value_type,
           orders.status AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           20 AS field_order
    FROM stock_inbound_orders orders
    WHERE trim(orders.status) <> ''
    GROUP BY orders.status

    UNION ALL

    SELECT 'base:item' AS field_key,
           '物品名称' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           items.name AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           30 AS field_order
    FROM stock_inbound_orders orders
    JOIN stock_inbound_order_items inbound_items ON inbound_items.order_id = orders.id
    JOIN stock_items items ON items.id = inbound_items.item_id
    WHERE trim(items.name) <> ''
    GROUP BY items.name

    UNION ALL

    SELECT 'base:sku' AS field_key,
           '物品 SKU' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           items.sku AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           40 AS field_order
    FROM stock_inbound_orders orders
    JOIN stock_inbound_order_items inbound_items ON inbound_items.order_id = orders.id
    JOIN stock_items items ON items.id = inbound_items.item_id
    WHERE trim(items.sku) <> ''
    GROUP BY items.sku

    UNION ALL

    SELECT 'base:location' AS field_key,
           '库位' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           inbound_items.location AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           50 AS field_order
    FROM stock_inbound_orders orders
    JOIN stock_inbound_order_items inbound_items ON inbound_items.order_id = orders.id
    WHERE inbound_items.location IS NOT NULL
      AND trim(inbound_items.location) <> ''
    GROUP BY inbound_items.location

    UNION ALL

    SELECT 'base:batch_no' AS field_key,
           '批次号' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           inbound_items.batch_no AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           60 AS field_order
    FROM stock_inbound_orders orders
    JOIN stock_inbound_order_items inbound_items ON inbound_items.order_id = orders.id
    WHERE inbound_items.batch_no IS NOT NULL
      AND trim(inbound_items.batch_no) <> ''
    GROUP BY inbound_items.batch_no

    ORDER BY field_order ASC, field_label ASC, value_count DESC, field_value ASC
    "#
    .to_owned()
}

fn inbound_template_filter_values_sql() -> String {
    template_filter_values_sql(
        "orders.id",
        r#"
        FROM stock_inbound_orders orders
        JOIN stock_inbound_order_items inbound_items
          ON inbound_items.order_id = orders.id
        JOIN stock_items items ON items.id = inbound_items.item_id
        JOIN stock_template_fields fields
          ON fields.template_id = items.category_id
         AND fields.searchable = 1
        "#,
        "1 = 1",
    )
}

fn outbound_base_filter_values_sql() -> String {
    let outbound_batch_join = outbound_batch_join_sql();
    format!(
        r#"
    SELECT 'base:destination' AS field_key,
           '出库去向' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           orders.destination AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           10 AS field_order
    FROM stock_outbound_orders orders
    WHERE trim(orders.destination) <> ''
    GROUP BY orders.destination

    UNION ALL

    SELECT 'base:status' AS field_key,
           '出库状态' AS field_label,
           'base' AS field_source,
           'select' AS field_value_type,
           orders.status AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           20 AS field_order
    FROM stock_outbound_orders orders
    WHERE trim(orders.status) <> ''
    GROUP BY orders.status

    UNION ALL

    SELECT 'base:item' AS field_key,
           '物品名称' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           items.name AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           30 AS field_order
    FROM stock_outbound_orders orders
    JOIN stock_outbound_order_items outbound_items ON outbound_items.order_id = orders.id
    JOIN stock_items items ON items.id = outbound_items.item_id
    WHERE trim(items.name) <> ''
    GROUP BY items.name

    UNION ALL

    SELECT 'base:sku' AS field_key,
           '物品 SKU' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           items.sku AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           40 AS field_order
    FROM stock_outbound_orders orders
    JOIN stock_outbound_order_items outbound_items ON outbound_items.order_id = orders.id
    JOIN stock_items items ON items.id = outbound_items.item_id
    WHERE trim(items.sku) <> ''
    GROUP BY items.sku

    UNION ALL

    SELECT 'base:location' AS field_key,
           '库位' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           outbound_items.location AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           50 AS field_order
    FROM stock_outbound_orders orders
    JOIN stock_outbound_order_items outbound_items ON outbound_items.order_id = orders.id
    WHERE outbound_items.location IS NOT NULL
      AND trim(outbound_items.location) <> ''
    GROUP BY outbound_items.location

    UNION ALL

    SELECT 'base:batch_no' AS field_key,
           '批次号' AS field_label,
           'base' AS field_source,
           'text' AS field_value_type,
           batches.batch_no AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           60 AS field_order
    FROM stock_outbound_orders orders
    JOIN stock_outbound_order_items outbound_items ON outbound_items.order_id = orders.id
    {outbound_batch_join}
    WHERE trim(batches.batch_no) <> ''
    GROUP BY batches.batch_no

    ORDER BY field_order ASC, field_label ASC, value_count DESC, field_value ASC
    "#
    )
}

fn outbound_template_filter_values_sql() -> String {
    template_filter_values_sql(
        "orders.id",
        &format!(
            r#"
        FROM stock_outbound_orders orders
        JOIN stock_outbound_order_items outbound_items
          ON outbound_items.order_id = orders.id
        JOIN stock_items items ON items.id = outbound_items.item_id
        JOIN stock_template_fields fields
          ON fields.template_id = items.category_id
         AND fields.searchable = 1
        {}
        JOIN stock_inbound_order_items inbound_items
          ON inbound_items.id = batches.inbound_order_item_id
        "#,
            outbound_batch_join_sql()
        ),
        "1 = 1",
    )
}

fn outbound_batch_join_sql() -> &'static str {
    r#"
    JOIN stock_batches batches
      ON (
          batches.id = outbound_items.batch_id
          OR EXISTS (
              SELECT 1
              FROM stock_movements movements
              WHERE movements.outbound_order_item_id = outbound_items.id
                AND movements.batch_id = batches.id
                AND movements.movement_type = 'outbound'
          )
      )
    "#
}

fn template_filter_values_sql(
    entity_id_expr: &str,
    from_clause: &str,
    where_clause: &str,
) -> String {
    let json_each = json_each_object("inbound_items.ext_attributes_json");
    let json_predicate = json_scalar_predicate("json_values");
    let json_value = json_scalar_value("json_values");
    format!(
        r#"
        WITH template_values AS (
            SELECT 'template:' || fields.field_name AS field_key,
                   fields.field_name AS field_label,
                   'template' AS field_source,
                   fields.field_type AS raw_field_type,
                   {json_value} AS field_value,
                   {entity_id_expr} AS entity_id
            {from_clause}
            JOIN {json_each} AS json_values
              ON json_values.key = fields.field_name
            WHERE {where_clause}
              AND {json_predicate}
        ),
        template_field_types AS (
            SELECT field_key,
                   field_label,
                   field_source,
                   CASE
                       WHEN COUNT(DISTINCT raw_field_type) = 1 THEN MIN(raw_field_type)
                       ELSE 'mixed'
                   END AS field_value_type
            FROM template_values
            GROUP BY field_key, field_label, field_source
        )
        SELECT values_rows.field_key AS field_key,
               values_rows.field_label AS field_label,
               values_rows.field_source AS field_source,
               template_field_types.field_value_type AS field_value_type,
               values_rows.field_value AS field_value,
               COUNT(DISTINCT values_rows.entity_id) AS value_count,
               1000 AS field_order
        FROM template_values values_rows
        JOIN template_field_types
          ON template_field_types.field_key = values_rows.field_key
        GROUP BY values_rows.field_key,
                 values_rows.field_label,
                 values_rows.field_source,
                 template_field_types.field_value_type,
                 values_rows.field_value
        ORDER BY field_order ASC, values_rows.field_label ASC, value_count DESC, values_rows.field_value ASC
        "#
    )
}

fn json_each_object(column: &str) -> String {
    format!(
        "json_each(CASE WHEN {column} IS NOT NULL AND json_valid({column}) AND substr(ltrim({column}), 1, 1) = '{{' THEN {column} ELSE '{{}}' END)"
    )
}

fn json_scalar_predicate(alias: &str) -> String {
    format!(
        "{alias}.type IN ('text', 'integer', 'real', 'true', 'false') AND ({alias}.type <> 'text' OR trim(CAST({alias}.value AS TEXT)) <> '')"
    )
}

fn json_scalar_value(alias: &str) -> String {
    format!(
        "CASE {alias}.type WHEN 'true' THEN 'true' WHEN 'false' THEN 'false' ELSE CAST({alias}.value AS TEXT) END"
    )
}
