import { describe, expect, it } from 'vitest'
import type { BlockView, RegisterEntry } from '@/lib/api/types'
import { ALL_WORDS, NO_BLOCK, splitWords, wordItems, wordsIn } from '@/lib/words'

function entry(id: string, word: string, bank: RegisterEntry['bank']): RegisterEntry {
  return {
    id,
    profile_id: 'p',
    word,
    forms: [],
    kind: 'noun',
    strictness: null,
    bank,
    sung: [],
    topic: null,
    note: null,
    created_at: 't',
    updated_at: 't',
    uses: 0,
  }
}

function block(id: string, termIds: string[]): BlockView {
  return {
    id,
    profile_id: 'p',
    name: id,
    position: 0,
    term_ids: termIds,
    created_at: 't',
    updated_at: 't',
  }
}

const ENTRIES = [
  entry('a', 'wave', 'dropped'),
  entry('b', 'anchor', 'parked'),
  entry('c', 'tide', 'fresh'),
  entry('d', 'beacon', 'fresh'),
  entry('e', 'spent', null),
  // Out of the bank, but still in a block: the block keeps showing it.
  entry('f', 'buoy', null),
]
const BLOCKS = [block('sea', ['f', 'a', 'gone', 'c'])]

describe('wordsIn', () => {
  it('reads the whole bank fresh first, then by the word, with what a block still holds', () => {
    expect(wordsIn(ALL_WORDS, ENTRIES, BLOCKS).map((one) => one.word)).toEqual([
      'beacon',
      'tide',
      'anchor',
      'wave',
      'buoy',
    ])
  })

  it('keeps a block in the order its words were put there, passing over one that went', () => {
    expect(wordsIn('sea', ENTRIES, BLOCKS).map((one) => one.word)).toEqual(['buoy', 'wave', 'tide'])
  })

  it('reads the banked words in no block', () => {
    expect(wordsIn(NO_BLOCK, ENTRIES, BLOCKS).map((one) => one.word)).toEqual(['beacon', 'anchor'])
  })
})

describe('splitWords', () => {
  it('splits by commas and lines, not by spaces, and keeps a word once', () => {
    expect(splitWords('маяк, on the edge\nприбой\r\n\n, Маяк ,')).toEqual([
      'маяк',
      'on the edge',
      'прибой',
    ])
    expect(splitWords('  ')).toEqual([])
  })
})

describe('wordItems', () => {
  it('names each word the way an apply keeps it', () => {
    expect(wordItems({ words: [{ word: 'a' }, { word: 'b' }] })).toEqual(['word:0', 'word:1'])
  })
})
