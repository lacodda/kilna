import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
// The line's rule: a component names a colour from the dowel vocabulary and
// never writes one down, so the theme can swap it and the accent can move.
import dowel from 'dowel-ui/eslint'

export default tseslint.config(
  // `docs` is its own Astro project with its own toolchain, and most of what
  // lives there is generated.
  { ignores: ['dist', 'src-tauri', 'docs'] },
  js.configs.recommended,
  tseslint.configs.recommended,
  // The `flat` variant; the top-level one is still in the legacy shape.
  reactHooks.configs.flat['recommended-latest'],
  ...dowel.configs.recommended,
  // Where a colour is the subject rather than the styling: the cover gradients
  // are a palette this app owns, and the mark in the rail is drawn in the
  // brand's own colours. Neither follows the theme, and neither should.
  {
    files: ['src/lib/cover.ts', 'src/components/shell/Sidebar.tsx'],
    rules: { 'dowel/no-raw-color': 'off' },
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
