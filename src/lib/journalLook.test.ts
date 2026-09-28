import { describe, expect, it } from 'vitest'
import { Dot, Plus, Star, Trash2, TriangleAlert } from 'lucide-react'
import { journalLook, LOOKED_UP_ACTIONS, needsALook } from '@/lib/journalLook'
import en from '@/i18n/locales/en.json'

/** The actions the window words: the dotted keys of the journal section,
 * each plural folded back to the key the backend records. */
const worded = [
  ...new Set(
    Object.keys(en.journal)
      .filter((key) => key.includes('.'))
      .map((key) => key.replace(/_(zero|one|two|few|many|other)$/, '')),
  ),
]

describe('a line of history', () => {
  // The watchdog: if the section were read wrong, the tests below would pass
  // over nothing.
  it('is read from a locale that words dozens of actions', () => {
    expect(worded.length).toBeGreaterThan(40)
    expect(worded).toContain('status.resynced')
  })

  // The fallback is silent by design, so a sentence added without a look
  // would draw the generic dot and nobody would say so. This says so.
  it('has a look of its own for every action the window words', () => {
    const missing = worded.filter((action) => !LOOKED_UP_ACTIONS.includes(action))
    expect(missing).toEqual([])
  })

  it('looks only after actions the window words', () => {
    const stale = LOOKED_UP_ACTIONS.filter((action) => !worded.includes(action))
    expect(stale).toEqual([])
  })

  it('names the area of every action the window words', () => {
    const unnamed = worded.filter((action) => journalLook({ action, level: 'info' }).kind === null)
    expect(unnamed).toEqual([])
    for (const action of worded) {
      const kind = journalLook({ action, level: 'info' }).kind!
      expect(en.journal.kind, kind).toHaveProperty(kind.replace('journal.kind.', ''))
    }
  })

  it('is tinted by what happened', () => {
    expect(journalLook({ action: 'score.added', level: 'info' })).toMatchObject({
      tone: 'good',
      glyph: Star,
      kind: 'journal.kind.score',
    })
    expect(journalLook({ action: 'work.created', level: 'info' }).tone).toBe('accent')
    expect(journalLook({ action: 'work.status', level: 'info' }).tone).toBe('warn')
    expect(journalLook({ action: 'trash.restored', level: 'info' }).tone).toBe('dim')
  })

  it('is a warning whatever its action says, when it was written as one', () => {
    expect(journalLook({ action: 'score.added', level: 'warn' }).tone).toBe('warn')
    // An action nobody gave a glyph takes the warning's rather than the dot.
    expect(journalLook({ action: 'plan.broken', level: 'warn' }).glyph).toBe(TriangleAlert)
  })

  it('is still drawn when a newer backend wrote an action the window has no look for', () => {
    expect(journalLook({ action: 'track.deleted', level: 'info' })).toMatchObject({
      tone: 'dim',
      glyph: Trash2,
      kind: null,
    })
    expect(journalLook({ action: 'track.created', level: 'info' }).glyph).toBe(Plus)
    expect(journalLook({ action: 'something', level: 'info' }).glyph).toBe(Dot)
  })

  // A prototype key is not an action. Without `hasOwn` the lookup would hand
  // back `Object.prototype.constructor` as the tone and the glyph.
  it('is not fooled by inherited object keys', () => {
    expect(journalLook({ action: 'constructor', level: 'info' }).glyph).toBe(Dot)
    expect(journalLook({ action: 'toString.x', level: 'info' }).kind).toBeNull()
  })

  it('asks for a look while it is a warning nobody has marked seen', () => {
    expect(needsALook({ level: 'warn', read_at: null })).toBe(true)
    expect(needsALook({ level: 'warn', read_at: '2026-09-15T10:00:00Z' })).toBe(false)
    expect(needsALook({ level: 'info', read_at: null })).toBe(false)
  })
})
