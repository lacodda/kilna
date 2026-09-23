import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { SCREEN_NAMES, screenKey } from '@/lib/screens'

describe('screen names', () => {
  it('names every route the app has', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8')
    const segments = [...app.matchAll(/path="\/([a-z]+)/g)].map((match) => match[1])
    // The scan must see the routes, or this test tests nothing.
    expect(segments.length).toBeGreaterThan(8)
    for (const segment of segments) expect(SCREEN_NAMES, segment).toHaveProperty(segment!)
  })

  it('reads the first segment of an address', () => {
    expect(screenKey('/notes/n1')).toBe('nav.notes')
    expect(screenKey('/works/w1/versions')).toBe('nav.catalogue')
    expect(screenKey('/nowhere')).toBe('nav.dashboard')
  })
})
