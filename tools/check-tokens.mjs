// Holds every colour class to a token the theme actually defines.
//
// `hover:text-fg` was in seven places and drew nothing at all: the token is
// `--color-text`, so the class compiled to no rule and the hover was silently
// dead everywhere it was written. Tailwind does not complain about a class it
// has no value for, and neither does the type checker — nothing in the build
// says a word, and the only symptom is a hover that does not happen.
//
// The theme is the list, read here rather than copied, so a token added in
// dowel needs no change in this file. It lives with the frontend's checks
// because it needs the frontend's dependencies installed; the Rust side of CI
// has no `node_modules` to read.
//
//   node tools/check-tokens.mjs
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('../', import.meta.url))
const THEME = join(ROOT, 'node_modules/dowel-ui/dist/theme.css')
const SOURCE = join(ROOT, 'src')

/** The prefixes that carry a colour. */
const PREFIXES = ['text-', 'bg-', 'border-', 'fill-', 'stroke-']

/**
 * Tailwind's own keywords under those prefixes: sides for `border-`,
 * alignment and wrapping for `text-`, the plain colour words.
 */
const KEYWORDS = new Set([
  'white', 'black', 'transparent', 'current', 'inherit', 'none',
  't', 'r', 'b', 'l', 'x', 'y', 's', 'e',
  'solid', 'dashed', 'dotted', 'double', 'hidden',
  'left', 'right', 'center', 'justify', 'start', 'end', 'nowrap', 'balance', 'pretty', 'wrap',
  'ellipsis', 'clip',
])

/** Every `--<family>-<name>` the theme defines, as bare names. */
function tokensOf(css, families) {
  const found = new Set()
  for (const family of families) {
    for (const match of css.matchAll(new RegExp(`--${family}-([a-z0-9-]+)`, 'g'))) {
      // `--text-sm--line-height` defines the same `sm`.
      const [base] = match[1].split('--')
      if (base !== '') found.add(base)
    }
  }
  return found
}

/** Every `.ts`/`.tsx` under a directory. */
function* sources(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry)
    if (statSync(path).isDirectory()) yield* sources(path)
    else if (/\.tsx?$/.test(entry)) yield path
  }
}

/** Lines of a file with comments dropped: prose talks about CSS too. */
function* codeLines(text) {
  let inBlock = false
  let number = 0
  for (const line of text.split('\n')) {
    number += 1
    const trimmed = line.trimStart()
    if (inBlock) {
      if (line.includes('*/')) inBlock = false
      continue
    }
    if (trimmed.startsWith('//') || trimmed.startsWith('*')) continue
    if (trimmed.startsWith('/*') && !line.includes('*/')) {
      inBlock = true
      continue
    }
    yield [number, line]
  }
}

const css = readFileSync(THEME, 'utf8')
// Colours, and the other scales that share the same prefixes: `text-sm` is a
// size, `font-medium` a weight. Collected rather than listed, so a new step
// does not fail this check.
const known = tokensOf(css, ['color', 'text', 'font-weight', 'leading', 'tracking'])

const stray = []
for (const path of sources(SOURCE)) {
  for (const [number, line] of codeLines(readFileSync(path, 'utf8'))) {
    for (const word of line.split(/["'`\s{}()]+/)) {
      // Only the colour part: `hover:text-fg` is checked as `text-fg`.
      const cls = word.slice(word.lastIndexOf(':') + 1)
      const prefix = PREFIXES.find((p) => cls.startsWith(p))
      if (prefix === undefined) continue
      const token = cls.slice(prefix.length)
      if (token === '' || token.includes('/') || token.includes('[')) continue
      // A number: an opacity, or a width on one side like `border-r-0`.
      if (/^\d/.test(token) || /-\d+$/.test(token)) continue
      if (known.has(token) || KEYWORDS.has(token)) continue
      stray.push(`${path.slice(ROOT.length)}:${number}: ${cls}`)
    }
  }
}

if (stray.length > 0) {
  console.error(
    'These classes name a colour token the theme does not define, so they draw nothing at all:',
  )
  for (const line of stray) console.error(`  ${line}`)
  process.exit(1)
}

console.log(`✓ ${known.size} tokens, every colour class accounted for`)
