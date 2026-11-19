//! A release's numbers kept from YouTube Studio's export (v0.160).

use rusqlite::Connection;

use crate::analytics::{self, AnalyticsImport};
use crate::db::unit::atomically;
use crate::error::Result;
use crate::release::{self, ReleasePatch};

/// Keep the numbers of every row matched to a release, as its
/// `meta.analytics`, in one unit: one undo takes the whole import back.
/// How many releases were given numbers.
pub fn keep(conn: &Connection, import: &AnalyticsImport) -> Result<u32> {
    let at = crate::time::now();
    atomically(conn, |conn| {
        let mut kept = 0;
        for row in &import.rows {
            let Some(id) = row.release_id.as_deref() else {
                continue;
            };
            if row.metrics.is_empty() {
                continue;
            }
            let Some(found) = release::get(conn, id)? else {
                continue;
            };
            super::release::update(
                conn,
                id,
                ReleasePatch {
                    meta: Some(analytics::with_numbers(&found.meta, &row.metrics, &at)),
                    ..ReleasePatch::default()
                },
            )?;
            kept += 1;
        }
        Ok(kept)
    })
}
