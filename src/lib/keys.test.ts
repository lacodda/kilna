import { describe, expect, it } from 'vitest'
import { DESTINATIONS, opensChord, readChord, readStroke, type Stroke } from './keys'

/** A keystroke with nothing held and nothing focused. */
function stroke(key: string, over: Partial<Stroke> = {}): Stroke {
  return {
    key,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    typing: false,
    ...over,
  }
}

describe('readStroke', () => {
  it('reads ? as the shortcut list', () => {
    expect(readStroke(stroke('?'))).toEqual({ kind: 'help' })
  })

  it('reads ? even though the layout produced it with shift', () => {
    expect(readStroke(stroke('?', { shiftKey: true }))).toEqual({ kind: 'help' })
  })

  it('leaves ? alone while text is being typed', () => {
    expect(readStroke(stroke('?', { typing: true }))).toBeNull()
  })

  it('walks the history on the browser keys', () => {
    expect(readStroke(stroke('ArrowLeft', { altKey: true }))).toEqual({
      kind: 'history',
      delta: -1,
    })
    expect(readStroke(stroke('ArrowRight', { altKey: true }))).toEqual({
      kind: 'history',
      delta: 1,
    })
  })

  it('still walks the history from inside a field', () => {
    // Alt+arrow types nothing, so it interrupts nothing — and someone editing
    // a version still expects the back button's keyboard equivalent to work.
    expect(readStroke(stroke('ArrowLeft', { altKey: true, typing: true }))).toEqual({
      kind: 'history',
      delta: -1,
    })
  })

  it('leaves a bare arrow to whatever is focused', () => {
    // The version list and the calendar both walk on bare arrows; stealing
    // them here would break the list under the cursor.
    expect(readStroke(stroke('ArrowLeft'))).toBeNull()
    expect(readStroke(stroke('ArrowRight'))).toBeNull()
  })

  it('does not read a history key that carries another modifier', () => {
    expect(readStroke(stroke('ArrowLeft', { altKey: true, ctrlKey: true }))).toBeNull()
    expect(readStroke(stroke('ArrowLeft', { altKey: true, shiftKey: true }))).toBeNull()
    expect(readStroke(stroke('ArrowLeft', { altKey: true, metaKey: true }))).toBeNull()
  })

  it('is silent on an ordinary key', () => {
    expect(readStroke(stroke('a'))).toBeNull()
    expect(readStroke(stroke('Enter'))).toBeNull()
    expect(readStroke(stroke('Escape'))).toBeNull()
  })

  it('does not read a shortcut out of a browser chord', () => {
    // Ctrl+D bookmarks a page in a browser; it must never navigate here.
    expect(readStroke(stroke('d', { ctrlKey: true }))).toBeNull()
    expect(readStroke(stroke('?', { ctrlKey: true }))).toBeNull()
    expect(readStroke(stroke('?', { metaKey: true }))).toBeNull()
  })
})

describe('opensChord', () => {
  it('opens on a bare g', () => {
    expect(opensChord(stroke('g'))).toBe(true)
  })

  it('does not open while text is being typed', () => {
    expect(opensChord(stroke('g', { typing: true }))).toBe(false)
  })

  it('does not open on a g that carries a modifier', () => {
    expect(opensChord(stroke('g', { ctrlKey: true }))).toBe(false)
    expect(opensChord(stroke('g', { metaKey: true }))).toBe(false)
    expect(opensChord(stroke('g', { altKey: true }))).toBe(false)
    expect(opensChord(stroke('g', { shiftKey: true }))).toBe(false)
  })

  it('does not open on another letter', () => {
    expect(opensChord(stroke('h'))).toBe(false)
    expect(opensChord(stroke('G'))).toBe(false)
  })
})

describe('readChord', () => {
  it('sends every destination somewhere', () => {
    for (const [key, where] of Object.entries(DESTINATIONS)) {
      expect(readChord(stroke(key))).toEqual({ kind: 'go', where })
    }
  })

  it('reads a capital as the same destination', () => {
    // Shift held through a chord is a slip, not a different intent.
    expect(readChord(stroke('D', { shiftKey: true }))).toEqual({ kind: 'go', where: '/dashboard' })
  })

  it('gives nothing for a letter that goes nowhere', () => {
    expect(readChord(stroke('q'))).toBeNull()
    expect(readChord(stroke('Enter'))).toBeNull()
  })

  it('gives nothing while text is being typed', () => {
    expect(readChord(stroke('d', { typing: true }))).toBeNull()
  })

  it('gives nothing when a modifier is held', () => {
    expect(readChord(stroke('d', { ctrlKey: true }))).toBeNull()
    expect(readChord(stroke('d', { metaKey: true }))).toBeNull()
    expect(readChord(stroke('d', { altKey: true }))).toBeNull()
  })

  it('sends no two letters to the same screen', () => {
    const screens = Object.values(DESTINATIONS)
    expect(new Set(screens).size).toBe(screens.length)
  })

  it('names a real screen for every letter', () => {
    // A destination that does not start with a slash is not a route, and the
    // router would silently send it to the dashboard.
    for (const where of Object.values(DESTINATIONS)) expect(where).toMatch(/^\/[a-z]+$/)
  })
})
