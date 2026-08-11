# 0004 — Plugins are external executables

Date: 2026-08-11
Status: Accepted

## Context

kilna deliberately leaves whole categories of work out of its core: publishing
to a platform, cutting video, fetching metrics. Each is specific to one
service, one craft, or one person's habits, and putting any of them in the
application would mean maintaining an integration nobody else uses.

Three ways to let others add them were considered.

**Dynamic libraries** loaded into kilna's process are the fastest and the most
capable. They are also unusable here: Rust has no stable ABI, so a plugin would
have to be rebuilt against every kilna release, and a crash or memory error in
somebody else's code takes the user's workspace down with it.

**A sandboxed runtime** — WebAssembly — solves the safety problem and the ABI
problem at once. It fails on purpose: the integrations plugins exist for are
precisely the ones a sandbox forbids. Reading a credential from the system
keychain, driving a local ffmpeg, talking to a corporate API over a VPN — a
sandbox exists to prevent exactly this, and every escape hatch added to permit
it removes the reason for having one.

**External executables** are what git, cargo and kubectl do, and what the rest
of this product line already does.

## Decision

A plugin is an ordinary program named `kilna-plugin-*`, found on the `PATH` or
in the workspace's `plugins` directory. kilna runs it twice:

```sh
kilna-plugin-x --manifest   # what it offers, as JSON on stdout
kilna-plugin-x run          # invocation as JSON on stdin, outcome on stdout
```

The manifest declares a `protocol_version`, and a version kilna does not speak
is refused rather than attempted.

Consequences that follow from the shape, each chosen deliberately:

- **A plugin that cannot describe itself is still listed, with the reason.** An
  integration that silently fails to appear is indistinguishable from one that
  was never installed — and that is the harder failure to report.
- **The invocation carries the subject in full**, including every version
  role's latest body for a work. Found while writing the first plugin: without
  it, a plugin's first act would be asking for the text back.
- **What a plugin returns is merged into `meta`, never substituted for it.** A
  plugin may add and overwrite its own keys and cannot clear the rest. Losing
  unrelated metadata to a third-party integration is not recoverable.
- **Plugins live for the duration of a call.** Nothing here starts a service,
  so there is no lifecycle to manage and nothing to leak.

Each plugin is its own repository with its own version and releases, as in the
rest of the line.

## Consequences

**Positive.** A plugin can be written in any language. A broken one cannot take
kilna down, corrupt its memory, or force a rebuild when kilna is upgraded. The
protocol is small enough to implement in an afternoon —
[kilna-plugin-wordcount](https://github.com/lacodda/kilna-plugin-wordcount) is
about a hundred lines.

**Negative.** A plugin runs with the user's full privileges; installing one is
as much a decision as installing any other program, and kilna cannot make it
safer. Process startup costs milliseconds per call, which rules this out for
anything wanted on every keystroke. The JSON contract has to stay compatible
once plugins exist outside this repository — `protocol_version` is the escape
hatch, and using it means every existing plugin stops working until updated.
