# Contributing to kilna

## Development

Requires Rust (1.90 or newer), Node 22+ and pnpm.

```sh
pnpm install
pnpm tauri dev            # run the app
pnpm format               # prettier, the layout of every file it covers
pnpm lint                 # prettier --check + eslint + tsc + locales + tests
pnpm test                 # frontend tests: logic in Node, components in jsdom
pnpm snippets             # editor snippets for the dowel components kilna has
cd src-tauri
cargo test                # backend tests
cargo clippy -- -D warnings
cargo fmt
```

`src/components/ui/` holds copies of [dowel](https://lacodda.github.io/dowel)'s
components, byte for byte: `pnpm registry` fails on a copy that differs from
the installed `dowel-ui`. Add or refresh one with
`npx shadcn add @dowel/<name>`, and run `pnpm snippets` after: it writes
`.vscode/dowel.code-snippets` (kept out of git) from the copies there, so
typing `dw-` in a `.tsx` file offers each component with its props.

The formatting commit is listed in `.git-blame-ignore-revs`; run
`git config blame.ignoreRevsFile .git-blame-ignore-revs` once so `git blame`
looks past it.

The workspace database is created under the platform's application data
directory on first run. Schema changes are versioned migrations in
`src-tauri/migrations/`; the schema is never edited in place.

## Where things live in the backend

- `src-tauri/src/commands/` — the Tauri commands, one file per domain. A command
  is an adapter: it takes the connection and calls a read or an action.
- `src-tauri/src/actions/` — one function per gesture: the rows, the operation
  the log records and the journal line, in one unit of work
  ([ADR 0040](docs/adr/0040-a-gesture-is-one-function-and-one-unit-of-work.md)).
  `tests/operation_coverage.rs` fails when a write goes around them.
- The domain modules under them (`work`, `release`, `scene`…) read and write
  rows; a function that writes several rows wraps them in `db::unit::atomically`.
- A refusal is a code with a sentence in `src/i18n/locales/*.json` under
  `refusal`, in both languages
  ([ADR 0041](docs/adr/0041-a-refusal-is-a-code-and-the-window-says-it.md));
  `tests/refusal_keys.rs` holds the two to each other.
- The types that cross IPC are generated from Rust into
  `src/lib/api/generated/` ([ADR 0042](docs/adr/0042-the-ipc-types-are-generated-from-rust.md)).
  After changing one, run `KILNA_BLESS=1 cargo test --test bindings` in
  `src-tauri/`; the gate fails while the folder is stale.
- Test fixtures are in `src-tauri/src/fixtures.rs`. A workspace from
  `fixtures::workspace()` (in memory) or `fixtures::workspace_file()` (on disk)
  is a copy of one database migrated once per process
  ([ADR 0076](docs/adr/0076-a-test-workspace-is-a-copy-of-one-migrated-database.md));
  open an empty path with `db::open` only when opening is what the test is about.
- The app writes its own log to `logs/kilna.log` beside the workspace.

## Built with

[Tauri v2](https://v2.tauri.app/), Rust, SQLite, React.

## Documentation

Full documentation: [lacodda.github.io/kilna](https://lacodda.github.io/kilna).
Architecture decisions live in [docs/adr](https://github.com/lacodda/kilna/tree/main/docs/adr).

The site's pictures and transcripts are not kept in the repository: the Docs
workflow builds the application and takes them from it
([ADR 0089](https://github.com/lacodda/kilna/blob/main/docs/adr/0089-the-site-is-built-from-the-build-it-documents.md)).
To show a new screen, add it to `docs/screens.json` - a name, the words that
describe it, the steps that bring the window to it - and show it on a page as
`![the same words](/kilna/screens/<name>.png)`; to show what the program says,
put `<!-- transcript: mcp <tool> {arguments} -->` above a fenced block.
`pnpm pictures` holds the list and the pages to each other. To take them
yourself, from a build of your own:

```sh
cargo install tauri-driver --locked
node tools/screens.mjs --app src-tauri/target/release/kilna
node tools/transcripts.mjs --app src-tauri/target/release/kilna
```

On Windows the driver also needs the Edge WebDriver of your WebView2's
version, named with `--native-driver <path to msedgedriver.exe>`.

## Commits

English, [Conventional Commits](https://www.conventionalcommits.org/), no
trailers.

## License

By contributing, you agree that your contributions will be licensed under the
project's MIT license.
