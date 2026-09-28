import { describe, expect, it } from 'vitest'
import type { ProfileConfig } from '@/lib/api/types'
import { drawn, JUMPS, onRail, railAt, SCREENS, SOON, screenAt } from '@/app/screens'
import { CARD_TABS, SCREENS as PLACES } from '@/test/places'
import { studio } from '@/test/workspace'
import { version } from '../../package.json'

/*
 * The one list of screens, held to the rules the four places that read it
 * used to keep by hand.
 */

describe('the screens', () => {
  it('are each named by the first segment of their address', () => {
    for (const screen of SCREENS) expect(screen.path).toMatch(new RegExp(`^/${screen.key}(/|$)`))
    const keys = SCREENS.map((screen) => screen.key)
    expect(new Set(keys).size).toBe(keys.length)
  })

  it('give each shortcut letter to one screen, and every letter a real address', () => {
    const letters = SCREENS.flatMap((screen) => (screen.jump === undefined ? [] : [screen.jump]))
    expect(new Set(letters).size).toBe(letters.length)
    for (const letter of letters) expect(letter).toMatch(/^[a-z]$/)
    for (const where of Object.values(JUMPS)) {
      expect(SCREENS.some((screen) => `/${screen.key}` === where)).toBe(true)
    }
  })

  it('are all in the rail, but for an open work, which is reached from the others', () => {
    const unreachable = SCREENS.filter((screen) => screen.rail === undefined).map((s) => s.key)
    expect(unreachable).toEqual(['works'])
  })

  it('are each opened by the smoke test', () => {
    // A screen added to the list and not to the places the smoke test opens
    // would be the one screen nobody checks.
    const opened = (key: string) =>
      key === 'works'
        ? CARD_TABS.length > 0
        : PLACES.some(([path]) => path === `/${key}` || path.startsWith(`/${key}/`))
    const missing = drawn(false)
      .map((screen) => screen.key)
      .filter((key) => !opened(key))
    expect(missing).toEqual([])
  })

  it('read the screen at an address, and the dashboard for one nobody has', () => {
    expect(screenAt('/notes/n1').key).toBe('notes')
    expect(screenAt('/works/w1/versions').nav).toBe('nav.catalogue')
    expect(screenAt('/nowhere').key).toBe('dashboard')
  })

  it('light the rail entry of the screen open, or of the one it is named after', () => {
    expect(railAt('/notes/n1')).toBe('notes')
    expect(railAt('/settings/general')).toBe('settings')
    // An open work is not in the rail; it belongs to the catalogue, which is
    // what its trail says too.
    expect(railAt('/works/w1/versions')).toBe('catalogue')
    for (const screen of SCREENS) {
      const lit = railAt(`/${screen.key}`)
      expect(lit, screen.key).toBeDefined()
      expect(SCREENS.find((other) => other.key === lit)?.rail, screen.key).toBeDefined()
    }
  })

  it('offer the rail and the palette what the build and the craft have', () => {
    const config = studio().profile.config
    const keys = (dev: boolean, craft: ProfileConfig) => onRail(dev, craft).map((s) => s.key)

    // Every screen with a door, none without one, in the list's own order.
    expect(keys(true, config)).toEqual(
      SCREENS.filter((screen) => screen.rail !== undefined).map((screen) => screen.key),
    )
    expect(keys(false, config)).not.toContain('styleguide')
    // A craft with no style types has no dictionary, and no door to one.
    expect(keys(true, { ...config, style_types: [] })).not.toContain('styles')
  })
})

describe('a screen still to come', () => {
  it('names a version later than this one', () => {
    // A dimmed door saying "coming in 0.87" after 0.87 shipped is a promise
    // broken in plain sight - which is what the collections door said.
    const later = (a: string, b: string) => {
      const [x, y] = [a, b].map((v) => v.split('.').map(Number))
      for (let i = 0; i < 3; i += 1) {
        if ((x![i] ?? 0) !== (y![i] ?? 0)) return (x![i] ?? 0) > (y![i] ?? 0)
      }
      return false
    }
    for (const soon of SOON) expect(later(soon.version, version), soon.nav).toBe(true)
  })
})
