import { describe, expect, it } from 'vitest'
import { SESSION_PAUSE_MS, begun, continues, touched } from './editing'

const NOW = 1_700_000_000_000

describe('continues', () => {
  it('keeps writing into the version the session minted', () => {
    const session = begun('v2', 'lyrics', NOW)
    expect(continues(session, 'lyrics', 'v2', NOW + 1000)).toBe(true)
  })

  it('starts over when the text open on screen is another version', () => {
    const session = begun('v2', 'lyrics', NOW)
    expect(continues(session, 'lyrics', 'v1', NOW + 1000)).toBe(false)
  })

  it('starts over in another role, even for the same id', () => {
    const session = begun('v2', 'lyrics', NOW)
    expect(continues(session, 'style', 'v2', NOW + 1000)).toBe(false)
  })

  it('starts over after a pause', () => {
    const session = begun('v2', 'lyrics', NOW)
    expect(continues(session, 'lyrics', 'v2', NOW + SESSION_PAUSE_MS)).toBe(true)
    expect(continues(session, 'lyrics', 'v2', NOW + SESSION_PAUSE_MS + 1)).toBe(false)
  })

  it('measures the pause from the last keystroke, not from the start', () => {
    const session = touched(begun('v2', 'lyrics', NOW), NOW + SESSION_PAUSE_MS - 1)
    expect(continues(session, 'lyrics', 'v2', NOW + 2 * SESSION_PAUSE_MS - 2)).toBe(true)
  })

  it('has nothing to continue without a session or an open version', () => {
    expect(continues(null, 'lyrics', 'v2', NOW)).toBe(false)
    expect(continues(begun('v2', 'lyrics', NOW), 'lyrics', null, NOW)).toBe(false)
  })
})
