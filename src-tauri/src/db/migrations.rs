use rusqlite::Connection;

use crate::error::{Error, Result};

/// One versioned schema step. Migrations are embedded in the binary so that a
/// workspace can always be upgraded by the build that opens it.
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
    /// Whether the step rebuilds a table: SQLite cannot change a column's
    /// constraint in place, so the table is written anew, its rows copied
    /// across and the old one dropped. Foreign keys are off while it runs -
    /// with them on, dropping the old table would delete every row that
    /// points at it - and checked whole before the step commits.
    pub rebuilds: bool,
}

/// Ordered by version; never edited once released, only appended to.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "core_schema",
        sql: include_str!("../../migrations/0001_core_schema.sql"),
        rebuilds: false,
    },
    Migration {
        version: 2,
        name: "chat",
        sql: include_str!("../../migrations/0002_chat.sql"),
        rebuilds: false,
    },
    Migration {
        version: 3,
        name: "trash",
        sql: include_str!("../../migrations/0003_trash.sql"),
        rebuilds: false,
    },
    Migration {
        version: 4,
        name: "journal",
        sql: include_str!("../../migrations/0004_journal.sql"),
        rebuilds: false,
    },
    Migration {
        version: 5,
        name: "status_pin",
        sql: include_str!("../../migrations/0005_status_pin.sql"),
        rebuilds: false,
    },
    Migration {
        version: 6,
        name: "slot_pin",
        sql: include_str!("../../migrations/0006_slot_pin.sql"),
        rebuilds: false,
    },
    Migration {
        version: 7,
        name: "chat_run",
        sql: include_str!("../../migrations/0007_chat_run.sql"),
        rebuilds: false,
    },
    Migration {
        version: 8,
        name: "chat_waiting",
        sql: include_str!("../../migrations/0008_chat_waiting.sql"),
        rebuilds: false,
    },
    Migration {
        version: 9,
        name: "focus_board",
        sql: include_str!("../../migrations/0009_focus_board.sql"),
        rebuilds: false,
    },
    Migration {
        version: 10,
        name: "work_tags_and_marks",
        sql: include_str!("../../migrations/0010_work_tags_and_marks.sql"),
        rebuilds: false,
    },
    Migration {
        version: 11,
        name: "tombstones_and_field_clocks",
        sql: include_str!("../../migrations/0011_tombstones_and_field_clocks.sql"),
        rebuilds: false,
    },
    Migration {
        version: 12,
        name: "scoring_and_release_model",
        sql: include_str!("../../migrations/0012_scoring_and_release_model.sql"),
        rebuilds: false,
    },
    Migration {
        version: 13,
        name: "operation_log",
        sql: include_str!("../../migrations/0013_operation_log.sql"),
        rebuilds: false,
    },
    Migration {
        version: 14,
        name: "version_body_clock",
        sql: include_str!("../../migrations/0014_version_body_clock.sql"),
        rebuilds: false,
    },
    Migration {
        version: 15,
        name: "work_link",
        sql: include_str!("../../migrations/0015_work_link.sql"),
        rebuilds: false,
    },
    Migration {
        version: 16,
        name: "scene",
        sql: include_str!("../../migrations/0016_scene.sql"),
        rebuilds: false,
    },
    Migration {
        version: 17,
        name: "chat_action",
        sql: include_str!("../../migrations/0017_chat_action.sql"),
        rebuilds: false,
    },
    Migration {
        version: 18,
        name: "duration_in_seconds",
        sql: include_str!("../../migrations/0018_duration_in_seconds.sql"),
        rebuilds: false,
    },
    Migration {
        version: 19,
        name: "scene_note",
        sql: include_str!("../../migrations/0019_scene_note.sql"),
        rebuilds: false,
    },
    Migration {
        version: 20,
        name: "asset_profile_and_origin",
        sql: include_str!("../../migrations/0020_asset_profile_and_origin.sql"),
        rebuilds: false,
    },
    Migration {
        version: 21,
        name: "scene_frame",
        sql: include_str!("../../migrations/0021_scene_frame.sql"),
        rebuilds: false,
    },
    Migration {
        version: 22,
        name: "scene_media_kind",
        sql: include_str!("../../migrations/0022_scene_media_kind.sql"),
        rebuilds: false,
    },
    Migration {
        version: 23,
        name: "stage_and_search_index",
        sql: include_str!("../../migrations/0023_stage_and_search_index.sql"),
        rebuilds: false,
    },
    Migration {
        version: 24,
        name: "cut",
        sql: include_str!("../../migrations/0024_cut.sql"),
        rebuilds: false,
    },
    Migration {
        version: 25,
        name: "final_stage",
        sql: include_str!("../../migrations/0025_final_stage.sql"),
        rebuilds: false,
    },
    Migration {
        version: 26,
        name: "style_brick",
        sql: include_str!("../../migrations/0026_style_brick.sql"),
        rebuilds: false,
    },
    Migration {
        version: 27,
        name: "comment",
        sql: include_str!("../../migrations/0027_comment.sql"),
        rebuilds: false,
    },
    Migration {
        version: 28,
        name: "style_brick_restore",
        sql: include_str!("../../migrations/0028_style_brick_restore.sql"),
        rebuilds: false,
    },
    Migration {
        version: 29,
        name: "canon",
        sql: include_str!("../../migrations/0029_canon.sql"),
        rebuilds: false,
    },
    Migration {
        version: 30,
        name: "register",
        sql: include_str!("../../migrations/0030_register.sql"),
        rebuilds: false,
    },
    Migration {
        version: 31,
        name: "frame",
        sql: include_str!("../../migrations/0031_frame.sql"),
        rebuilds: false,
    },
    Migration {
        version: 32,
        name: "style_set",
        sql: include_str!("../../migrations/0032_style_set.sql"),
        rebuilds: false,
    },
    Migration {
        version: 33,
        name: "scene_framing",
        sql: include_str!("../../migrations/0033_scene_framing.sql"),
        rebuilds: false,
    },
    Migration {
        version: 34,
        name: "cover_idea",
        sql: include_str!("../../migrations/0034_cover_idea.sql"),
        rebuilds: false,
    },
    Migration {
        version: 35,
        name: "word_and_release",
        sql: include_str!("../../migrations/0035_word_and_release.sql"),
        rebuilds: true,
    },
    Migration {
        version: 36,
        name: "media_root",
        sql: include_str!("../../migrations/0036_media_root.sql"),
        rebuilds: false,
    },
    Migration {
        version: 37,
        name: "brick_explanation",
        sql: include_str!("../../migrations/0037_brick_explanation.sql"),
        rebuilds: false,
    },
    Migration {
        version: 38,
        name: "trial",
        sql: include_str!("../../migrations/0038_trial.sql"),
        rebuilds: false,
    },
    Migration {
        version: 39,
        name: "time_off",
        sql: include_str!("../../migrations/0039_time_off.sql"),
        rebuilds: false,
    },
    Migration {
        version: 40,
        name: "two_shelves",
        sql: include_str!("../../migrations/0040_two_shelves.sql"),
        rebuilds: true,
    },
    Migration {
        version: 41,
        name: "search_by_key",
        sql: include_str!("../../migrations/0041_search_by_key.sql"),
        rebuilds: false,
    },
    Migration {
        version: 42,
        name: "board_card",
        sql: include_str!("../../migrations/0042_board_card.sql"),
        rebuilds: false,
    },
    Migration {
        version: 43,
        name: "library",
        sql: include_str!("../../migrations/0043_library.sql"),
        rebuilds: false,
    },
    Migration {
        version: 44,
        name: "main_version",
        sql: include_str!("../../migrations/0044_main_version.sql"),
        rebuilds: false,
    },
];

