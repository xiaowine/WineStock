//! WineStock v1 SQLite 初始数据库结构迁移。
//!
//! 本 migration 创建当前 server/API 阶段需要的用户、鉴权、刷新令牌、文件元数据和库存业务表。
//! 这里保留显式 SQL，是为了直接表达 SQLite CHECK、局部唯一索引和默认时间格式。

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

/// 创建 WineStock v1 本地 SQLite schema。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 首版 schema 使用显式 SQL，便于保留 SQLite CHECK、局部唯一索引和时间默认值。
        for statement in INITIAL_SCHEMA {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚按依赖反序删除索引和表，避免外键引用影响 schema 清理。
        for statement in DROP_SCHEMA {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }

        Ok(())
    }
}

// 首版 schema 的建表和索引语句，按外键依赖顺序执行。
const INITIAL_SCHEMA: &[&str] = &[
    // 鉴权账号、权限定义和用户权限分配统一使用 auth_ 前缀，避免和 SQLite/SeaORM 系统表混淆。
    r#"
    CREATE TABLE IF NOT EXISTS auth_users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
        password_change_required INTEGER NOT NULL DEFAULT 0 CHECK (password_change_required IN (0, 1)),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_auth_users_visible_id
        ON auth_users(id)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_permissions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        description TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_user_permission_assignments (
        user_id INTEGER NOT NULL,
        permission_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (user_id, permission_id),
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE,
        FOREIGN KEY (permission_id) REFERENCES auth_permissions(id) ON DELETE CASCADE
    )
    "#,
    // 鉴权策略和 JWT access token 签名密钥由数据库管理，不进入 JSON 启动配置。
    r#"
    CREATE TABLE IF NOT EXISTS auth_settings (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_signing_keys (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        key_id TEXT NOT NULL UNIQUE,
        algorithm TEXT NOT NULL,
        key_material TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('active', 'retired')),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        activated_at TEXT,
        retired_at TEXT,
        CHECK (status != 'active' OR activated_at IS NOT NULL),
        CHECK (status != 'retired' OR retired_at IS NOT NULL)
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_signing_keys_single_active
        ON auth_signing_keys(status)
        WHERE status = 'active'
    "#,
    // 刷新令牌保留设备来源和吊销时间，轮换逻辑必须在仓储层事务中完成。
    r#"
    CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        device_name TEXT NOT NULL,
        client_kind TEXT NOT NULL CHECK (client_kind IN ('desktop', 'android', 'web')),
        app_version TEXT NOT NULL,
        refresh_token_version TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        expires_at TEXT NOT NULL,
        last_used_at TEXT,
        revoked_at TEXT,
        replaced_by_token_id INTEGER,
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_user_id
        ON auth_refresh_tokens(user_id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_active_hash
        ON auth_refresh_tokens(token_hash)
        WHERE revoked_at IS NULL
    "#,
    // 文件内容存放在 files/ 目录，SQLite 只记录可查询和可校验的元数据。
    r#"
    CREATE TABLE IF NOT EXISTS storage_file_objects (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sha256 TEXT NOT NULL,
        mime_type TEXT,
        size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
        storage_path TEXT NOT NULL,
        original_name TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        owner_user_id INTEGER,
        FOREIGN KEY (owner_user_id) REFERENCES auth_users(id) ON DELETE SET NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_storage_file_objects_sha256
        ON storage_file_objects(sha256)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_storage_file_objects_owner_created
        ON storage_file_objects(owner_user_id, created_at DESC)
    "#,
    // 分类只负责归类，属性模板是可选录入预设，二者不能再由同一个 ID 表达。
    r#"
    CREATE TABLE IF NOT EXISTS stock_item_categories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_item_categories_name_active
        ON stock_item_categories(name)
        WHERE deleted_at IS NULL
    "#,
    // 物品属性模板仅提供推荐字段；物品仍允许不使用模板或增加自定义字段。
    r#"
    CREATE TABLE IF NOT EXISTS stock_item_attribute_templates (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT,
        is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_item_attribute_templates_name_active
        ON stock_item_attribute_templates(name)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_item_attribute_definitions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        template_id INTEGER,
        owner_item_id INTEGER,
        field_name TEXT NOT NULL,
        field_type TEXT NOT NULL CHECK (field_type IN ('text', 'number', 'select', 'date', 'file', 'url', 'boolean')),
        required INTEGER NOT NULL DEFAULT 0 CHECK (required IN (0, 1)),
        searchable INTEGER NOT NULL DEFAULT 0 CHECK (searchable IN (0, 1)),
        catalog_visible INTEGER NOT NULL DEFAULT 0 CHECK (catalog_visible IN (0, 1)),
        options_json TEXT,
        default_value TEXT,
        unit_mode TEXT NOT NULL DEFAULT 'none' CHECK (unit_mode IN ('none', 'fixed', 'select')),
        fixed_unit TEXT,
        unit_options_json TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (template_id) REFERENCES stock_item_attribute_templates(id) ON DELETE CASCADE,
        FOREIGN KEY (owner_item_id) REFERENCES stock_items(id) ON DELETE CASCADE,
        CHECK ((template_id IS NOT NULL AND owner_item_id IS NULL) OR (template_id IS NULL AND owner_item_id IS NOT NULL))
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_item_attribute_definitions_template_order
        ON stock_item_attribute_definitions(template_id, sort_order, id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_item_attribute_definitions_owner_order
        ON stock_item_attribute_definitions(owner_item_id, sort_order, id)
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS uq_stock_item_attribute_definitions_template_name
        ON stock_item_attribute_definitions(template_id, field_name COLLATE NOCASE)
        WHERE template_id IS NOT NULL
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS uq_stock_item_attribute_definitions_owner_name
        ON stock_item_attribute_definitions(owner_item_id, field_name COLLATE NOCASE)
        WHERE owner_item_id IS NOT NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        sku TEXT NOT NULL,
        category_id INTEGER,
        attribute_template_id INTEGER,
        image_file_id INTEGER NOT NULL UNIQUE,
        unit TEXT NOT NULL,
        description TEXT,
        default_price REAL CHECK (default_price IS NULL OR default_price >= 0),
        reorder_point REAL CHECK (reorder_point IS NULL OR reorder_point >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT,
        FOREIGN KEY (category_id) REFERENCES stock_item_categories(id) ON DELETE SET NULL,
        FOREIGN KEY (attribute_template_id) REFERENCES stock_item_attribute_templates(id) ON DELETE SET NULL,
        FOREIGN KEY (image_file_id) REFERENCES storage_file_objects(id) ON DELETE RESTRICT
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_items_sku_active
        ON stock_items(sku)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_items_category_active
        ON stock_items(category_id, id)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_items_attribute_template_active
        ON stock_items(attribute_template_id, id)
        WHERE deleted_at IS NULL
    "#,
    // 属性值只引用统一定义，字段配置由定义表唯一维护。
    r#"
    CREATE TABLE IF NOT EXISTS stock_item_attributes (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        item_id INTEGER NOT NULL,
        definition_id INTEGER NOT NULL,
        value_json TEXT NOT NULL,
        unit TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE CASCADE,
        FOREIGN KEY (definition_id) REFERENCES stock_item_attribute_definitions(id) ON DELETE CASCADE,
        UNIQUE (item_id, definition_id)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_item_attributes_item_order
        ON stock_item_attributes(item_id, sort_order, id)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS storage_item_file_bindings (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        file_object_id INTEGER NOT NULL UNIQUE,
        item_attribute_id INTEGER NOT NULL UNIQUE,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (file_object_id) REFERENCES storage_file_objects(id) ON DELETE RESTRICT,
        FOREIGN KEY (item_attribute_id) REFERENCES stock_item_attributes(id) ON DELETE CASCADE
    )
    "#,
    // 库位分组支持树形结构；库位本身不挂在物品上，而是挂在批次和库存流转记录上。
    r#"
    CREATE TABLE IF NOT EXISTS stock_location_groups (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        parent_id INTEGER,
        name TEXT NOT NULL,
        sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT,
        FOREIGN KEY (parent_id) REFERENCES stock_location_groups(id) ON DELETE RESTRICT,
        CHECK (parent_id IS NULL OR parent_id != id)
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_location_groups_root_name_active
        ON stock_location_groups(name)
        WHERE parent_id IS NULL AND deleted_at IS NULL
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_location_groups_child_name_active
        ON stock_location_groups(parent_id, name)
        WHERE parent_id IS NOT NULL AND deleted_at IS NULL
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_location_groups_parent_order
        ON stock_location_groups(parent_id, sort_order, id)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_locations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        group_id INTEGER NOT NULL,
        name TEXT NOT NULL,
        notes TEXT,
        sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        deleted_at TEXT,
        FOREIGN KEY (group_id) REFERENCES stock_location_groups(id) ON DELETE RESTRICT
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_locations_name_active
        ON stock_locations(name)
        WHERE deleted_at IS NULL
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_locations_group_order
        ON stock_locations(group_id, sort_order, id)
        WHERE deleted_at IS NULL
    "#,
    // 出入库单据创建时保持 pending；只有审批事务会写批次、库存流水和审计事件。
    r#"
    CREATE TABLE IF NOT EXISTS stock_inbound_orders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
        notes TEXT,
        created_by_user_id INTEGER,
        approved_by_user_id INTEGER,
        rejected_by_user_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        approved_at TEXT,
        rejected_at TEXT,
        FOREIGN KEY (created_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        FOREIGN KEY (approved_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        FOREIGN KEY (rejected_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        CHECK (status != 'approved' OR approved_at IS NOT NULL),
        CHECK (status != 'rejected' OR rejected_at IS NOT NULL)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_inbound_orders_status_created
        ON stock_inbound_orders(status, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_inbound_order_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id INTEGER NOT NULL,
        item_id INTEGER NOT NULL,
        quantity REAL NOT NULL CHECK (quantity > 0),
        unit_price REAL NOT NULL CHECK (unit_price >= 0),
        location_id INTEGER NOT NULL,
        batch_no TEXT,
        expires_at TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (order_id) REFERENCES stock_inbound_orders(id) ON DELETE CASCADE,
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE RESTRICT,
        FOREIGN KEY (location_id) REFERENCES stock_locations(id) ON DELETE RESTRICT
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_inbound_order_items_order
        ON stock_inbound_order_items(order_id, id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_inbound_order_items_item
        ON stock_inbound_order_items(item_id, id)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_batches (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        item_id INTEGER NOT NULL,
        inbound_order_item_id INTEGER,
        batch_no TEXT NOT NULL,
        location_id INTEGER NOT NULL,
        initial_quantity REAL NOT NULL CHECK (initial_quantity > 0),
        remaining_quantity REAL NOT NULL CHECK (remaining_quantity >= 0),
        unit_cost REAL NOT NULL CHECK (unit_cost >= 0),
        received_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        expires_at TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE RESTRICT,
        FOREIGN KEY (inbound_order_item_id) REFERENCES stock_inbound_order_items(id) ON DELETE SET NULL,
        FOREIGN KEY (location_id) REFERENCES stock_locations(id) ON DELETE RESTRICT,
        CHECK (remaining_quantity <= initial_quantity)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_batches_item_fifo
        ON stock_batches(item_id, location_id, expires_at, received_at, id)
        WHERE remaining_quantity > 0
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_batches_batch_no
        ON stock_batches(batch_no)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_outbound_orders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        destination TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
        notes TEXT,
        created_by_user_id INTEGER,
        approved_by_user_id INTEGER,
        rejected_by_user_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        approved_at TEXT,
        rejected_at TEXT,
        FOREIGN KEY (created_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        FOREIGN KEY (approved_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        FOREIGN KEY (rejected_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        CHECK (status != 'approved' OR approved_at IS NOT NULL),
        CHECK (status != 'rejected' OR rejected_at IS NOT NULL)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_outbound_orders_status_created
        ON stock_outbound_orders(status, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_outbound_order_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id INTEGER NOT NULL,
        item_id INTEGER NOT NULL,
        quantity REAL NOT NULL CHECK (quantity > 0),
        batch_id INTEGER,
        location_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (order_id) REFERENCES stock_outbound_orders(id) ON DELETE CASCADE,
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE RESTRICT,
        FOREIGN KEY (batch_id) REFERENCES stock_batches(id) ON DELETE RESTRICT,
        FOREIGN KEY (location_id) REFERENCES stock_locations(id) ON DELETE RESTRICT
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_outbound_order_items_order
        ON stock_outbound_order_items(order_id, id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_outbound_order_items_item
        ON stock_outbound_order_items(item_id, id)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_movements (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        item_id INTEGER NOT NULL,
        batch_id INTEGER,
        movement_type TEXT NOT NULL CHECK (movement_type IN ('inbound', 'outbound', 'adjustment')),
        quantity_delta REAL NOT NULL,
        unit_cost REAL CHECK (unit_cost IS NULL OR unit_cost >= 0),
        balance_after REAL NOT NULL CHECK (balance_after >= 0),
        location_id INTEGER,
        inbound_order_item_id INTEGER,
        outbound_order_item_id INTEGER,
        created_by_user_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE RESTRICT,
        FOREIGN KEY (batch_id) REFERENCES stock_batches(id) ON DELETE SET NULL,
        FOREIGN KEY (location_id) REFERENCES stock_locations(id) ON DELETE SET NULL,
        FOREIGN KEY (inbound_order_item_id) REFERENCES stock_inbound_order_items(id) ON DELETE SET NULL,
        FOREIGN KEY (outbound_order_item_id) REFERENCES stock_outbound_order_items(id) ON DELETE SET NULL,
        FOREIGN KEY (created_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        CHECK (quantity_delta != 0)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_movements_item_created
        ON stock_movements(item_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_movements_type_created
        ON stock_movements(movement_type, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_location_transfers (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        batch_id INTEGER NOT NULL,
        item_id INTEGER NOT NULL,
        from_location_id INTEGER NOT NULL,
        to_location_id INTEGER NOT NULL,
        quantity REAL NOT NULL CHECK (quantity > 0),
        notes TEXT,
        created_by_user_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (batch_id) REFERENCES stock_batches(id) ON DELETE RESTRICT,
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE RESTRICT,
        FOREIGN KEY (from_location_id) REFERENCES stock_locations(id) ON DELETE RESTRICT,
        FOREIGN KEY (to_location_id) REFERENCES stock_locations(id) ON DELETE RESTRICT,
        FOREIGN KEY (created_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        CHECK (from_location_id != to_location_id)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_location_transfers_batch_created
        ON stock_location_transfers(batch_id, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS stock_substitutes (
        item_id INTEGER NOT NULL,
        substitute_item_id INTEGER NOT NULL,
        priority INTEGER NOT NULL CHECK (priority > 0),
        notes TEXT,
        created_by_user_id INTEGER,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (item_id, substitute_item_id),
        FOREIGN KEY (item_id) REFERENCES stock_items(id) ON DELETE CASCADE,
        FOREIGN KEY (substitute_item_id) REFERENCES stock_items(id) ON DELETE CASCADE,
        FOREIGN KEY (created_by_user_id) REFERENCES auth_users(id) ON DELETE SET NULL,
        CHECK (item_id != substitute_item_id)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_stock_substitutes_substitute
        ON stock_substitutes(substitute_item_id)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS audit_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        user_id INTEGER,
        entity_type TEXT NOT NULL,
        entity_id INTEGER,
        action TEXT NOT NULL CHECK (action IN ('created', 'updated', 'deleted', 'approved', 'rejected', 'linked', 'unlinked', 'moved')),
        details_json TEXT,
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE SET NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_audit_events_entity_created
        ON audit_events(entity_type, entity_id, timestamp DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_audit_events_action_created
        ON audit_events(action, timestamp DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_audit_events_user_created
        ON audit_events(user_id, timestamp DESC)
    "#,
];

// 首版 schema 的回滚语句，顺序必须与建表依赖相反。
const DROP_SCHEMA: &[&str] = &[
    "DROP INDEX IF EXISTS idx_audit_events_user_created",
    "DROP INDEX IF EXISTS idx_audit_events_action_created",
    "DROP INDEX IF EXISTS idx_audit_events_entity_created",
    "DROP TABLE IF EXISTS audit_events",
    "DROP INDEX IF EXISTS idx_stock_substitutes_substitute",
    "DROP TABLE IF EXISTS stock_substitutes",
    "DROP INDEX IF EXISTS idx_stock_location_transfers_batch_created",
    "DROP TABLE IF EXISTS stock_location_transfers",
    "DROP INDEX IF EXISTS idx_stock_movements_type_created",
    "DROP INDEX IF EXISTS idx_stock_movements_item_created",
    "DROP TABLE IF EXISTS stock_movements",
    "DROP INDEX IF EXISTS idx_stock_outbound_order_items_item",
    "DROP INDEX IF EXISTS idx_stock_outbound_order_items_order",
    "DROP TABLE IF EXISTS stock_outbound_order_items",
    "DROP INDEX IF EXISTS idx_stock_outbound_orders_status_created",
    "DROP TABLE IF EXISTS stock_outbound_orders",
    "DROP INDEX IF EXISTS idx_stock_batches_batch_no",
    "DROP INDEX IF EXISTS idx_stock_batches_item_fifo",
    "DROP TABLE IF EXISTS stock_batches",
    "DROP INDEX IF EXISTS idx_stock_inbound_order_items_item",
    "DROP INDEX IF EXISTS idx_stock_inbound_order_items_order",
    "DROP TABLE IF EXISTS stock_inbound_order_items",
    "DROP INDEX IF EXISTS idx_stock_inbound_orders_status_created",
    "DROP TABLE IF EXISTS stock_inbound_orders",
    "DROP TABLE IF EXISTS storage_item_file_bindings",
    "DROP INDEX IF EXISTS idx_stock_item_attributes_item_order",
    "DROP TABLE IF EXISTS stock_item_attributes",
    "DROP INDEX IF EXISTS uq_stock_item_attribute_definitions_owner_name",
    "DROP INDEX IF EXISTS uq_stock_item_attribute_definitions_template_name",
    "DROP INDEX IF EXISTS idx_stock_item_attribute_definitions_owner_order",
    "DROP INDEX IF EXISTS idx_stock_item_attribute_definitions_template_order",
    "DROP TABLE IF EXISTS stock_item_attribute_definitions",
    "DROP INDEX IF EXISTS idx_stock_items_attribute_template_active",
    "DROP INDEX IF EXISTS idx_stock_items_category_active",
    "DROP INDEX IF EXISTS idx_stock_items_sku_active",
    "DROP TABLE IF EXISTS stock_items",
    "DROP INDEX IF EXISTS idx_stock_locations_group_order",
    "DROP INDEX IF EXISTS idx_stock_locations_name_active",
    "DROP TABLE IF EXISTS stock_locations",
    "DROP INDEX IF EXISTS idx_stock_location_groups_parent_order",
    "DROP INDEX IF EXISTS idx_stock_location_groups_child_name_active",
    "DROP INDEX IF EXISTS idx_stock_location_groups_root_name_active",
    "DROP TABLE IF EXISTS stock_location_groups",
    "DROP INDEX IF EXISTS idx_stock_item_attribute_templates_name_active",
    "DROP TABLE IF EXISTS stock_item_attribute_templates",
    "DROP INDEX IF EXISTS idx_stock_item_categories_name_active",
    "DROP TABLE IF EXISTS stock_item_categories",
    "DROP INDEX IF EXISTS idx_storage_file_objects_owner_created",
    "DROP INDEX IF EXISTS idx_storage_file_objects_sha256",
    "DROP TABLE IF EXISTS storage_file_objects",
    "DROP INDEX IF EXISTS idx_auth_refresh_tokens_active_hash",
    "DROP INDEX IF EXISTS idx_auth_refresh_tokens_user_id",
    "DROP TABLE IF EXISTS auth_refresh_tokens",
    "DROP INDEX IF EXISTS idx_auth_signing_keys_single_active",
    "DROP TABLE IF EXISTS auth_signing_keys",
    "DROP TABLE IF EXISTS auth_settings",
    "DROP TABLE IF EXISTS auth_user_permission_assignments",
    "DROP TABLE IF EXISTS auth_permissions",
    "DROP TABLE IF EXISTS auth_users",
];
