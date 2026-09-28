//! A unit of work: writes that land together or not at all, and that nest.
//!
//! A gesture used to be one transaction, opened by whoever made the change:
//! `conn.transaction()` in the command, or inside a domain function that had
//! several rows to write. Two such changes could not be joined into one - a
//! transaction cannot open inside another - so a proposal package that wrote
//! a work, three versions and a board was nine transactions, and a failure at
//! the board left the work and its versions behind (ADR 0040).
//!
//! A unit is a SAVEPOINT instead. The first one opened on a connection begins
//! the transaction and commits it on release; one opened inside it only marks
//! a point it can roll back to. So a domain function that must write three
//! rows together wraps them in a unit and stays correct on its own, and the
//! same function called inside a larger unit becomes part of that one.
//!
//! What cannot be rolled back - a file removed from disk - waits for the
//! outermost unit to commit: see [`after_commit`].

use std::cell::RefCell;
use std::collections::HashMap;

use rusqlite::Connection;

use crate::error::Result;

/// The savepoint every unit opens. One name for all of them: SQLite matches
/// RELEASE and ROLLBACK TO against the innermost savepoint of that name, which
/// is exactly the unit being closed.
const SAVEPOINT: &str = "kilna_unit";

/// Something to do once the writes it belongs to are committed for good.
type Effect = Box<dyn FnOnce(&Connection)>;

/// What each connection's open units are waiting to do, and how deep they are.
///
/// Each effect keeps the depth it was queued at: an inner unit that rolls back
/// drops what was queued inside it and leaves what its outer units queued.
#[derive(Default)]
struct Frame {
    depth: usize,
    effects: Vec<(usize, Effect)>,
}

thread_local! {
    /// Keyed by the connection's address: a unit runs on the thread that
    /// opened it, start to end, and a connection does not move while one is
    /// open - it is borrowed for the whole of it.
    static FRAMES: RefCell<HashMap<usize, Frame>> = RefCell::new(HashMap::new());
}

fn key(conn: &Connection) -> usize {
    std::ptr::from_ref(conn) as usize
}

/// Run `work` as one unit: everything it writes is committed together, or
/// none of it is.
///
/// Inside another unit this is a savepoint of that one - an error rolls back
/// what `work` wrote and leaves the outer unit to decide - so a batch can let
/// one item fail and keep the rest. Outermost, it is the transaction itself.
pub fn atomically<T>(conn: &Connection, work: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let mut guard = Guard::open(conn)?;
    let done = work(conn)?;
    guard.release()?;
    Ok(done)
}

/// Do `effect` once the current unit's writes are committed - now, when no
/// unit is open.
///
/// For what the database cannot take back: a file removed because the row
/// naming it was purged. Removed before the commit, it is gone even when the
/// purge then rolls back as part of a larger gesture that failed. Dropped
/// unrun when the outermost unit rolls back.
///
/// A transaction opened by hand rather than through [`atomically`] - a
/// migration, a replay - is not a unit: the effect runs straight away there,
/// as it would have before units existed.
pub fn after_commit(conn: &Connection, effect: impl FnOnce(&Connection) + 'static) {
    let queued = FRAMES.with(|frames| match frames.borrow_mut().get_mut(&key(conn)) {
        Some(frame) if frame.depth > 0 => {
            frame.effects.push((frame.depth, Box::new(effect)));
            None
        }
        _ => Some(effect),
    });
    if let Some(effect) = queued {
        effect(conn);
    }
}

/// One open unit. Rolls itself back when it is dropped unreleased - by an
/// early return, an error, or a panic - so an abandoned unit cannot leave the
/// connection inside a transaction every later write would join.
struct Guard<'c> {
    conn: &'c Connection,
    outermost: bool,
    open: bool,
}

impl<'c> Guard<'c> {
    fn open(conn: &'c Connection) -> Result<Self> {
        // Outermost when nothing is open: then the savepoint begins the
        // transaction, and releasing it commits.
        let outermost = conn.is_autocommit();
        conn.execute_batch(&format!("SAVEPOINT {SAVEPOINT}"))?;
        FRAMES.with(|frames| {
            frames.borrow_mut().entry(key(conn)).or_default().depth += 1;
        });
        Ok(Self {
            conn,
            outermost,
            open: true,
        })
    }

    fn release(&mut self) -> Result<()> {
        // Released first and only then counted closed: a commit that fails
        // leaves the unit open, and the drop below rolls it back.
        self.conn.execute_batch(&format!("RELEASE {SAVEPOINT}"))?;
        self.open = false;
        let effects = self.close();
        for effect in effects {
            effect(self.conn);
        }
        Ok(())
    }

