use std::borrow::Cow;
use std::path::Path;

use aionui_db::init_database_memory;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePoolOptions;

async fn run_migrations_through(pool: &sqlx::SqlitePool, max_version: i64) {
    let full = Migrator::new(Path::new("migrations")).await.unwrap();
    let migrations = full
        .migrations
        .iter()
        .filter(|migration| migration.version <= max_version)
        .cloned()
        .collect::<Vec<_>>();
    let migrator = Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    };
    migrator.run(pool).await.unwrap();
}

async fn run_migration(pool: &sqlx::SqlitePool, version: i64) {
    let full = Migrator::new(Path::new("migrations")).await.unwrap();
    let migrations = full
        .migrations
        .iter()
        .filter(|migration| migration.version == version)
        .cloned()
        .collect::<Vec<_>>();
    let migrator = Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: true,
        locking: true,
        no_tx: false,
    };
    migrator.run(pool).await.unwrap();
}

#[tokio::test]
async fn migration_creates_assistant_unification_tables_and_keeps_legacy_tables() {
    let db = init_database_memory().await.unwrap();

    let table_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN (
            'assistant_definitions',
            'assistant_overlays',
            'assistant_preferences',
            'conversation_assistant_snapshots',
            'assistants',
            'assistant_overrides'
        ) ORDER BY name",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();

    assert_eq!(
        table_names,
        vec![
            "assistant_definitions".to_string(),
            "assistant_overlays".to_string(),
            "assistant_overrides".to_string(),
            "assistant_preferences".to_string(),
            "assistants".to_string(),
            "conversation_assistant_snapshots".to_string(),
        ]
    );
}

#[tokio::test]
async fn assistant_definition_table_has_expected_default_columns() {
    let db = init_database_memory().await.unwrap();

    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('assistant_definitions')")
        .fetch_all(db.pool())
        .await
        .unwrap_or_default();

    assert!(
        !columns.is_empty(),
        "assistant_definitions should exist before inspecting columns"
    );

    assert!(columns.iter().any(|name| name == "id"));
    assert!(columns.iter().any(|name| name == "assistant_id"));
    assert!(!columns.iter().any(|name| name == "assistant_key"));
    assert!(columns.iter().any(|name| name == "default_model_mode"));
    assert!(columns.iter().any(|name| name == "default_permission_mode"));
    assert!(columns.iter().any(|name| name == "default_skill_ids"));
    assert!(columns.iter().any(|name| name == "default_mcp_ids"));
    assert!(columns.iter().any(|name| name == "avatar_type"));
    assert!(columns.iter().any(|name| name == "avatar_value"));

    let overlay_columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('assistant_overlays')")
        .fetch_all(db.pool())
        .await
        .unwrap_or_default();
    assert!(overlay_columns.iter().any(|name| name == "assistant_definition_id"));

    let preference_columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('assistant_preferences')")
            .fetch_all(db.pool())
            .await
            .unwrap_or_default();
    assert!(preference_columns.iter().any(|name| name == "assistant_definition_id"));

    let snapshot_columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('conversation_assistant_snapshots')")
            .fetch_all(db.pool())
            .await
            .unwrap_or_default();
    assert!(snapshot_columns.iter().any(|name| name == "conversation_id"));
    assert!(snapshot_columns.iter().any(|name| name == "assistant_definition_id"));
    assert!(snapshot_columns.iter().any(|name| name == "assistant_id"));
    assert!(!snapshot_columns.iter().any(|name| name == "assistant_name"));
    assert!(!snapshot_columns.iter().any(|name| name == "assistant_avatar_type"));
    assert!(!snapshot_columns.iter().any(|name| name == "assistant_avatar_value"));
    assert!(snapshot_columns.iter().any(|name| name == "default_model_mode"));
    assert!(snapshot_columns.iter().any(|name| name == "resolved_model_id"));
    assert!(snapshot_columns.iter().any(|name| name == "resolved_skill_ids"));
    assert!(snapshot_columns.iter().any(|name| name == "resolved_mcp_ids"));
}

