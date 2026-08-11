//! Getting data out and in: markdown export, workspace backups, and importing
//! a slice of the author's previous system.
//!
//! The export exists to make the "you are not locked in" promise checkable
//! rather than stated. See ADR 0002.

pub mod backup;
pub mod export;
pub mod import;
