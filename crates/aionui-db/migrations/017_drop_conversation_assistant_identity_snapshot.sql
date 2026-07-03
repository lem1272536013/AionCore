-- Migration 017: remove duplicated assistant identity fields from conversation snapshots.
--
-- Some development databases already had these columns removed while the
-- migration record was rolled back. Rebuild the table into the target schema
-- instead of issuing DROP COLUMN statements so the migration works whether
-- assistant_name / assistant_avatar_type / assistant_avatar_value are still
-- present or already absent.

DROP TABLE IF EXISTS _conversation_assistant_snapshots_new;

CREATE TABLE _conversation_assistant_snapshots_new (
    conversation_id                     TEXT PRIMARY KEY,
    assistant_definition_id             TEXT    NOT NULL,
    assistant_id                        TEXT    NOT NULL,
    assistant_source                    TEXT    NOT NULL,
    agent_id                            TEXT    NOT NULL,
    rules_content                       TEXT    NOT NULL DEFAULT '',
    default_model_mode                  TEXT    NOT NULL
                                                CHECK (default_model_mode IN ('auto', 'fixed')),
    resolved_model_id                   TEXT,
    default_permission_mode             TEXT    NOT NULL
                                                CHECK (default_permission_mode IN ('auto', 'fixed')),
    resolved_permission_value           TEXT,
    default_skills_mode                 TEXT    NOT NULL
                                                CHECK (default_skills_mode IN ('auto', 'fixed')),
    resolved_skill_ids                  TEXT    NOT NULL DEFAULT '[]',
    resolved_disabled_builtin_skill_ids TEXT    NOT NULL DEFAULT '[]',
    default_mcps_mode                   TEXT    NOT NULL
                                                CHECK (default_mcps_mode IN ('auto', 'fixed')),
    resolved_mcp_ids                    TEXT    NOT NULL DEFAULT '[]',
    created_at                          INTEGER NOT NULL,
    updated_at                          INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (assistant_definition_id) REFERENCES assistant_definitions(id) ON DELETE CASCADE
);

INSERT INTO _conversation_assistant_snapshots_new (
    conversation_id,
    assistant_definition_id,
    assistant_id,
    assistant_source,
    agent_id,
    rules_content,
    default_model_mode,
    resolved_model_id,
    default_permission_mode,
    resolved_permission_value,
    default_skills_mode,
    resolved_skill_ids,
    resolved_disabled_builtin_skill_ids,
    default_mcps_mode,
    resolved_mcp_ids,
    created_at,
    updated_at
)
SELECT
    conversation_id,
    assistant_definition_id,
    assistant_id,
    assistant_source,
    agent_id,
    rules_content,
    default_model_mode,
    resolved_model_id,
    default_permission_mode,
    resolved_permission_value,
    default_skills_mode,
    resolved_skill_ids,
    resolved_disabled_builtin_skill_ids,
    default_mcps_mode,
    resolved_mcp_ids,
    created_at,
    updated_at
FROM conversation_assistant_snapshots;

DROP INDEX IF EXISTS idx_conversation_assistant_snapshots_assistant_definition_id;
DROP INDEX IF EXISTS idx_conversation_assistant_snapshots_agent_id;

DROP TABLE conversation_assistant_snapshots;
ALTER TABLE _conversation_assistant_snapshots_new RENAME TO conversation_assistant_snapshots;

CREATE INDEX IF NOT EXISTS idx_conversation_assistant_snapshots_assistant_definition_id
    ON conversation_assistant_snapshots(assistant_definition_id);

CREATE INDEX IF NOT EXISTS idx_conversation_assistant_snapshots_agent_id
    ON conversation_assistant_snapshots(agent_id);