#[tokio::test]
async fn assistant_agent_identity_columns_are_named_for_agent_metadata_id() {
    let db = init_database_memory().await.unwrap();

    let definition_columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('assistant_definitions')")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert!(definition_columns.iter().any(|name| name == "agent_id"));
    assert!(!definition_columns.iter().any(|name| name == "agent_backend"));

    let overlay_columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('assistant_overlays')")
        .fetch_all(db.pool())
        .await
        .unwrap();
    assert!(overlay_columns.iter().any(|name| name == "agent_id_override"));
    assert!(!overlay_columns.iter().any(|name| name == "agent_backend_override"));

    let snapshot_columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('conversation_assistant_snapshots')")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert!(snapshot_columns.iter().any(|name| name == "agent_id"));
    assert!(!snapshot_columns.iter().any(|name| name == "agent_backend"));
}

#[tokio::test]
async fn migration_017_runs_when_snapshot_identity_columns_are_already_absent() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    run_migrations_through(&pool, 16).await;

    for column_name in ["assistant_name", "assistant_avatar_type", "assistant_avatar_value"] {
        sqlx::query(&format!(
            "ALTER TABLE conversation_assistant_snapshots DROP COLUMN {column_name}"
        ))
        .execute(&pool)
        .await
        .unwrap();
    }

    run_migration(&pool, 17).await;

    let removed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('conversation_assistant_snapshots')
         WHERE name IN ('assistant_name', 'assistant_avatar_type', 'assistant_avatar_value')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(removed_count, 0);

    let index_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master
         WHERE type = 'index'
           AND name IN (
               'idx_conversation_assistant_snapshots_agent_id',
               'idx_conversation_assistant_snapshots_assistant_definition_id'
           )
         ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        index_names,
        vec![
            "idx_conversation_assistant_snapshots_agent_id".to_string(),
            "idx_conversation_assistant_snapshots_assistant_definition_id".to_string(),
        ]
    );
}

#[tokio::test]
async fn assistant_definition_table_rejects_extension_source_and_owner_type() {
    let db = init_database_memory().await.unwrap();

    let source_err = sqlx::query(
        r#"
        INSERT INTO assistant_definitions (
            id, assistant_id, source, owner_type, source_ref,
            name, name_i18n, description_i18n, avatar_type, agent_id,
            rule_resource_type, recommended_prompts, recommended_prompts_i18n,
            default_model_mode, default_permission_mode, default_skills_mode, default_skill_ids,
            custom_skill_names, default_disabled_builtin_skill_ids, default_mcps_mode, default_mcp_ids,
            created_at, updated_at
        ) VALUES (
            'd-ext-source', 'ext-source', 'extension', 'system', 'ext-source',
            'Ext Source', '{}', '{}', 'none', 'aionrs',
            'none', '[]', '{}',
            'auto', 'auto', 'fixed', '[]',
            '[]', '[]', 'auto', '[]',
            1, 1
        )
        "#,
    )
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(source_err.to_string().contains("CHECK constraint failed"));

    let owner_err = sqlx::query(
        r#"
        INSERT INTO assistant_definitions (
            id, assistant_id, source, owner_type, source_ref,
            name, name_i18n, description_i18n, avatar_type, agent_id,
            rule_resource_type, recommended_prompts, recommended_prompts_i18n,
            default_model_mode, default_permission_mode, default_skills_mode, default_skill_ids,
            custom_skill_names, default_disabled_builtin_skill_ids, default_mcps_mode, default_mcp_ids,
            created_at, updated_at
        ) VALUES (
            'd-ext-owner', 'ext-owner', 'builtin', 'extension', 'ext-owner',
            'Ext Owner', '{}', '{}', 'none', 'aionrs',
            'none', '[]', '{}',
            'auto', 'auto', 'fixed', '[]',
            '[]', '[]', 'auto', '[]',
            1, 1
        )
        "#,
    )
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(owner_err.to_string().contains("CHECK constraint failed"));
}
