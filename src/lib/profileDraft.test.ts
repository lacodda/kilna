import { describe, expect, it } from 'vitest'
import type { ProfileConfig } from '@/lib/api/types'
import { parseProfileDraft, rebase, rebaseDraft, sameValue } from '@/lib/profileDraft'
import music from '@/test/fixtures/profiles/music.json'

/*
 * A kept profile draft carried onto a stored profile that moved on - read
 * against the shipped Studio profile, since the hazard is the real document:
 * the catalogue writing its columns into it, a new build adding actions.
 */
const shipped = music.config as unknown as ProfileConfig

/** A copy to edit, as the editor edits: whole, never the fixture itself. */
function copy(): ProfileConfig {
  return structuredClone(shipped)
}

describe('the same value', () => {
  it('ignores key order', () => {
    expect(sameValue({ a: 1, b: [1, 2] }, { b: [1, 2], a: 1 })).toBe(true)
  })

  it('reads an emptied field as one never written', () => {
    expect(sameValue({ hint: null, limit: undefined }, {})).toBe(true)
    expect(sameValue({ hint: '' }, {})).toBe(false)
  })

  it('holds a list to its order', () => {
    expect(sameValue(['title', 'status'], ['status', 'title'])).toBe(false)
  })
})

describe('a draft carried onto what is stored', () => {
  it('is the draft while nothing moved', () => {
    const draft = copy()
    draft.work_kinds[0]!.label = 'Track'
    expect(rebaseDraft({ base: shipped, config: draft }, shipped)).toEqual(draft)
  })

  it('keeps the columns the catalogue chose meanwhile', () => {
    const draft = copy()
    draft.work_kinds[0]!.axes![0]!.label = 'Hook'
    const stored = copy()
    stored.catalogue_columns = ['title', 'status']

    const merged = rebaseDraft({ base: shipped, config: draft }, stored)
    expect(merged.work_kinds[0]!.axes![0]!.label).toBe('Hook')
    expect(merged.catalogue_columns).toEqual(['title', 'status'])
  })

  it('keeps an action a new build added, beside the one being edited', () => {
    const draft = copy()
    draft.prompts[0]!.template = 'Say it shorter.'
    const stored = copy()
    stored.prompts.push({ key: 'new-in-this-build', label: 'New', template: '' })

    const merged = rebaseDraft({ base: shipped, config: draft }, stored)
    expect(merged.prompts[0]!.template).toBe('Say it shorter.')
    expect(merged.prompts.at(-1)!.key).toBe('new-in-this-build')
    expect(merged.prompts).toHaveLength(shipped.prompts.length + 1)
  })

  it('keeps an entry removed on either side removed', () => {
    const draft = copy()
    const gone = draft.prompts.shift()!
    const stored = copy()
    const alsoGone = stored.work_kinds[0]!.axes!.pop()!
    stored.rhythm = { every_days: 3, default_time: null }

    const merged = rebaseDraft({ base: shipped, config: draft }, stored)
    expect(merged.prompts.map((action) => action.key)).not.toContain(gone.key)
    expect(merged.work_kinds[0]!.axes!.map((axis) => axis.key)).not.toContain(alsoGone.key)
    expect(merged.rhythm).toEqual({ every_days: 3, default_time: null })
  })

  it('gives a part both sides changed to the person editing it', () => {
    expect(rebase({ label: 'A' }, { label: 'Mine' }, { label: 'Theirs' })).toEqual({
      label: 'Mine',
    })
    // A list of plain values is one value: its order is what it says.
    expect(rebase(['a', 'b'], ['b', 'a'], ['a', 'b', 'c'])).toEqual(['b', 'a'])
  })

  it('merges inside an entry both sides changed', () => {
    const base = [{ key: 'x', label: 'X', weight: 1 }]
    const ours = [{ key: 'x', label: 'Mine', weight: 1 }]
    const theirs = [{ key: 'x', label: 'X', weight: 2 }]
    expect(rebase(base, ours, theirs)).toEqual([{ key: 'x', label: 'Mine', weight: 2 }])
  })

  it('reads back after a trip through storage as it went in', () => {
    const draft = copy()
    // What the editor writes for an emptied box, which JSON drops.
    draft.prompts[0]!.description = undefined
    const text = JSON.stringify({ base: shipped, config: draft })
    const kept = parseProfileDraft(text)!
    expect(sameValue(rebaseDraft(kept, shipped), draft)).toBe(true)
  })
})

describe('kept text', () => {
  it('reads anything that is not a draft as none', () => {
    expect(parseProfileDraft(null)).toBeNull()
    expect(parseProfileDraft('{"base":')).toBeNull()
    expect(parseProfileDraft('{"config":{}}')).toBeNull()
    expect(parseProfileDraft('[]')).toBeNull()
  })
})
