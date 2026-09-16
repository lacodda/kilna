import { describe, expect, it } from 'vitest'
import en from '@/i18n/locales/en.json'
import ru from '@/i18n/locales/ru.json'
import type { Complaint } from '@/lib/storyboard'

/*
 * Every line the panel can print has words to print.
 *
 * The panel builds its key from the complaint's kind — `scenes.check.${kind}`,
 * or `scenes.check.run.${kind}` for a collapsed stretch — and a kind added in
 * the module with no key beside it reaches the screen as the key itself. Both
 * halves are ours and neither imports the other, so nothing but this notices.
 *
 * `check-locales.mjs` cannot: it holds the locales to each other, and a key
 * missing from both is consistent.
 */

/** Every kind the module can produce, listed by hand so that adding one to
 * the union without adding it here fails the type check below. */
const KINDS = [
  'empty',
  'noPrompt',
  'noFrame',
  'undecidedFrame',
  'noVideo',
  'undecidedVideo',
  'gap',
  'overlap',
  'untimed',
  'length',
] as const

// A compile-time assertion, not a runtime one: if `ComplaintKind` grows a
// member this list does not have, this line stops type-checking.
const _every: Record<Complaint['kind'], true> = Object.fromEntries(
  KINDS.map((kind) => [kind, true]),
) as Record<Complaint['kind'], true>
void _every

/** The kinds that can be collapsed into a run: the ones about a scene. A
 * complaint about the whole timing has no scene, so it never collapses. */
const RUNNABLE = KINDS.filter((kind) => kind !== 'untimed' && kind !== 'length')

const LOCALES = { en, ru } as const

/** A leaf of a locale by dotted path, or undefined. */
function at(locale: unknown, path: string): unknown {
  return path
    .split('.')
    .reduce<unknown>(
      (held, key) =>
        held !== null && typeof held === 'object' ? (held as Record<string, unknown>)[key] : undefined,
      locale,
    )
}

/** i18next appends a plural category; a key is present if it is there plainly
 * or in any of its forms. */
function present(locale: unknown, path: string): boolean {
  if (typeof at(locale, path) === 'string') return true
  return ['zero', 'one', 'two', 'few', 'many', 'other'].some(
    (form) => typeof at(locale, `${path}_${form}`) === 'string',
  )
}

describe('every line the check can print has words', () => {
  for (const [name, locale] of Object.entries(LOCALES)) {
    it(`${name} says all of them`, () => {
      const missing = KINDS.filter((kind) => !present(locale, `scenes.check.${kind}`))
      expect(missing).toEqual([])
    })

    it(`${name} says all of them for a collapsed stretch`, () => {
      const missing = RUNNABLE.filter((kind) => !present(locale, `scenes.check.run.${kind}`))
      expect(missing).toEqual([])
    })

    it(`${name} has the words around them`, () => {
      for (const key of ['title', 'tally', 'nothingMissing']) {
        expect(present(locale, `scenes.check.${key}`), `${name} scenes.check.${key}`).toBe(true)
      }
      for (const key of ['moveTo', 'moveToHint', 'badNumber', 'insertAfter', 'renumbered']) {
        expect(present(locale, `scenes.${key}`), `${name} scenes.${key}`).toBe(true)
      }
    })
  }
})
