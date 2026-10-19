import { describe, expect, it } from 'vitest'
import type { LineRow, ShelfView } from '@/lib/api/types'
import { ALL_LINES, linesIn, NO_SHELF, splitLines, sungOf } from '@/lib/lines'

function row(id: string, body: string, shelves: string[] = []): LineRow {
  return {
    id,
    profile_id: 'p',
    work_id: null,
    kind: 'phrase',
    title: null,
    body,
    attribution: null,
    tags: [],
    created_at: 't',
    updated_at: 't',
    layer: 'public',
    aliases: [],
    prompt: null,
    prompt_basis: null,
    state: 'fresh',
    shelf_ids: shelves,
    uses: 0,
    sung: [],
    term_id: null,
  }
}

function shelf(id: string, noteIds: string[]): ShelfView {
  return {
    id,
    profile_id: 'p',
    name: id,
    position: 0,
    note_ids: noteIds,
    created_at: 't',
    updated_at: 't',
  }
}

const ROWS = [row('a', 'a lighthouse', ['sea']), row('b', 'tide'), row('c', 'wave', ['sea'])]
const SHELVES = [shelf('sea', ['c', 'gone', 'a'])]

describe('linesIn', () => {
  it('reads every phrase in the order it came, latest touched first', () => {
    expect(linesIn(ALL_LINES, ROWS, SHELVES).map((one) => one.body)).toEqual([
      'a lighthouse',
      'tide',
      'wave',
    ])
  })

  it('keeps a shelf in the order its phrases were put there, passing over one that went', () => {
    expect(linesIn('sea', ROWS, SHELVES).map((one) => one.body)).toEqual(['wave', 'a lighthouse'])
  })

  it('reads the phrases on no shelf', () => {
    expect(linesIn(NO_SHELF, ROWS, SHELVES).map((one) => one.body)).toEqual(['tide'])
  })
})

describe('splitLines', () => {
  it('splits by lines, not by commas, and keeps a line once whatever its case', () => {
    expect(splitLines('кофе, пульсар и море\nприбой\r\n\n МАЯК \nмаяк')).toEqual([
      'кофе, пульсар и море',
      'прибой',
      'МАЯК',
    ])
    expect(splitLines('  ')).toEqual([])
  })
})

describe('sungOf', () => {
  it('says the sung spelling of the phrase itself, and the other forms with what they are', () => {
    expect(
      sungOf({
        body: 'Марсель',
        sung: [
          { written: 'марсель', sung: 'МарсЭль' },
          { written: 'Марселя', sung: 'МарсЭля' },
        ],
      }),
    ).toBe('МарсЭль, Марселя → МарсЭля')
  })
})
