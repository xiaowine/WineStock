//! 库存搜索和筛选值查询。
//!
//! 本模块属于 `stock` repository 的查询子模块，负责库存物品、入库历史和出库历史的自由搜索 SQL、
//! 筛选值聚合 SQL。它不拥有 HTTP DTO，也不直接处理权限或分页默认值。

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, EntityTrait, QueryFilter, Statement,
    Value,
};

use super::{
    items::{catalog_filter_sql, item_catalog_base_query},
    ItemCatalogFieldFilter, ItemFilterValuesCriteria, StockRepository,
};
use crate::persistence::entity::item_attribute_definition;

/// 筛选字段聚合记录，供 stock 服务层投影为 HTTP 响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockFilterFieldRecord {
    /// 前端使用的稳定字段 key，例如 `base:unit` 或 `template:42`。
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
    /// 查询物品目录当前筛选上下文下的分面筛选值。
    pub(crate) async fn list_item_filter_values(
        &self,
        criteria: ItemFilterValuesCriteria,
    ) -> Result<Vec<StockFilterFieldRecord>, DbErr> {
        let mut rows = Vec::new();
        let (unit_candidates, unit_values) = item_filter_candidates(&criteria, Some("base:unit"));
        rows.extend(
            query_filter_value_rows_with_values(
                self.database,
                format!(
                    r#"
                    SELECT 'base:unit' AS field_key,
                           '计量单位' AS field_label,
                           'base' AS field_source,
                           'text' AS field_value_type,
                           candidates.unit AS field_value,
                           COUNT(DISTINCT candidates.id) AS value_count,
                           20 AS field_order
                    FROM ({unit_candidates}) candidates
                    WHERE trim(candidates.unit) <> ''
                    GROUP BY candidates.unit
                    ORDER BY field_order, value_count DESC, field_value ASC
                    "#
                ),
                unit_values,
            )
            .await?,
        );

        let (location_candidates, location_values) =
            item_filter_candidates(&criteria, Some("base:location"));
        rows.extend(
            query_filter_value_rows_with_values(
                self.database,
                format!(
                    r#"
                    SELECT 'base:location' AS field_key,
                           '库位' AS field_label,
                           'base' AS field_source,
                           'text' AS field_value_type,
                           locations.name AS field_value,
                           COUNT(DISTINCT candidates.id) AS value_count,
                           30 AS field_order
                    FROM ({location_candidates}) candidates
                    JOIN stock_batches batches
                      ON batches.item_id = candidates.id AND batches.remaining_quantity > 0
                    JOIN stock_locations locations ON locations.id = batches.location_id
                    WHERE trim(locations.name) <> ''
                    GROUP BY locations.name
                    ORDER BY field_order, value_count DESC, field_value ASC
                    "#
                ),
                location_values,
            )
            .await?,
        );

        let definitions = item_attribute_definition::Entity::find()
            .filter(item_attribute_definition::Column::OwnerItemId.is_null())
            .filter(item_attribute_definition::Column::Searchable.eq(1))
            .all(self.database)
            .await?;
        for definition in definitions {
            if definition.field_type == "file" {
                continue;
            }
            let field_key = format!("template:{}", definition.id);
            let (candidates, mut values) = item_filter_candidates(&criteria, Some(&field_key));
            values.push(definition.id.into());
            rows.extend(
                query_filter_value_rows_with_values(
                    self.database,
                    format!(
                        r#"
                        SELECT ? AS field_key,
                               ? AS field_label,
                               'template' AS field_source,
                               ? AS field_value_type,
                               CASE json_type(attributes.value_json)
                                   WHEN 'true' THEN 'true'
                                   WHEN 'false' THEN 'false'
                                   ELSE CAST(json_extract(attributes.value_json, '$') AS TEXT)
                               END AS field_value,
                               COUNT(DISTINCT candidates.id) AS value_count,
                               1000 AS field_order
                        FROM ({candidates}) candidates
                        JOIN stock_item_attributes attributes ON attributes.item_id = candidates.id
                        WHERE attributes.definition_id = ?
                          AND json_valid(attributes.value_json)
                          AND json_type(attributes.value_json) IN ('text', 'integer', 'real', 'true', 'false')
                          AND (json_type(attributes.value_json) <> 'text'
                               OR trim(CAST(json_extract(attributes.value_json, '$') AS TEXT)) <> '')
                        GROUP BY field_value
                        ORDER BY field_order, value_count DESC, field_value ASC
                        "#
                    ),
                    {
                        let mut query_values = vec![
                            field_key.into(),
                            definition.field_name.into(),
                            definition.field_type.into(),
                        ];
                        query_values.extend(values);
                        query_values
                    },
                )
                .await?,
            );
        }

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
    sql.push_str(
        r#"
        AND (
            lower(stock_items.name) LIKE ?
            OR lower(stock_items.sku) LIKE ?
            OR lower(stock_items.unit) LIKE ?
            OR lower(COALESCE(stock_items.description, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_item_categories categories
                WHERE categories.id = stock_items.category_id
                  AND categories.deleted_at IS NULL
                  AND (
                      lower(categories.name) LIKE ?
                      OR lower(COALESCE(categories.description, '')) LIKE ?
                  )
            )
            OR EXISTS (
                SELECT 1
                FROM stock_item_attribute_templates templates
                WHERE templates.id = stock_items.attribute_template_id
                  AND templates.deleted_at IS NULL
                  AND (
                      lower(templates.name) LIKE ?
                      OR lower(COALESCE(templates.description, '')) LIKE ?
                  )
            )
            OR EXISTS (
                SELECT 1
                FROM stock_item_attributes attributes
                WHERE attributes.item_id = stock_items.id
                  AND lower(attributes.value_json) LIKE ?
            )
        )
        "#,
    );
    for _ in 0..9 {
        values.push(search_like.into());
    }
}

/// 给入库历史追加自由搜索条件；使用 EXISTS 避免多明细把入库单放大。
pub(super) fn append_inbound_search_filter(
    clauses: &mut Vec<String>,
    values: &mut Vec<Value>,
    search_like: &str,
) {
    clauses.push(
        r#"
        (
            lower(stock_inbound_orders.source) LIKE ?
            OR lower(stock_inbound_orders.status) LIKE ?
            OR lower(COALESCE(stock_inbound_orders.notes, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_inbound_order_items inbound_items
                JOIN stock_locations locations ON locations.id = inbound_items.location_id
                LEFT JOIN stock_items matched_items ON matched_items.id = inbound_items.item_id
                WHERE inbound_items.order_id = stock_inbound_orders.id
                  AND (
                      lower(locations.name) LIKE ?
                      OR lower(COALESCE(inbound_items.batch_no, '')) LIKE ?
                      OR lower(COALESCE(inbound_items.expires_at, '')) LIKE ?
                      OR lower(COALESCE(matched_items.name, '')) LIKE ?
                      OR lower(COALESCE(matched_items.sku, '')) LIKE ?
                      OR lower(COALESCE(matched_items.unit, '')) LIKE ?
                      OR lower(COALESCE(matched_items.description, '')) LIKE ?
                      OR EXISTS (
                          SELECT 1
                          FROM stock_inbound_order_item_attributes attributes
                          WHERE attributes.inbound_order_item_id = inbound_items.id
                            AND lower(attributes.value_json) LIKE ?
                      )
                  )
            )
        )
        "#
        .to_owned(),
    );
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
    clauses.push(
        r#"
        (
            lower(stock_outbound_orders.destination) LIKE ?
            OR lower(stock_outbound_orders.status) LIKE ?
            OR lower(COALESCE(stock_outbound_orders.notes, '')) LIKE ?
            OR EXISTS (
                SELECT 1
                FROM stock_outbound_order_items outbound_items
                LEFT JOIN stock_locations outbound_locations
                  ON outbound_locations.id = outbound_items.location_id
                LEFT JOIN stock_items matched_items ON matched_items.id = outbound_items.item_id
                WHERE outbound_items.order_id = stock_outbound_orders.id
                  AND (
                      lower(COALESCE(outbound_locations.name, '')) LIKE ?
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
                                    FROM stock_inbound_order_item_attributes attributes
                                    WHERE attributes.inbound_order_item_id = inbound_items.id
                                      AND lower(attributes.value_json) LIKE ?
                                )
                            )
                      )
                  )
            )
        )
        "#
        .to_owned(),
    );
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
    query_filter_value_rows_with_values(database, sql, Vec::new()).await
}

