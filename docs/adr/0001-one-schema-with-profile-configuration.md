# 0001 — One schema, configured by profile

Date: 2026-08-10
Status: Accepted

## Context

kilna targets several crafts: music, prose, podcasting, journalism. A song, a chapter, and an episode carry different fields and are judged by different criteria. Three ways to support that:

1. **Entity-attribute-value / arbitrary user fields.** The user defines whatever fields they want.
2. **A table per craft** — `songs`, `chapters`, `episodes`.
3. **One fixed schema plus a configuration object** describing the craft.

Option 1 makes querying impossible in practice: weighted scoring cannot be computed over untyped attributes, and every screen degenerates into a generic property list. Option 2 turns each new craft into a migration plus new code — the opposite of the goal.

The observation that settles it: crafts differ in **vocabulary and evaluation criteria**, not in structure. A song has a body; a chapter has a body. Both go through revisions, both get judged, both get scheduled and shipped.

## Decision

A single fixed schema, with craft-specific behaviour supplied by a `profile` row:

```jsonc
{
  "work_kinds":       ["song"],
  "release_kinds":    ["clip", "short", "audio-release"],
  "collection_kinds": ["album", "cycle"],
  "version_roles":    ["lyrics", "style"],
  "axes":             [{ "key": "hook", "label": "Hook", "weight": 2, "scale": 10 }],
  "tiers":            [{ "key": "clip", "min": 75 }],
  "statuses":         ["draft", "evaluated", "scheduled", "released"],
  "work_meta_fields": [{ "key": "bpm", "type": "number" }]
}
```

Craft-specific values live in `work.meta` (JSON), keyed by `work_meta_fields`. Profiles ship with the application and are editable; users pick and adjust rather than design a schema.

Consequences for related tables:

- **Score axis keys are strings from the profile, not foreign keys.** A score snapshot stores `{"hook": 8}`. Reconfiguring a profile therefore never invalidates historical scores.
- **`work.meta` is JSON rather than columns.** Columns per craft would produce a wide table that is mostly NULL. SQLite supports expression indexes over JSON if querying becomes necessary.

## Consequences

**Positive.** A new craft is a JSON document, not a migration. Screens, scoring, and the calendar are written once. Historical data survives profile edits.

**Negative.** Fields inside `meta` are not typed by the database; validation lives in application code against `work_meta_fields`. Cross-craft queries over `meta` require JSON functions. Profiles must be versioned carefully once users edit them, so that renaming an axis does not orphan past snapshots — mitigated by keeping keys stable and labels mutable.
