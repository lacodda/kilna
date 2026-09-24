import { describe, expect, it } from 'vitest'
import { start, step, type DraftEvent, type DraftState } from '@/lib/fieldDraft'

/** Run events in order and collect what was committed. */
function run(stored: string, events: DraftEvent[]): { state: DraftState; commits: string[] } {
  let state = start(stored)
  const commits: string[] = []
  for (const event of events) {
    const next = step(state, event)
    state = next.state
    if (next.commit !== null) commits.push(next.commit)
  }
  return { state, commits }
}

describe('a field draft', () => {
  it('writes nothing when a field is only passed through', () => {
    // Tabbing across a form used to write every field it touched.
    const { commits } = run('120', [{ type: 'focus' }, { type: 'leave' }])
    expect(commits).toEqual([])
  })

  it('writes what was typed when the field is left', () => {
    const { commits } = run('120', [{ type: 'focus' }, { type: 'type', text: '124' }, { type: 'leave' }])
    expect(commits).toEqual(['124'])
  })

  it('writes nothing when the typing ends where it started', () => {
    const { commits } = run('120', [
      { type: 'focus' },
      { type: 'type', text: '12' },
      { type: 'type', text: '120' },
      { type: 'leave' },
    ])
    expect(commits).toEqual([])
  })

  it('follows the stored value while nobody is in the field', () => {
    // A plugin wrote 128; the box has to show it, or the next blur writes the
    // old value back over it.
    const { state, commits } = run('120', [{ type: 'stored', value: '128' }, { type: 'focus' }, { type: 'leave' }])
    expect(state.draft).toBe('128')
    expect(commits).toEqual([])
  })

  it('keeps the words of whoever is typing when the stored value moves', () => {
    const { state, commits } = run('120', [
      { type: 'focus' },
      { type: 'type', text: '124' },
      { type: 'stored', value: '128' },
      { type: 'leave' },
    ])
    expect(commits).toEqual(['124'])
    expect(state.stored).toBe('128')
  })

  it('puts the stored value back on Escape and writes nothing', () => {
    const { state, commits } = run('120', [
      { type: 'focus' },
      { type: 'type', text: '999' },
      { type: 'escape' },
      { type: 'leave' },
    ])
    expect(state.draft).toBe('120')
    expect(commits).toEqual([])
  })
})
