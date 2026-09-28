import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

/*
 * The rules of the window that no rendering shows, read from the source.
 *
 * Each guards a single declaration whose absence only turns up as a gesture
 * doing something strange on someone's laptop - all of them were found that
 * way, after hundreds of green tests said nothing. Until v0.77 they were Rust
 * tests reading frontend files by path; they live with the frontend now, and
 * find what they read without naming a file: the stylesheet is the one the
 * window's entry imports, and the rules about components walk all of them.
 * The rules a render can show - the screen area, each screen's scrolling, the
 * card's header - are checked on the render, in `app/smoke.test.tsx`.
 */

const ROOT = fileURLToPath(new URL('../../', import.meta.url))
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8').replace(/\r\n/g, '\n')

/** The stylesheet the window loads: `index.html`'s module, and the CSS it imports. */
function entryStylesheet(): string {
  const entries = [...read('index.html').matchAll(/<script type="module" src="\/([^"]+)"/g)]
  expect(entries, 'index.html loads one module').toHaveLength(1)
  const main = read(entries[0]![1]!)
  const sheets = [...main.matchAll(/^import '@\/([^']+\.css)'/gm)]
  expect(sheets, 'the entry imports one stylesheet').toHaveLength(1)
  return read(`src/${sheets[0]![1]}`)
}

/** Every rule of a stylesheet, as its selectors and its declarations. */
function rules(css: string): { selectors: string[]; body: string }[] {
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, '')
  return [...plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    // What stands before a rule since the last brace may end a statement
    // (`@import …;`) before the selectors start.
    selectors: match[1]!
      .split(';')
      .at(-1)!
      .split(',')
      .map((selector) => selector.trim()),
    body: match[2]!,
  }))
}

interface Component {
  path: string
  text: string
}

/**
 * Every component of the app's own: `.tsx` under `src`, tests left out, and
 * dowel's registry copies too - they are never edited here and know nothing of
 * this app's classes; the product component that uses one is still read.
 */
function components(): Component[] {
  // Where the copies live is what `components.json` tells `shadcn add`.
  const { aliases } = JSON.parse(read('components.json')) as { aliases: { ui: string } }
  const registry = `${aliases.ui.replace(/^@\//, 'src/')}/`
  const found: Component[] = []
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name)
      if (statSync(path).isDirectory()) walk(path)
      else if (name.endsWith('.tsx') && !name.endsWith('.test.tsx')) {
        const relativePath = relative(ROOT, path).replace(/\\/g, '/')
        if (relativePath.startsWith(registry)) continue
        found.push({ path: relativePath, text: readFileSync(path, 'utf8') })
      }
    }
  }
  walk(join(ROOT, 'src'))
  // The walk's watchdog: a walk that found a handful went somewhere else.
  expect(found.length, 'components found').toBeGreaterThan(100)
  return found
}

describe('the document', () => {
  it('is sealed: html, body and the root never scroll', () => {
    // `height: 100%` without it left the page scrollable both ways, and a
    // two-finger swipe slid the shell sideways until the sidebar left the
    // window, and down until a blank strip sat above the title bar.
    const root = rules(entryStylesheet()).filter(({ selectors }) =>
      ['html', 'body', '#root'].every((name) => selectors.includes(name)),
    )
    expect(root, 'one rule for html, body and #root').toHaveLength(1)
    expect(root[0]!.body).toMatch(/overflow:\s*hidden/)
  })

  it('switches selection off, and hands it back to text', () => {
    // Without the first the app reads as a web page caught mid-copy; without
    // the second nobody can copy a lyric out of the application built for
    // writing them.
    const sheet = rules(entryStylesheet())
    expect(sheet.some(({ body }) => /user-select:\s*none/.test(body))).toBe(true)
    const given = sheet.filter(({ selectors }) => selectors.some((s) => s.includes('.selectable')))
    expect(given.some(({ body }) => /user-select:\s*text/.test(body))).toBe(true)
  })
})

describe('every component', () => {
  it('makes text it shows verbatim selectable', () => {
    // `whitespace-pre-wrap` and `<pre>` are how this codebase shows text
    // exactly as it was typed - a version body, a note, an error to paste
    // into a report - so a file that renders either and never says
    // `selectable` is a place a reader will try to copy from and cannot.
    // A file, not a line, is the unit: selection is inherited, and the right
    // place to grant it is often the box around the text.
    const offenders = components()
      .filter(({ text }) => /whitespace-pre-wrap|<pre\b/.test(text))
      .filter(({ text }) => !text.includes('selectable'))
      .map(({ path }) => path)
    expect(offenders).toEqual([])
  })

  it('says numbers, dates and durations through `lib/format` only', () => {
    // A Russian window read "7.5" where it writes "7,5", and a table showed
    // "2026-09-15" beside a feed that said "Sep 15": each screen formatted
    // for itself, and most in no language at all. `lib/format` asks `Intl`
    // in the interface's language; a component that formats on its own is
    // where that stops being true.
    //
    // `toFixed` used to be allowed in the score's sparkline, where the number
    // is the geometry of an SVG path rather than prose; the sparkline is the
    // registry's since v0.78, and nothing of kilna's own draws one.
    const rules: [RegExp, string][] = [
      [/\.toLocale(Date|Time)?String\(/, 'toLocale…String'],
      [/new Intl\./, 'new Intl'],
      [/\.slice\(0,\s*10\)/, 'a timestamp cut to its date'],
      [/\.toFixed\(/, 'toFixed'],
    ]
    const offenders = components().flatMap(({ path, text }) =>
      text.split('\n').flatMap((line, index) => {
        const found = rules.filter(([rule]) => rule.test(line)).map(([, name]) => name)
        return found.map((name) => `${path}:${index + 1}: ${name}`)
      }),
    )
    expect(offenders).toEqual([])
  })

  it('never adds `relative` to an overlay that is already positioned', () => {
    // `cn` merges classes with tailwind-merge, which keeps the last class of
    // a group - and `position` is one group. A wrapper adding `relative` to a
    // dialog's popup removed the `fixed` it is built on, and every dialog
    // moved below the fold on a tall screen.
    const offenders = components().flatMap(({ path, text }) =>
      text
        .split('\n')
        .map((line, index) => ({ line, index }))
        .filter(({ line }) => /Popup className=|<Dialog[^>]*className/.test(line))
        .filter(({ line }) => /['"`\s]relative['"`\s]/.test(line))
        .map(({ index }) => `${path}:${index + 1}`),
    )
    expect(offenders).toEqual([])
  })
})
