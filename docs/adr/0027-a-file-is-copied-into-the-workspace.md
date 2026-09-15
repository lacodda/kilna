# 0027 — A file is copied into the workspace, and the backup takes it along

Date: 2026-09-16
Status: Accepted

## Context

The `asset` table has stood since the first migration with no writer (ADR
0008): media lives on disk, the row holds the path (ADR 0002). Giving it a
writer forces three questions the schema alone never answered.

Where the bytes live. What a backup is. And whether the window can show a
picture at all — it could not: the CSP had no `img-src`, so `default-src
'self'` blocked `asset:`, `blob:` and `data:` alike, and there was no asset
protocol configured.

## Decision

**A file is copied into the workspace, not pointed at.** The copy goes in
`media/`, beside the database, named by the asset's id and the extension it
arrived with. Pointing at a file where it lies makes a cover that works
until the day the person tidies their downloads; naming the copy by id means
two files called `cover.png` cannot collide, and nothing outside can rename
one into a collision. The name the file *arrived* under is kept in the row
and shown — it is what a person recognises, and what a generator wrote in
it.

**The backup is the database and the files.** `kilna-…db` gains a
`kilna-….media/` beside it, and a restore brings both back, setting the
replaced directory aside rather than merging into it (decision of
2026-09-15). The alternative was a restored workspace whose every cover
pointed at nothing — which looks exactly like a workspace whose pictures
were lost. Rejected: keeping bytes in the database, which ADR 0002 already
turned down (binaries bloat every backup and every future sync); and paths
to files elsewhere on the machine, which makes a workspace unmovable.

**The window reads the workspace's own directory and nothing else.** The
asset protocol is compiled in (`protocol-asset`) and enabled in the config,
but its scope is granted *at startup* for the directory the workspace
actually opened: `--workspace` moves it, and a path written into
`tauri.conf.json` would be right for one workspace and wrong for the next.
The CSP gains `img-src 'self' asset: http://asset.localhost data:`.

Two things learnt the hard way and worth writing down: there is no
`core:asset:default` permission — the protocol is configuration and scope
only, and asking for the permission fails the build; and a `fetch()` to an
asset URL is refused by `connect-src` while an `<img>` loads fine, so a
picture's arrival must be proven with an image, not a fetch.

**Attaching and detaching are not undoable, and do not go through the
trash.** Both are declared so with their reasons. The trash promises a
deletion can be taken back; a row restored beside bytes that are gone is
that promise broken. Detaching removes the row and the copy together, and
the original the person attached was never touched.

**A cover is the newest asset of kind `cover`** on the work, rather than a
flag on one of them. Attaching a second cover is how a person changes the
first, and a "chosen" flag would be a second truth to keep in step. Scene
frames need an order and a chosen one, which is why they get a table of
their own in 0.68 rather than being squeezed into this shape.
