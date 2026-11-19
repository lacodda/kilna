//! Getting data out and in: markdown export, workspace backups, a work as a
//! parcel that leaves one space and lands in another, works coming in from
//! files and the clipboard, and importing a slice of the author's previous
//! system.
//!
//! The export exists to make the "you are not locked in" promise checkable
//! rather than stated. See ADR 0002.

pub mod backup;
pub mod channel;
pub mod chat;
pub mod convert;
pub mod export;
pub mod ideas;
pub mod import;
pub mod intake;
pub mod material;
pub mod package;
pub mod parcel;
pub mod rehome;
pub mod songs;
