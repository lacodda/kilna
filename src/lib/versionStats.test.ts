import { describe, expect, it } from 'vitest'
import { statsOf } from '@/lib/versionStats'

describe('statsOf', () => {
  it('counts the words and lines of the text, not its headers or blank lines', () => {
    const body = '[Verse 1]\nLanterns on the water\nfloating out of reach\n\n[Chorus]\nPaper, paper'
    expect(statsOf(body, null)).toEqual({ words: 10, lines: 3, change: null })
  })

  it('reads a word in any script, with its apostrophe or hyphen', () => {
    expect(statsOf("ты — мой свет, don't stop, well-known", null)).toMatchObject({
      words: 6,
      lines: 1,
    })
  })

  it('counts the lines in and out since the predecessor, as the comparison does', () => {
    const before = 'one\ntwo\nthree'
    const after = 'one\ntwo, changed\nthree\nfour'
    expect(statsOf(after, before).change).toEqual({ added: 2, removed: 1 })
    expect(statsOf(before, before).change).toEqual({ added: 0, removed: 0 })
  })

  it('says nothing for an empty text', () => {
    expect(statsOf('', null)).toEqual({ words: 0, lines: 0, change: null })
  })
})