/// The newest schema this build understands.
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Schema version currently stored in the database.
pub fn current_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// Apply every migration the database has not seen yet.
///
/// Each step runs in its own transaction, so a failure leaves the database on
/// the last version that fully applied rather than half-way through one.
pub fn apply(conn: &mut Connection) -> Result<i64> {
    let mut version = current_version(conn)?;
    let latest = latest_version();

    if version > latest {
        return Err(Error::SchemaTooNew {
            found: version,
            supported: latest,
        });
    }

    let from = version;
    for migration in MIGRATIONS.iter().filter(|m| m.version > from) {
        // Outside the transaction: inside one, the pragma does nothing.
        if migration.rebuilds {
            conn.pragma_update(None, "foreign_keys", false)?;
        }
        let applied = step(conn, migration);
        if migration.rebuilds {
            conn.pragma_update(None, "foreign_keys", true)?;
        }
        applied?;
        version = migration.version;
    }

    Ok(version)
}

/// One migration in its own transaction. A step that rebuilt a table proves,
/// before it commits, that every row still points at a row that exists: the
/// keys were not checked while it ran.
fn step(conn: &mut Connection, migration: &Migration) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(migration.sql)?;
    if migration.rebuilds {
        let broken: i64 =
            tx.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if broken > 0 {
            return Err(Error::Internal(format!(
                "migration `{}` left {broken} rows pointing at nothing",
                migration.name
            )));
        }
    }
    // PRAGMA does not accept bound parameters.
    tx.pragma_update(None, "user_version", migration.version)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_sequential_and_start_at_one() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                migration.version,
                index as i64 + 1,
                "migration `{}` breaks the sequence",
                migration.name
            );
        }
    }

    #[test]
    fn apply_brings_an_empty_database_to_the_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(current_version(&conn).unwrap(), 0);

        let version = apply(&mut conn).unwrap();

        assert_eq!(version, latest_version());
        assert_eq!(current_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn apply_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();
        // A second run must not attempt to recreate existing tables.
        assert_eq!(apply(&mut conn).unwrap(), latest_version());
    }

    #[test]
    fn a_database_at_an_older_version_is_carried_forward() {
        let mut conn = Connection::open_in_memory().unwrap();
        // Apply only the first migration, as an older build would have left it.
        let first = &MIGRATIONS[0];
        conn.execute_batch(first.sql).unwrap();
        conn.pragma_update(None, "user_version", first.version)
            .unwrap();

        let version = apply(&mut conn).unwrap();

        assert_eq!(version, latest_version());
        let chat_exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'chat'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(chat_exists, 1, "the later migration must have run");
    }

    /// A workspace that held two releases on one work comes over without
    /// loss (ADR 0051): the work keeps its first, the second becomes a
    /// publication of its own - made from the same song, with its files -
    /// and a release's own title moves into its title field, or stays beside
    /// it where the field says something else. A third release is refused by
    /// the index from then on.
    #[test]
    fn a_second_release_becomes_a_publication_of_its_own() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        for migration in MIGRATIONS.iter().filter(|m| m.version <= 34) {
            conn.execute_batch(migration.sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 34).unwrap();
        crate::device::ensure(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO profile (id, key, name, config, is_active, created_at, updated_at)
                 VALUES ('p', 'music', 'Studio', '{}', 1, 't0', 't0');
             INSERT INTO work (id, profile_id, kind, title, status, meta, created_at, updated_at)
                 VALUES ('song', 'p', 'song', 'Tide', 'draft', '{}', 't0', 't0'),
                        ('clip', 'p', 'video', 'Tide (video)', 'draft', '{\"mood\":\"calm\"}', 't1', 't1');
             INSERT INTO work_link (id, profile_id, work_id, source_id, role, created_at)
                 VALUES ('l', 'p', 'clip', 'song', 'donor', 't1');
             INSERT INTO release (id, work_id, kind, status, title, meta, created_at, updated_at)
                 VALUES ('first', 'clip', 'youtube', 'released', 'Old name', '{\"title\":\"Tide\"}', 't2', 't2'),
                        ('second', 'clip', 'premiere', 'planned', 'Tide, premiere', '{}', 't3', 't3');
             INSERT INTO asset (id, work_id, release_id, kind, path, created_at, profile_id)
                 VALUES ('poster', 'clip', 'second', 'image', 'poster.png', 't3', 'p');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let owner = |release: &str| -> String {
            conn.query_row(
                "SELECT work_id FROM release WHERE id = ?1",
                [release],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(owner("first"), "clip", "the work keeps its first");
        let made = owner("second");
        assert_ne!(made, "clip");
        let (title, kind, meta): (String, String, String) = conn
            .query_row(
                "SELECT title, kind, meta FROM work WHERE id = ?1",
                [&made],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (title.as_str(), kind.as_str(), meta.as_str()),
            ("Tide (video) (premiere)", "video", "{\"mood\":\"calm\"}")
        );
        let source: String = conn
            .query_row(
                "SELECT source_id FROM work_link WHERE work_id = ?1",
                [&made],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            source, "song",
            "made from what the work it left was made from"
        );
        let poster: String = conn
            .query_row("SELECT work_id FROM asset WHERE id = 'poster'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(poster, made, "its files go with it");

        let meta = |release: &str| -> String {
            conn.query_row("SELECT meta FROM release WHERE id = ?1", [release], |row| {
                row.get(0)
            })
            .unwrap()
        };
        assert_eq!(
            meta("first"),
            "{\"title\":\"Tide\",\"former_title\":\"Old name\"}",
            "a word the field does not say is kept beside it"
        );
        assert_eq!(meta("second"), "{\"title\":\"Tide, premiere\"}");

        let third = conn.execute(
            "INSERT INTO release (id, work_id, kind, status, meta, created_at, updated_at)
                 VALUES ('third', 'clip', 'premiere', 'planned', '{}', 't4', 't4')",
            [],
        );
        assert!(third.is_err(), "one release per publication");
    }

    /// The two shelves (ADR 0064): a word of the bank becomes a phrase - the
    /// phrase already written in the same words when there is one - its
    /// blocks become shelves, a word only kept stops being a term while a
    /// word sung its own way stays one without its bank, an idea becomes a
    /// note tagged `idea`, and what the trash holds is rewritten to match.
    #[test]
    fn the_bank_becomes_phrases_on_shelves_and_an_idea_a_tagged_note() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        for migration in MIGRATIONS.iter().filter(|m| m.version <= 39) {
            conn.execute_batch(migration.sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 39).unwrap();
        crate::device::ensure(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO profile (id, key, name, config, is_active, created_at, updated_at)
                 VALUES ('p', 'music', 'Studio', '{}', 1, 't0', 't0');
             INSERT INTO term (id, profile_id, word, forms, kind, strictness, bank, sung, created_at, updated_at)
                 VALUES ('kept', 'p', 'nebula', '[]', 'noun', NULL, 'fresh', '[]', 't1', 't1'),
                        ('parked', 'p', 'pulsar', '[]', 'noun', NULL, 'parked', '[]', 't2', 't2'),
                        ('sung', 'p', 'Marseille', '[]', 'noun', NULL, 'fresh',
                         '[{\"written\":\"Marseille\",\"sung\":\"MarsEille\"}]', 't3', 't3'),
                        ('out', 'p', 'comet', '[]', 'noun', NULL, NULL, '[]', 't4', 't4'),
                        ('spent', 'p', 'window', '[]', 'noun', 'limit', NULL, '[]', 't5', 't5');
             INSERT INTO term_block (id, profile_id, name, position, created_at, updated_at)
                 VALUES ('space', 'p', 'Space', 0, 't1', 't1');
             INSERT INTO term_block_word (id, profile_id, block_id, term_id, created_at)
                 VALUES ('w1', 'p', 'space', 'kept', 't1'),
                        ('w2', 'p', 'space', 'out', 't4');
             INSERT INTO note (id, profile_id, kind, title, body, tags, created_at, updated_at, state)
                 VALUES ('already', 'p', 'phrase', NULL, 'pulsar', '[\"space\"]', 't0', 't0', 'fresh'),
                        ('seed', 'p', 'idea', 'Chickens in Istanbul', 'a yard like ours', '[\"seed\"]', 't6', 't6', 'used'),
                        ('tagged', 'p', 'idea', 'Twice', 'x', '[\"idea\"]', 't7', 't7', 'fresh');
             INSERT INTO deletion (id, profile_id, entity, entity_id, label, snapshot, deleted_at)
                 VALUES ('d-term', 'p', 'term', 'gone', 'gone',
                         '{\"term\":[{\"id\":\"gone\",\"word\":\"gone\",\"bank\":\"fresh\"}],\"term_block_word\":[{\"id\":\"w9\"}]}', 't8'),
                        ('d-block', 'p', 'block', 'kitchen', 'Kitchen',
                         '{\"term_block\":[{\"id\":\"kitchen\",\"name\":\"Kitchen\"}],\"term_block_word\":[]}', 't8'),
                        ('d-note', 'p', 'note', 'old', 'Old',
                         '{\"note\":[{\"id\":\"old\",\"kind\":\"idea\",\"tags\":\"[\\\"x\\\"]\",\"state\":\"used\"}]}', 't8');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let query = |sql: &str| -> Vec<String> {
            let mut statement = conn.prepare(sql).unwrap();
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert_eq!(
            query("SELECT body || ':' || state FROM note WHERE kind = 'phrase' ORDER BY body"),
            [
                "Marseille:fresh",
                "comet:dropped",
                "nebula:fresh",
                "pulsar:fresh"
            ],
            "every word kept, or still in a block, is a phrase; `pulsar` is the one already written"
        );
        assert_eq!(
            query("SELECT count(*) || '' FROM note WHERE kind = 'phrase' AND body = 'pulsar'"),
            ["1"],
            "no second copy beside the phrase in the same words"
        );
        assert_eq!(
            query(
                "SELECT s.name || ':' || n.body FROM shelf s JOIN shelf_note p ON p.shelf_id = s.id
                   JOIN note n ON n.id = p.note_id ORDER BY n.body"
            ),
            ["Space:comet", "Space:nebula"],
            "the block is a shelf of the same phrases, under its own id"
        );
        assert_eq!(query("SELECT id FROM shelf"), ["space"]);
        assert_eq!(
            query("SELECT word FROM term ORDER BY word"),
            ["Marseille", "window"],
            "a word only kept said everything as a phrase; sung or spent, it stays a term"
        );
        let columns = query("SELECT name FROM pragma_table_info('term')");
        assert!(!columns.contains(&"bank".to_owned()), "{columns:?}");
        let tables = query("SELECT name FROM sqlite_master WHERE type = 'table'");
        assert!(
            !tables.iter().any(|t| t.starts_with("term_block")),
            "{tables:?}"
        );

        assert_eq!(
            query(
                "SELECT kind || ':' || state || ':' || tags FROM note WHERE id IN ('seed', 'tagged') ORDER BY id"
            ),
            ["note:fresh:[\"seed\",\"idea\"]", "note:fresh:[\"idea\"]"],
            "an idea is a note tagged once, and a note is not spent"
        );

        assert_eq!(
            query("SELECT snapshot FROM deletion WHERE id = 'd-term'"),
            ["{\"term\":[{\"id\":\"gone\",\"word\":\"gone\"}]}"]
        );
        assert_eq!(
            query("SELECT entity || ' ' || snapshot FROM deletion WHERE id = 'd-block'"),
            ["shelf {\"shelf\":[{\"id\":\"kitchen\",\"name\":\"Kitchen\"}]}"]
        );
        assert_eq!(
            query("SELECT snapshot FROM deletion WHERE id = 'd-note'"),
            [
                "{\"note\":[{\"id\":\"old\",\"kind\":\"note\",\"tags\":\"[\\\"x\\\",\\\"idea\\\"]\",\"state\":\"fresh\"}]}"
            ],
            "an idea in the trash comes back as a note, its tags still text"
        );
    }

    /// The index of a workspace made before 0041 is keyed by the rows it
    /// holds: each keeps its rowid as its key, and a thing indexed twice keeps
    /// its later text only - the triggers find one row per key from then on.
    #[test]
    fn the_index_takes_its_rows_as_keys_and_keeps_the_later_of_two() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        for migration in MIGRATIONS.iter().filter(|m| m.version <= 40) {
            conn.execute_batch(migration.sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 40).unwrap();
        conn.execute_batch(
            "INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
                 VALUES (3, 'note', 'n1', 'p', NULL, 'older words'),
                        (7, 'note', 'n1', 'p', NULL, 'later words'),
                        (9, 'work', 'w1', 'p', 'w1', 'Harbour lights');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut statement = conn
            .prepare(
                "SELECT k.id, k.entity, k.entity_id, s.body FROM search_key k
                   JOIN search_index s ON s.rowid = k.id ORDER BY k.rowid",
            )
            .unwrap();
        let rows: Vec<(i64, String, String, String)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        assert_eq!(
            rows,
            [
                (7, "note".into(), "n1".into(), "later words".into()),
                (9, "work".into(), "w1".into(), "Harbour lights".into()),
            ]
        );
        let left: i64 = conn
            .query_row("SELECT count(*) FROM search_index", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 2, "the older text of the same note is gone");
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_downgraded() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", latest_version() + 1)
            .unwrap();

        let error = apply(&mut conn).unwrap_err();

        assert!(matches!(error, Error::SchemaTooNew { .. }));
    }
}
