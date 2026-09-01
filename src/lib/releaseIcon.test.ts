import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { CalendarDays, Film } from 'lucide-react'
import { describe, expect, it } from 'vitest'
import { iconNames, knownIcon, releaseIcon } from './releaseIcon'

describe('releaseIcon', () => {
  it('draws the glyph the profile names', () => {
    expect(releaseIcon('film')).toBe(Film)
  })

  // Three ways a name can be absent, one answer. A chip with a gap where its
  // neighbours have a mark reads as broken; a neutral mark reads as "a release
  // of some kind", which is what is actually known.
  it('falls back to a neutral glyph when the name is missing or unknown', () => {
    expect(releaseIcon(null)).toBe(CalendarDays)
    expect(releaseIcon(undefined)).toBe(CalendarDays)
    expect(releaseIcon('no-such-glyph')).toBe(CalendarDays)
  })

  // The set is closed on purpose: the name arrives from a file the owner edits,
  // so it must not reach anything the icon module happens to export.
  it('refuses a name that is not in the set, however real it is elsewhere', () => {
    // Real exports of lucide-react, deliberately left out of the vocabulary.
    expect(knownIcon('Trash2')).toBe(false)
    expect(knownIcon('Lock')).toBe(false)
    expect(releaseIcon('Trash2')).toBe(CalendarDays)
  })

  // A prototype key is not a glyph. Without `Object.hasOwn` the lookup would
  // hand back `Object.prototype.constructor` and render it as a component.
  it('is not fooled by inherited object keys', () => {
    expect(knownIcon('constructor')).toBe(false)
    expect(knownIcon('toString')).toBe(false)
    expect(releaseIcon('constructor')).toBe(CalendarDays)
    expect(releaseIcon('toString')).toBe(CalendarDays)
  })

  it('lists its vocabulary', () => {
    const names = iconNames()
    expect(names).toContain('film')
    expect(names.length).toBeGreaterThan(0)
    expect([...names].sort()).toEqual(names)
    for (const name of names) expect(knownIcon(name)).toBe(true)
  })
})

// The vocabulary and the shipped profiles are two files that have to agree. A
// glyph renamed here and not there costs every chip of that kind its mark, and
// nothing else would say so: the fallback is silent by design.
describe('the shipped profiles', () => {
  const dir = fileURLToPath(new URL('../../src-tauri/profiles/', import.meta.url))

  it('name a glyph the vocabulary holds, for every release kind', () => {
    const files = readdirSync(dir).filter((name) => name.endsWith('.json'))
    expect(files.length).toBeGreaterThan(0)

    for (const file of files) {
      const profile = JSON.parse(readFileSync(dir + file, 'utf8')) as {
        config: { release_kinds: { key: string; icon?: string | null }[] }
      }
      for (const kind of profile.config.release_kinds) {
        // Stated, and stated from the vocabulary. The first half matters as
        // much as the second: a shipped kind with no glyph would fall back to
        // the neutral mark and look exactly like a kind whose glyph was
        // renamed away -- silently, which is the whole reason this test exists.
        // Both facts go into one comparison so the failure names the profile
        // and the kind rather than just saying `false`.
        expect([file, kind.key, typeof kind.icon, knownIcon(kind.icon ?? '')]).toEqual([
          file,
          kind.key,
          'string',
          true,
        ])
      }
    }
  })
})