    /// Leave the frame, handing back what now has to run - the queue, when
    /// this was the outermost unit and it committed.
    fn close(&self) -> Vec<Effect> {
        FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            let Some(frame) = frames.get_mut(&key(self.conn)) else {
                return Vec::new();
            };
            let depth = frame.depth;
            frame.depth = depth.saturating_sub(1);
            if self.open {
                // Rolled back: what was queued inside this unit never happens.
                frame.effects.retain(|(queued_at, _)| *queued_at < depth);
            }
            if frame.depth > 0 {
                return Vec::new();
            }
            let effects = std::mem::take(&mut frame.effects);
            frames.remove(&key(self.conn));
            effects.into_iter().map(|(_, effect)| effect).collect()
        })
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if !self.open {
            return;
        }
        // The outermost unit takes the whole transaction with it; an inner
        // one returns to where it began and leaves the outer one open.
        let undone = if self.outermost {
            self.conn.execute_batch("ROLLBACK")
        } else {
            self.conn
                .execute_batch(&format!("ROLLBACK TO {SAVEPOINT}; RELEASE {SAVEPOINT}"))
        };
        if let Err(cause) = undone {
            crate::log::error("db", &format!("a unit could not be rolled back: {cause}"));
        }
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use std::rc::Rc;

    fn table() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (v INTEGER NOT NULL)")
            .unwrap();
        conn
    }

    fn values(conn: &Connection) -> Vec<i64> {
        let mut statement = conn.prepare("SELECT v FROM t ORDER BY v").unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    }

    fn insert(conn: &Connection, v: i64) -> Result<()> {
        conn.execute("INSERT INTO t (v) VALUES (?1)", [v])?;
        Ok(())
    }

    #[test]
    fn a_unit_commits_what_it_wrote() {
        let conn = table();
        atomically(&conn, |c| {
            insert(c, 1)?;
            insert(c, 2)
        })
        .unwrap();
        assert_eq!(values(&conn), [1, 2]);
        assert!(conn.is_autocommit(), "the transaction is closed");
    }

    #[test]
    fn a_failed_unit_leaves_nothing_behind() {
        let conn = table();
        let failed = atomically(&conn, |c| {
            insert(c, 1)?;
            Err::<(), _>(Error::Internal("stop".into()))
        });
        assert!(failed.is_err());
        assert!(values(&conn).is_empty(), "the first row went with the unit");
        assert!(conn.is_autocommit());
    }

    #[test]
    fn a_failed_inner_unit_leaves_the_outer_one_going() {
        let conn = table();
        atomically(&conn, |c| {
            insert(c, 1)?;
            let inner = atomically(c, |c| {
                insert(c, 2)?;
                Err::<(), _>(Error::Internal("one item".into()))
            });
            assert!(inner.is_err());
            insert(c, 3)
        })
        .unwrap();
        assert_eq!(values(&conn), [1, 3]);
    }

    #[test]
    fn a_failed_outer_unit_takes_the_inner_ones_with_it() {
        let conn = table();
        let failed = atomically(&conn, |c| {
            atomically(c, |c| insert(c, 1))?;
            atomically(c, |c| insert(c, 2))?;
            Err::<(), _>(Error::Internal("late".into()))
        });
        assert!(failed.is_err());
        assert!(
            values(&conn).is_empty(),
            "what the inner units wrote is gone too"
        );
    }

    #[test]
    fn a_panic_inside_a_unit_rolls_it_back() {
        let conn = table();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = atomically(&conn, |c| -> Result<()> {
                insert(c, 1)?;
                panic!("mid-unit");
            });
        }));
        assert!(caught.is_err());
        assert!(conn.is_autocommit(), "no transaction is left open");
        assert!(values(&conn).is_empty());
        // And the connection still works.
        atomically(&conn, |c| insert(c, 4)).unwrap();
        assert_eq!(values(&conn), [4]);
    }

    #[test]
    fn an_effect_waits_for_the_outermost_commit() {
        let conn = table();
        let ran = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&ran);
        atomically(&conn, |c| {
            atomically(c, |c| {
                let seen = Rc::clone(&seen);
                after_commit(c, move |_| seen.borrow_mut().push("effect"));
                Ok(())
            })?;
            assert!(
                seen.borrow().is_empty(),
                "not before the outer unit commits"
            );
            Ok(())
        })
        .unwrap();
        assert_eq!(*ran.borrow(), ["effect"]);
    }

    #[test]
    fn an_effect_of_a_rolled_back_unit_never_runs() {
        let conn = table();
        let ran = Rc::new(RefCell::new(0));
        let seen = Rc::clone(&ran);
        let _ = atomically(&conn, |c| {
            after_commit(c, move |_| *seen.borrow_mut() += 1);
            Err::<(), _>(Error::Internal("no".into()))
        });
        // A later unit on the same connection does not inherit it either.
        atomically(&conn, |c| insert(c, 1)).unwrap();
        assert_eq!(*ran.borrow(), 0);
    }

    #[test]
    fn an_effect_of_a_rolled_back_inner_unit_is_dropped_and_the_outer_ones_run() {
        let conn = table();
        let ran = Rc::new(RefCell::new(Vec::new()));
        let outer = Rc::clone(&ran);
        let inner = Rc::clone(&ran);
        atomically(&conn, |c| {
            after_commit(c, move |_| outer.borrow_mut().push("outer"));
            let _ = atomically(c, |c| {
                after_commit(c, move |_| inner.borrow_mut().push("inner"));
                Err::<(), _>(Error::Internal("the item failed".into()))
            });
            Ok(())
        })
        .unwrap();
        assert_eq!(*ran.borrow(), ["outer"]);
    }

    #[test]
    fn an_effect_outside_any_unit_runs_at_once() {
        let conn = table();
        let ran = Rc::new(RefCell::new(false));
        let seen = Rc::clone(&ran);
        after_commit(&conn, move |_| *seen.borrow_mut() = true);
        assert!(*ran.borrow());
    }
}
