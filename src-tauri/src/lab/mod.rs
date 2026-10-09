//! The lab: experiments and their boards of trials (v0.95, ADR 0061).
//!
//! An experiment is a work of a kind the profile marks as a lab. It carries
//! what any work carries - a brief and findings as versions, fields, notes,
//! links, a history, the trash - and one thing of its own: a board of
//! trials. A trial is a text tried out somewhere outside kilna - a style
//! prompt run through a music generator - heard, and kept or thrown away.
//!
//! What a kept trial becomes is the other half: a version of the role the
//! lab names, in a work of a kind that has it (a song's style), written
//! beside that work's current version; or a phrase of the dictionary; or the
//! first version of a new work. Each remembers the trial it came from, and
//! the board shows where a trial went by asking - material is spent, not
//! moved (ADR 0045).
//!
//! kilna knows nothing here about sound or a generator: the role, the
//! composition a trial is read against and the field that holds the anchors
//! are the profile's words.

pub mod answer;
pub mod harvest;
pub mod trial;

pub use trial::{NewTrial, Trial, TrialBoard, TrialCard, TrialPatch, Verdict};
