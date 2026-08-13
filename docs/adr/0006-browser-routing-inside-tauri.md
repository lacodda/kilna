# 0006 — Browser routing inside the Tauri webview

Date: 2026-08-13
Status: Accepted

## Context

Until v0.10 the frame was four tab buttons over a `useState`, and the open
work lived in another `useState` beside it. Nothing was addressable: no way
to link to a screen or a work, no history, and every new screen grew the
switch by hand. The approved mockup frames the app as a sidebar and a topbar,
which needs real routes underneath.

Tauri serves the frontend from an origin (`http://localhost:1420` in
development, `http://tauri.localhost` in production on WebView2), so the
webview has a working History API — the usual SPA routing machinery applies
unchanged.

Candidates: React Router in library mode, TanStack Router, or a hand-rolled
history listener.

## Decision

React Router (v8) in library mode: `BrowserRouter`, `Routes`, `NavLink` —
no framework mode, no loaders, no code splitting. Data fetching stays in the
components; a desktop app loading from disk gains nothing from route-level
loaders, and v0.12 hands that job to a query cache anyway.

Screens own URLs: `/works/:workId?` carries the open work, so a work is a
link and the back button walks real history. TanStack Router's typed routes
were not worth a second router ecosystem for six routes; a hand-rolled
listener is the same work as adopting the library, minus the edge cases
already solved.

## Consequences

**Positive.** Every screen and every work is addressable; selection state
stopped being component state. New screens are a `Route` line plus a sidebar
entry. Deep links compose with anything that can hold a URL — including the
journal and search when they arrive.

**Negative.** One more dependency in the critical path of every render.
Route paths are strings, unchecked by the compiler — a typo in a `navigate()`
call is a runtime 404 to the wildcard redirect, found by eye, not by `tsc`.
