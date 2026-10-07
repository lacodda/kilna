import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
// The line's rule: a component names a colour from the dowel vocabulary and
// never writes one down, so the theme can swap it and the accent can move.
import dowel from 'dowel-ui/eslint'

export default tseslint.config(
  // `docs` is its own Astro project with its own toolchain, and most of what
  // lives there is generated. `src/lib/api/generated` is ts-rs's output
  // (`KILNA_BLESS=1 cargo test --test bindings`) - not written here either.
  { ignores: ['dist', 'src-tauri', 'docs', 'src/lib/api/generated'] },
  js.configs.recommended,
  tseslint.configs.recommended,
  // The `flat` variant; the top-level one is still in the legacy shape.
  reactHooks.configs.flat['recommended-latest'],
  ...dowel.configs.recommended,
  // The registry's copies are dowel's, byte for byte (`pnpm registry`): what
  // they draw is answered in dowel, not here.
  {
    files: ['src/components/ui/**'],
    rules: { 'dowel/no-arbitrary-scale': 'off' },
  },
  // Where a colour is the subject rather than the styling: the cover gradients
  // are a palette this app owns, and do not follow the theme; a background
  // style is a colour the owner picked, and its editor and rules speak in it;
  // a scheme of a cover's frame is drawn in the picture's own colours, which
  // the tests of the constructor write out.
  // (The mark, the other such exception, is dowel's ProductMark now and draws
  // itself.)
  {
    files: [
      'src/lib/cover.ts',
      'src/lib/styleBrick.ts',
      'src/lib/styleBrick.test.ts',
      'src/lib/styleDraft.test.ts',
      'src/features/styles/StylesView.test.tsx',
      'src/features/work/tabs/cover/CoverTab.test.tsx',
      'src/features/work/tabs/frame/FrameTab.test.tsx',
    ],
    rules: { 'dowel/no-raw-color': 'off' },
  },
  // Where a component is the subject rather than the render: the release-kind
  // glyph is looked up in a table fixed at module level, so the identity is
  // stable across renders and nothing remounts. The rule reads any call that
  // returns a component as a factory, which is what it should do everywhere
  // else -- this is the one table the profile is allowed to point into.
  {
    files: ['src/lib/releaseIcon.tsx'],
    rules: { 'react-hooks/static-components': 'off' },
  },
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
  },
  // Build tooling runs under Node, not in the webview.
  {
    files: ['tools/**/*.mjs', '*.config.{js,ts}'],
    languageOptions: { globals: globals.node },
  },
)
