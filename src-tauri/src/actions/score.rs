//! Gestures on scores: a work judged along the craft's axes.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::journal::Record;
use crate::score::{self, NewScore, Score};

/// Score a work: a new snapshot, never an overwrite of the last one.
pub fn create(conn: &Connection, work_id: &str, new: NewScore) -> Result<Score> {
    gesture(conn, "score.create", |act| {
        act.param("workId", work_id);
        act.json("score", &new)?;
        let minted = act.mint();
        let created = score::create_minted(act, work_id, new, minted)?;
        act.journal(
            Record::new("score.added")
                .param("title", act.title_of(work_id))
                // Rounded here rather than when the line is drawn: a total of
                // 29.999999999999993 is what floating point makes of three
                // threes, and once it is in the entry it is in there for
                // good. The score keeps its full precision in `work_score`;
                // this is the number a person reads.
                .param("total", (created.total * 10.0).round() / 10.0)
                .param("tier", created.tier.clone().unwrap_or_default())
                .about("work", work_id.to_owned()),
        );
        act.restate(work_id);
        Ok(created)
    })
}
