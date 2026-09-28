# 42. The IPC types are generated from Rust

Date: 2026-09-28

## Status

Accepted. Replaces the hand-written mirror ADR 0003 described for the types
that cross the wire; the typed `invoke` wrappers stay.

## Context

The window calls the backend by name and gets JSON back. Every shape on that
wire was written twice: as a Rust struct or enum, and by hand in
`src/lib/api/types.ts` - some 145 types in 1,550 lines. Nothing held them to
each other. They drifted: the trash's kinds lacked the comment a v0.76 deletion
sent, a day's verdict was spelled `displaces` in the window and `taken` in the
backend, a run carried a `task` the window never declared, and optional fields
were optional on one side only. Each drift was found by a person looking at the
wrong screen, and each fix added a gate for that one type.

## Decision

**Rust is the one source.** Every type that crosses IPC derives `ts_rs::TS`,
following its serde attributes, and the TypeScript is generated into
`src/lib/api/generated/`, one file per type. `types.ts` re-exports them under
the names the window already used and keeps only what is the window's own:
narrow unions for fields Rust keeps open on purpose, and helper types.

**The generated folder is held by a test**, `src-tauri/tests/bindings.rs`: it
exports every command's argument and return type and every event payload, with
what they depend on, into a temporary directory and compares it file by file
with the committed folder. A stale folder fails the gate;
`KILNA_BLESS=1 cargo test --test bindings` writes it again. A second test holds
the export list to the commands: a type named in a command's signature that has
no generated file fails, so a new command cannot quietly bring back a
hand-written type. Nothing is exported by an ordinary `cargo test`.

**The wire's numbers are numbers.** 64-bit integers generate as `number`, not
`bigint` (`TS_RS_LARGE_INT` in `src-tauri/.cargo/config.toml`): JSON carries
plain numbers, and every count kilna sends is far below 2^53.

**Optional means what serde does.** An output field skipped when empty is
`field?: T`; an input field with `#[serde(default)]` is omittable; a patch's
"clear" is `field?: T | null`.

The error's own shape stays hand-written in `src/lib/errors.ts`: its
serialisation is custom (ADR 0041), and the `Reason` inside it is generated.

## Consequences

A change to a Rust type that crosses the wire changes the window's type in the
same commit, or the gate says so; the TypeScript compiler then shows every
place the window assumed the old shape. The first run found sixteen fields
that were optional on one side only and a dozen call sites sending `null`
where the backend accepts only an absent field. `window_unions.rs` lost the two
tests that compared hand-written unions with enums - they are generated now -
and keeps the one that holds every trash kind to a word in both languages.
