import { describe, expect, it } from 'vitest'
import { wordItems } from '@/lib/words'

describe('wordItems', () => {
  it('names each word the way an apply keeps it', () => {
    expect(wordItems({ words: [{ word: 'a' }, { word: 'b' }] })).toEqual(['word:0', 'word:1'])
  })
})
