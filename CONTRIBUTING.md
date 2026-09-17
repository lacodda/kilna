# Contributing to kilna

## Development

Requires Rust (1.85 or newer), Node 22+ and pnpm.

```sh
pnpm install
pnpm tauri dev            # run the app
pnpm lint                 # eslint + tsc + locales + tests
pnpm test                 # frontend unit tests
cd src-tauri
cargo test                # backend tests
cargo clippy -- -D warnings
```

The workspace database is created under the platform's application data
directory on first run. Schema changes are versioned migrations in
`src-tauri/migrations/`; the schema is never edited in place.

## Built with

[Tauri v2](https://v2.tauri.app/), Rust, SQLite, React.

## Documentation

Full documentation: [lacodda.github.io/kilna](https://lacodda.github.io/kilna).
Architecture decisions live in [docs/adr](https://github.com/lacodda/kilna/tree/main/docs/adr).

## Commits

English, [Conventional Commits](https://www.conventionalcommits.org/), no
trailers.

## License

By contributing, you agree that your contributions will be licensed under the
project's MIT license.
