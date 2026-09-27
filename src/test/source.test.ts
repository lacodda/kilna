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
  const found: Component[] = []
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name)
      if (statSync(path).isDirectory()) walk(path)
      else if (name.endsWith('.tsx') && !name.endsWith('.test.tsx')) {
        const relativePath = relative(ROOT, path).replace(/\\/g, '/')
        if (relativePath.startsWith('src/components/ui/')) continue
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
  it('rounds its corners by the scale, not by a number someone picked', () => {
    // Four roundings on one gesture - a row under the pointer - is what the
    // owner read as "strange corners": the rail at 10px, a nav at 9, a stage
    // stop at 5, a calendar chip at 7. The exceptions are smaller than the
    // smallest step on purpose: a badge inside a calendar tile, where `sm`
    // would swallow the tile's own corner.
    const allowed = new Set(['rounded-[4px]', 'rounded-[3px]'])
    const stray = components().flatMap(({ path, text }) =>
      text.split('\n').flatMap((line, index) =>
        [...line.matchAll(/rounded-\[[^\]]*px\]/g)]
          .map((match) => match[0])
          .filter((token) => !allowed.has(token))
          .map((token) => `${path}:${index + 1}: ${token}`),
      ),
    )
    expect(stray, 'hand-picked radii (xs 4 / sm 6 / md 9 / lg 12 / xl 16)').toEqual([])
  })

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
