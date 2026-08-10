# 0002 — Local-first storage: SQLite for text, disk for media

Date: 2026-08-10
Status: Accepted

## Context

kilna ships as a desktop application (Tauri v2). It must decide where a user's work lives: the text of a song or chapter, the score history, the release calendar, and the media files attached to them.

The predecessor system spent a year discovering the boundary the hard way. Its lesson, in short: **structure belongs in a database, authored bodies belong wherever they can be diffed and edited directly, and binaries never belong in either.** That system kept song lyrics as markdown files in a vault and everything structural in SQLite. The split worked, but it cost a permanent synchronisation burden — roughly a dozen import and re-sync scripts existed only because the same entity had two homes.

The same audit produced a harder rule: whenever a fact lived in two places, the two places eventually disagreed. Two separate incidents traced back to exactly that.

## Decision

**One truth per entity**, allocated as follows:

| Data | Home | Rationale |
|---|---|---|
| Work bodies, versions, scores, releases, notes, profiles | SQLite (`TEXT` columns) | Queryable, transactional, one consistent backup |
| Audio, video, images, covers | Plain files on disk; `asset` row holds the path | Binaries bloat backups and replication; Tauri resolves paths natively |
| Everything, on demand | Markdown export | The user is never locked in |

Version bodies are stored **whole, not as diffs**. Text is measured in kilobytes; reconstructing a document from a diff chain is a well-known source of corruption for no meaningful saving.

Scores are stored as **snapshots tied to a version**, not as the current state of a work. This makes the effect of a revision visible — the reason scoring exists at all.

**Schema migrations are versioned from the first commit.** The predecessor pushed schema changes directly and kept manual database copies as the safety net; that debt compounds and is not repeated in software that will run on other people's machines.

## Consequences

**Positive.** A single file is the entire backup. No file/database synchronisation layer exists, so the class of bugs it produced cannot occur. Media stays out of the database, keeping it small and its backups fast.

**Negative.** Work bodies lose the free revision history a filesystem plus git would provide; `work_version` reimplements it deliberately and must be good enough to replace it. Users cannot edit their text in an external editor against the live store — the markdown export is a copy, not a working directory. Recovering a single work from a backup means opening a database rather than copying a file.
