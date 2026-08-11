# 0003 — React and Vite for the frontend

Date: 2026-08-10
Status: Accepted

## Context

Tauri leaves the frontend open. kilna's screens are dense and stateful — a work
list with filters, a version comparison, a score card that recomputes as it is
filled, a calendar where slots displace one another. That is ordinary
application UI, not a document, and most of the work will happen there rather
than in the Rust core.

Three candidates were weighed: React with Vite, Svelte with Vite, and a
Rust-side UI framework such as Leptos or Dioxus.

A Rust-side framework would keep the language count at one, which is a real
benefit for a small project. It was rejected because the component ecosystem
kilna leans on — a table, a combobox, a date picker, a drag-and-drop
calendar — is thin there, and building those is not this product's work.

Between React and Svelte, Svelte produces a smaller and faster bundle. For a
desktop application whose assets are loaded from disk, neither matters much:
the frontend never crosses a network.

## Decision

React 19 with Vite and TypeScript, styled with Tailwind, using
[shadcn/ui](https://ui.shadcn.com) components copied into the repository rather
than imported as a dependency.

The deciding factor is component supply and transferable experience. The
predecessor project runs on the same stack, so patterns and components carry
over directly; shadcn's React implementation is the reference one, while the
Svelte port lags it.

Two consequences accepted deliberately:

- **UI strings go through i18n from the first screen**, with English as the
  source language rather than a fallback. Retrofitting i18n after screens exist
  is the expensive order.
- **Components are copied, not depended on.** shadcn's model puts the source in
  the repository, so a component can be adjusted without forking a package.

## Consequences

**Positive.** The screens the product needs already exist as components. The
stack is familiar, so UI work is not also a learning exercise. TypeScript types
mirror the Rust command signatures, keeping the boundary explicit.

**Negative.** Two languages and two toolchains: a change to a command touches
both a Rust signature and a TypeScript type, and nothing enforces that they
agree — a generator or a shared schema may be needed once the surface grows.
The bundle is larger than a Svelte equivalent, which costs some start-up time.
Copied components must be updated by hand when upstream fixes something.

TypeScript is currently pinned to 6.x rather than the released 7.0:
typescript-eslint does not yet support the TS 7 compiler API, and linting fails
outright on it. This is a temporary pin, not a preference.
