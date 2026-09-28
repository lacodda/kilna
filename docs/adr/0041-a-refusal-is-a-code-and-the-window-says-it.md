# 41. A refusal is a code, and the window says it

Date: 2026-09-28

## Status

Accepted. The same rule ADR 0007 set for the journal, applied to errors.

## Context

The backend refused things with English sentences: `Error::Other("the profile
has no kind of work `opera`")`, some two hundred of them. The window showed the
sentence as it came, so a Russian window read English whenever something was
refused - and the refusals are exactly the moments a person has to understand.
A batch was no better: "3 changed, 2 skipped", with no word on which two or
why. And what went wrong where nobody was looking - a journal line that could
not be written, a file the trash could not remove, a queued task that would not
start - went to `eprintln!`, which a Windows release build sends nowhere.

## Decision

**An error the person can act on is a refusal with a code**:
`Error::refused("work.unknownKind").param("kind", …)`. It travels to the
window as `{ kind: "refused", code, params, message }`; the window says
`refusal.<code>` from its own locale with the params put in. A word of the
profile is passed as its label, so it is said in the window's language.

**The English sentence comes from the same locale file.** The backend reads
`refusal.*` from `src/i18n/locales/en.json` at build time for the message it
logs, returns to a test, and sends an agent over `kilna --mcp`. One wording,
not two that drift. `tests/refusal_keys.rs` holds the codes the backend
refuses with and the sentences the locales carry to each other.

**An assumption that did not hold is `Error::Internal`**: a row that vanished
after its own insert, a log entry missing a field. The window says one generic
sentence; the detail goes to the log. `Error::Other` is gone.

**A reason is a locale key and its params** (`refusal.<code>`, `error.<kind>`,
`skip.<why>`), and can be a value of another: a refused proposal carries its
problems as a list of reasons, a batch that stopped carries the reason it
stopped. A batch returns every item it passed over with its title and its
reason, and the window names them.

**The application keeps its own log**: `logs/kilna.log` beside the workspace
(`kilna-mcp.log` for the headless server, a separate process), one line per
event, moved aside at start past a megabyte. The window sends what its error
boundary catches there too. Settings → Data shows where it is.

## Consequences

A Russian window refuses in Russian, and an agent reads the same English it
always did. The problems a profile document is refused for are still English
detail inside `profile.invalid`: they name paths in a JSON document, and
translating each is left for later. A new refusal is a code in the backend and
a sentence in both locales, and the gate fails until both exist.
