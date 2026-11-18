import { describe, expect, it } from 'vitest'
import { inAppLinks } from '@/lib/wikilink'
import { selectionsOf } from '@/lib/selections'
import { outlineOf } from './NoteDetail'

// v0.145, FR-09: a document read by its outline, kilna's links followed in
// the window, selections kept as the storage holds them.
describe('documents and their links', () => {
  it('reads a document by its first three levels of heading, outside code', () => {
    const text =
      '# Plan\n\nintro\n\n## Goals ##\n```\n# not a heading\n```\n### Week one\n#### too deep'
    expect(outlineOf(text)).toEqual([
      { level: 1, text: 'Plan' },
      { level: 2, text: 'Goals' },
      { level: 3, text: 'Week one' },
    ])
  })

  it("follows kilna's links as the window's own routes", () => {
    expect(
      inAppLinks(
        'See [a](kilna://note/n1), [b](kilna://canon/c-2), [c](kilna://work/w_3), [d](kilna://works/w4) and [e](https://x.org).',
      ),
    ).toBe(
      'See [a](/notes/n1), [b](/canon/c-2), [c](/works/w_3), [d](/works/w4) and [e](https://x.org).',
    )
  })

  it('keeps the selections a stored value holds, and nothing else', () => {
    expect(selectionsOf('[{"kind":"document","tag":"strategy"},{"kind":3},"x"]')).toEqual([
      { kind: 'document', tag: 'strategy' },
    ])
    expect(selectionsOf('not json')).toEqual([])
  })
})
