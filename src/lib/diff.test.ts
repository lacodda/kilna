import { describe, expect, it } from 'vitest'
import { countChanges, diffLines } from '@/lib/diff'

/** `s:`/`a:`/`r:` prefixes, so a whole result reads on one line. */
const kinds = (before: string, after: string) =>
  diffLines(before, after).map((change) => `${change.kind[0]}:${change.text}`)

describe('diffLines', () => {
  it('reports no change when the text is the same', () => {
    expect(kinds('a\nb', 'a\nb')).toEqual(['s:a', 's:b'])
  })

  it('marks an inserted line and leaves its neighbours alone', () => {
    expect(kinds('a\nc', 'a\nb\nc')).toEqual(['s:a', 'a:b', 's:c'])
  })

  it('marks a deleted line', () => {
    expect(kinds('a\nb\nc', 'a\nc')).toEqual(['s:a', 'r:b', 's:c'])
  })

  it('reads a rewritten line as one line out and one line in', () => {
    expect(kinds('a\nold\nc', 'a\nnew\nc')).toEqual(['s:a', 'r:old', 'a:new', 's:c'])
  })

  it('puts the removal before the addition, so the old line reads first', () => {
    // Order is what a reader relies on: "this went, that came" rather than the
    // reverse. It falls out of the tie-break in the walk, which is easy to
    // flip by accident.
    expect(kinds('one', 'two')).toEqual(['r:one', 'a:two'])
  })

  it('counts both sides', () => {
    expect(countChanges(diffLines('a\nb\nc', 'a\nx\ny\nc'))).toEqual({ added: 2, removed: 1 })
  })

  // The property that matters more than any single case: the result has to
  // contain both texts. Keeping what stayed plus what went rebuilds the old
  // version; keeping what stayed plus what arrived rebuilds the new one. A diff
  // that loses a line anywhere breaks one of these.
  it.each([
    ['', 'a'],
    ['a', ''],
    ['a\nb\nc', 'a\nc'],
    ['a\nc', 'a\nb\nc'],
    ['The cranes go still.\nThe water keeps the noise.', 'The cranes go still.\nThe harbour keeps it.\nAnd stops.'],
    ['same\nsame\nsame', 'same\nsame\nsame'],
  ])('rebuilds both sides of %j → %j', (before, after) => {
    const changes = diffLines(before, after)
    expect(changes.filter((c) => c.kind !== 'added').map((c) => c.text).join('\n')).toBe(before)
    expect(changes.filter((c) => c.kind !== 'removed').map((c) => c.text).join('\n')).toBe(after)
  })

  it('refuses to compare two very long texts rather than freezing on them', () => {
    const huge = Array.from({ length: 2001 }, (_, n) => `line ${n}`).join('\n')
    const changes = diffLines(huge, `${huge}\nand one more`)
    // Whole-text removal and addition: honest, and instant.
    expect(changes.map((c) => c.kind)).toEqual(['removed', 'added'])
  })
})
