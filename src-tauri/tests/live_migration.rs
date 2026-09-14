//! A migration run against a real workspace, by hand.
//!
//! Ignored by default: it needs a database to point at, which CI has none of.
//! Run it before a release with `KILNA_LIVE_DB` set to a *copy* of one.

#[test]
#[ignore]
fn the_migrations_apply_to_a_real_workspace() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    let before: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let after = kilna_lib::db::migrations::apply(&mut conn).unwrap();
    println!("schema {before} -> {after}");

    let mut statement = conn.prepare("SELECT name, config FROM profile").unwrap();
    let profiles: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (name, config) in profiles {
        let config: serde_json::Value = serde_json::from_str(&config).unwrap();
        let duration = config["work_meta_fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["key"] == "duration")
            .map(|field| field["type"].clone());
        println!("profile {name}: duration is {duration:?}");
    }

    let mut statement = conn
        .prepare("SELECT title, json_extract(meta, '$.duration') FROM work WHERE json_valid(meta) AND json_extract(meta, '$.duration') IS NOT NULL")
        .unwrap();
    let lengths: Vec<(String, serde_json::Value)> = statement
        .query_map([], |row| {
            let raw: rusqlite::types::Value = row.get(1)?;
            let value = match raw {
                rusqlite::types::Value::Integer(number) => serde_json::json!(number),
                rusqlite::types::Value::Real(number) => serde_json::json!(number),
                rusqlite::types::Value::Text(text) => serde_json::json!(text),
                other => serde_json::json!(format!("{other:?}")),
            };
            Ok((row.get(0)?, value))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    // The values only -- a real workspace's titles are the owner's and stay
    // out of any output this repository can capture.
    println!("{} works carry a duration", lengths.len());
    for (_, value) in &lengths {
        println!("  -> {value}");
    }
    assert!(
        lengths.iter().all(|(_, value)| value.is_number()),
        "every stored duration is a number of seconds after the migration"
    );
}