async fn query_filter_value_rows_with_values<C>(
    database: &C,
    sql: String,
    values: Vec<Value>,
) -> Result<Vec<RawFilterValueRow>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = database
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            values,
        ))
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

fn item_filter_candidates(
    criteria: &ItemFilterValuesCriteria,
    excluded_key: Option<&str>,
) -> (String, Vec<Value>) {
    let search_like = criteria
        .search
        .as_ref()
        .map(|search| format!("%{}%", search.to_lowercase()));
    let field_filters = criteria
        .field_filters
        .iter()
        .filter(|filter| !item_filter_matches_key(filter, excluded_key))
        .cloned()
        .collect::<Vec<_>>();
    let (base_sql, values) = item_catalog_base_query(
        search_like.as_deref(),
        criteria.category_id,
        criteria.attribute_template_id,
        &field_filters,
    );
    (
        format!(
            "SELECT * FROM ({base_sql}) catalog {}",
            catalog_filter_sql(criteria.stock_filter)
        ),
        values,
    )
}

fn item_filter_matches_key(filter: &ItemCatalogFieldFilter, key: Option<&str>) -> bool {
    match (filter, key) {
        (ItemCatalogFieldFilter::Unit(_), Some("base:unit"))
        | (ItemCatalogFieldFilter::Location(_), Some("base:location")) => true,
        (ItemCatalogFieldFilter::Template { definition_id, .. }, Some(key)) => {
            key.strip_prefix("template:")
                .and_then(|value| value.parse::<i64>().ok())
                == Some(*definition_id)
        }
        _ => false,
    }
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
           locations.name AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           50 AS field_order
    FROM stock_inbound_orders orders
    JOIN stock_inbound_order_items inbound_items ON inbound_items.order_id = orders.id
    JOIN stock_locations locations ON locations.id = inbound_items.location_id
    WHERE trim(locations.name) <> ''
    GROUP BY locations.name

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
    attribute_filter_values_sql(
        "'template:' || fields.field_name",
        "orders.id",
        r#"
        FROM stock_inbound_orders orders
        JOIN stock_inbound_order_items inbound_items
          ON inbound_items.order_id = orders.id
        JOIN stock_inbound_order_item_attributes attributes
          ON attributes.inbound_order_item_id = inbound_items.id
        JOIN stock_inbound_template_fields fields
          ON fields.id = attributes.template_field_id AND fields.searchable = 1
        "#,
        "1 = 1",
        "attributes",
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
           locations.name AS field_value,
           COUNT(DISTINCT orders.id) AS value_count,
           50 AS field_order
    FROM stock_outbound_orders orders
    JOIN stock_outbound_order_items outbound_items ON outbound_items.order_id = orders.id
    JOIN stock_locations locations ON locations.id = outbound_items.location_id
    WHERE trim(locations.name) <> ''
    GROUP BY locations.name

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
    attribute_filter_values_sql(
        "'template:' || fields.field_name",
        "orders.id",
        &format!(
            r#"
        FROM stock_outbound_orders orders
        JOIN stock_outbound_order_items outbound_items
          ON outbound_items.order_id = orders.id
        {}
        JOIN stock_inbound_order_items inbound_items
          ON inbound_items.id = batches.inbound_order_item_id
        JOIN stock_inbound_order_item_attributes attributes
          ON attributes.inbound_order_item_id = inbound_items.id
        JOIN stock_inbound_template_fields fields
          ON fields.id = attributes.template_field_id AND fields.searchable = 1
        "#,
            outbound_batch_join_sql()
        ),
        "1 = 1",
        "attributes",
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

fn attribute_filter_values_sql(
    field_key_expr: &str,
    entity_id_expr: &str,
    from_clause: &str,
    where_clause: &str,
    attribute_alias: &str,
) -> String {
    let json_type = format!("json_type({attribute_alias}.value_json)");
    let json_value = format!("CASE {json_type} WHEN 'true' THEN 'true' WHEN 'false' THEN 'false' ELSE CAST(json_extract({attribute_alias}.value_json, '$') AS TEXT) END");
    format!(
        r#"
        WITH template_values AS (
            SELECT {field_key_expr} AS field_key,
                   fields.field_name AS field_label,
                   'template' AS field_source,
                   fields.field_type AS raw_field_type,
                   {json_value} AS field_value,
                   {entity_id_expr} AS entity_id
            {from_clause}
            WHERE {where_clause}
              AND json_valid({attribute_alias}.value_json)
              AND {json_type} IN ('text', 'integer', 'real', 'true', 'false')
              AND ({json_type} <> 'text' OR trim(CAST(json_extract({attribute_alias}.value_json, '$') AS TEXT)) <> '')
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
