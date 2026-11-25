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
  it('stills everything for a reader who asked for less motion', () => {
    // An element and what it draws before and after itself: a rule on `*`
    // alone leaves a shimmer or a caret drawn in a pseudo-element moving.
    const css = read('src/styles.css')
    const block = /@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/.exec(css)?.[1] ?? ''
    expect(block).toMatch(/\*,\s*\*::before,\s*\*::after \{/)
    for (const stilled of ['transition: none', 'animation: none', 'scroll-behavior: auto']) {
      expect(block).toContain(stilled)
    }
  })

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

  it("leaves the step between a dialog's parts to the dialog", () => {
    // `Dialog` stands what it is given in one column, a `gap-3` apart
    // (FR-19 19.2). Before that each dialog brought its own wrapper - at
    // `gap-3`, at `gap-4` - or none, and two fields put straight into one
    // touched. A part that brings a margin of its own, or a wrapper with a
    // step of its own, is how the dialogs drift apart again.
    expect(read('src/components/AppDialog.tsx')).toMatch(
      /DIALOG_STACK = '[^']*\bflex flex-col gap-3\b[^']*'/,
    )
    const offenders = components().flatMap(({ path, text }) => {
      if (path === 'src/components/AppDialog.tsx') return []
      if (!text.includes("from '@/components/AppDialog'")) return []
      return dialogParts(text).flatMap((parts) =>
        parts
          .filter(
            ({ line }) =>
              /className="[^"]*(?<![\w-])m[tby]-\d/.test(line) ||
              // A dialog's only part is its wrapper; one of several is a
              // group with a step of its own - a caption over its control.
              (parts.length === 1 &&
                /className="[^"]*\bflex flex-col gap-(?!3")[\d.]+"/.test(line)),
          )
          .map(({ index }) => `${path}:${index + 1}`),
      )
    })
    expect(offenders).toEqual([])
  })

  it('gives a focus ring room wherever the outside is cut off', () => {
    // A ring is drawn outside its control, and a region that scrolls clips
    // what is outside it: a field against the edge of a widget, a tab or a
    // dialog's body lost a side of its ring (FR-19 19.1). Two defences, each
    // enough for its own case and both kept: a field's ring is drawn inside
    // its box, and a region that scrolls pads its content by the ring's
    // width, so a button or a row against its edge keeps its ring too.
    const fields = rules(entryStylesheet()).filter(({ selectors }) =>
      selectors.some((one) =>
        /input.*textarea.*:focus-visible|textarea\)?:focus-visible/.test(one),
      ),
    )
    expect(
      fields.some(({ body }) => /outline-offset:\s*-2px/.test(body)),
      "a field's ring inside its box",
    ).toBe(true)

    const scroll = read('src/components/Scroll.tsx')
    expect(scroll).toMatch(/const SAFE = 'p-0\.5'/)
    expect(scroll).toMatch(/className=\{cn\(!bound && SAFE, contentClassName\)\}/)
    expect(read('src/components/AppDialog.tsx')).toMatch(/DIALOG_STACK = '[^']*\bpy-1\.5\b[^']*'/)
  })

  it('rings what a clipped region holds inside its box, where the region has no room', () => {
    // The third case of the rule above (v0.167): a region that is only
    // clipped - a panel cut to its rounded corners, a pane that hands its
    // height to what scrolls in it - is no `Scroll` and pads nothing, and a
    // row lying against its edge lost a side of its ring. Such a region says
    // `ring-in`, and the stylesheet draws the rings of what it holds inside
    // their own boxes. A region with padding of its own has the room; one
    // that clips a picture, a bar or a line of text holds nothing to ring.
    const ringIn = rules(entryStylesheet()).filter(({ selectors }) =>
      selectors.some((one) => /\.ring-in\)? :focus-visible/.test(one)),
    )
    expect(
      ringIn.some(({ body }) => /outline-offset:\s*-2px/.test(body)),
      'the rule `ring-in` stands for',
    ).toBe(true)

    const roomy =
      /whitespace-nowrap|(?<![\w-])aspect-|(?<![\w-])size-|(?<![\w-])h-\d|(?<![\w-])p[xytblr]?-(?!0\b)[\d.]+/
    // The frame every screen is drawn in: each screen pads itself.
    const frames = ['src/app/App.tsx']
    const offenders = components()
      .filter(({ path }) => !frames.includes(path))
      .flatMap(({ path, text }) =>
        text
          .split('\n')
          .map((line, index) => ({ line, index }))
          .filter(
            ({ line }) =>
              /\boverflow-hidden\b/.test(line) &&
              !/^\s*(\/\/|\{\/\*|\*)/.test(line) &&
              !line.includes('`overflow-hidden`') &&
              !line.includes('ring-in') &&
              !roomy.test(line),
          )
          .map(({ index }) => `${path}:${index + 1}`),
      )
    expect(offenders).toEqual([])
  })

  it('draws what is read in an ink, never in opacity', () => {
    // The line's rule of contrast (dowel, ADR 0010): an ink on a ground is a
    // promised pair, and a row faded to 60% is no pair at all - it measured
    // under 3:1 in the light theme. What is set aside and still read says
    // `set-aside` and takes the quietest promised ink (v0.167). Opacity stays
    // for what is not read: something being dragged, a mark that is only a
    // picture, a control that appears on hover (0 and 100).
    expect(read('src/styles.css')).toMatch(/@utility set-aside \{\s*color: var\(--color-faint\)/)

    const fades = /(?<![\w-])opacity-(?!0\b|100\b)\d+/
    const notRead = /drag|carr|aria-hidden/
    // A mark that is a picture of a state, with no words in it.
    const pictures = [
      // A vibe taken from the song, drawn quieter than a work's own.
      'src/components/VibePicker.tsx',
      // The swatch of a row that cannot be restored.
      'src/features/trash/TrashView.tsx',
    ]
    const offenders = components()
      .filter(({ path }) => !pictures.includes(path))
      .flatMap(({ path, text }) =>
        text
          .split('\n')
          .map((line, index) => ({ line, index }))
          .filter(({ line }) => fades.test(line) && !notRead.test(line))
          .map(({ index }) => `${path}:${index + 1}`),
      )
    expect(offenders).toEqual([])
  })

  it('never puts the faint ink on a tinted fill', () => {
    // `faint` is promised on the neutral grounds only; on a tint - a status's
    // soft fill, the accent's - the quiet ink is `dim` (dowel, ADR 0010).
    const faint = /(?<![\w:-])text-faint\b/
    const tint = /(?<![\w:-])bg-(?:accent|good|warn|bad|info)(?:-soft)?\b(?!\/)/
    const offenders = components().flatMap(({ path, text }) =>
      text
        .split('\n')
        .map((line, index) => ({ line, index }))
        // One list of classes at a time: a line may hold two, either side
        // of a choice - the tint on one, the faint ink on the other.
        .filter(({ line }) =>
          (line.match(/'[^']*'|"[^"]*"|`[^`]*`/g) ?? []).some(
            (classes) => faint.test(classes) && tint.test(classes),
          ),
        )
        .map(({ index }) => `${path}:${index + 1}`),
    )
    expect(offenders).toEqual([])
  })
})

/**
 * The lines that open a direct child of a `<Dialog>` in a component's source:
 * what the dialog itself spaces apart. Read by indentation, which the
 * formatter guarantees - a child is one step in from its dialog's tag.
 */
function dialogParts(text: string): { line: string; index: number }[][] {
  const lines = text.split('\n')
  const dialogs: { line: string; index: number }[][] = []
  lines.forEach((line, at) => {
    const opening = /^(\s*)<Dialog\b/.exec(line)
    if (opening === null) return
    const indent = opening[1]!
    const closing = lines.findIndex((one, index) => index > at && one === `${indent}</Dialog>`)
    if (closing === -1) return
    // The opening tag ends at the first line that is its `>` alone, or on
    // its own line when it fits there.
    const body = line.trimEnd().endsWith('>')
      ? at + 1
      : lines.findIndex((one, index) => index > at && one === `${indent}>`) + 1
    if (body === 0) return
    const parts: { line: string; index: number }[] = []
    for (let index = body; index < closing; index++) {
      const one = lines[index]!
      if (one.startsWith(`${indent}  <`) && !one.startsWith(`${indent}  </`)) {
        parts.push({ line: one, index })
      }
    }
    dialogs.push(parts)
  })
  return dialogs
}
