import { describe, expect, it } from 'vitest'
import type { StressNote } from '@/lib/api/types'
import {
  ACUTE,
  answer,
  answerAll,
  answerEach,
  carryCaret,
  groupNotes,
  hasOneAnswer,
  toggleStress,
  vowelAt,
  vowelNear,
  withAccents,
  wordAt,
  wordsOf,
} from '@/lib/stress'

/*
 * The stress gesture writes the owner's convention (ADR 0053) into the text
 * at the caret: a capital vowel past the first letter, an acute over a first
 * letter, one mark per word, and nothing done to an abbreviation. Every case
 * here is one the backend reads back - the two sides speak one convention.
 */

/** The text after toggling the vowel at `char`'s first occurrence. */
function toggled(text: string, char: string, from = 0): string | undefined {
  return toggleStress(text, text.indexOf(char, from))?.text
}

describe('words', () => {
  it('cuts words as the backend does: letters, marks, a joining hyphen', () => {
    const text = `О${ACUTE}блако из-за гор - и всё.`
    const words = wordsOf(text).map((span) => text.slice(span.start, span.end))
    expect(words).toEqual([`О${ACUTE}блако`, 'из-за', 'гор', 'и', 'всё'])
  })

  it('finds the word a character belongs to, and none for a space', () => {
    expect(wordAt('мой замок', 5)).toEqual({ start: 4, end: 9 })
    expect(wordAt('мой замок', 3)).toBeNull()
  })
})

describe('the stress gesture', () => {
  it('marks a vowel past the first letter with a capital', () => {
    expect(toggled('пульсар', 'а')).toBe('пульсАр')
    expect(toggled('мой замок', 'о', 5)).toBe('мой замОк')
  })

  it('marks a first letter with the acute, keeping its case', () => {
    expect(toggled('Облако', 'О')).toBe(`О${ACUTE}блако`)
    expect(toggled('облако', 'о')).toBe(`о${ACUTE}блако`)
  })

  it('takes a mark off when the marked vowel is toggled again', () => {
    expect(toggled('пульсАр', 'А')).toBe('пульсар')
    expect(toggled(`О${ACUTE}блако`, 'О')).toBe('Облако')
    // The acute itself is part of its vowel.
    expect(toggleStress(`О${ACUTE}блако`, 1)?.text).toBe('Облако')
  })

  it('keeps one mark per word: marking another vowel moves it', () => {
    expect(toggled('зАмок', 'о')).toBe('замОк')
    expect(toggled(`за${ACUTE}мок`, 'о')).toBe('замОк')
    expect(toggled('замОк', 'з') ?? 'nothing').toBe('nothing')
  })

  it('walks е through Е and ё back to е', () => {
    const once = toggled('весна', 'е')!
    expect(once).toBe('вЕсна')
    const twice = toggled(once, 'Е')!
    expect(twice).toBe('вёсна')
    expect(toggled(twice, 'ё')).toBe('весна')
  })

  it('walks a first е through the acute and ё back to е', () => {
    const once = toggled('Ель', 'Е')!
    expect(once).toBe(`Е${ACUTE}ль`)
    const twice = toggleStress(once, 0)!.text
    expect(twice).toBe('Ёль')
    expect(toggled(twice, 'Ё')).toBe('Ель')
  })

  it('turns ё back to е, the way a wrong ё is undone', () => {
    expect(toggled('ещё', 'ё')).toBe('еще')
  })

  it('leaves an abbreviation and a shout as they are, marking with the acute', () => {
    // Two capitals past the first letter are not a mark: lowering one to
    // make room would change the word, and a third capital would mean
    // nothing. The acute marks any letter.
    expect(toggled('ЗАМОК', 'О')).toBe(`ЗАМО${ACUTE}К`)
    expect(toggled(`ЗАМО${ACUTE}К`, 'О')).toBe('ЗАМОК')
    expect(toggled('СССР и ООН', 'О')).toBe(`СССР и О${ACUTE}ОН`)
  })

  it('uses the acute in a word with a capital of its own past the first letter', () => {
    // A second capital would read as an abbreviation, not a stress.
    expect(toggled('НьюЙорк', 'о')).toBe(`НьюЙо${ACUTE}рк`)
  })

  it('does nothing on a consonant, a space or a word with no Russian vowel', () => {
    expect(toggleStress('кот', 0)).toBeNull()
    expect(toggleStress('кот кит', 3)).toBeNull()
    expect(toggleStress('LinkedIn', 1)).toBeNull()
  })

  it('touches only the word it is in', () => {
    expect(toggled('замок\nзамок', 'о', 6)).toBe('замок\nзамОк')
  })

  it('keeps the caret where it was, across a mark added or taken off', () => {
    // A capital changes no length; the caret stays put.
    expect(toggleStress('пульсар и', 5, 9)).toEqual({ text: 'пульсАр и', caret: 9 })
    // An acute added before the caret moves it by one.
    expect(toggleStress('облако дом', 0, 10)).toEqual({ text: `о${ACUTE}блако дом`, caret: 11 })
    // A caret right after the vowel lands after its acute, not between them.
    expect(toggleStress('облако', 0, 1)).toEqual({ text: `о${ACUTE}блако`, caret: 2 })
    // An acute taken off before the caret moves it back.
    expect(toggleStress(`о${ACUTE}блако`, 0, 7)).toEqual({ text: 'облако', caret: 6 })
    // A caret before the word does not move.
    expect(toggleStress('и облако', 2, 0)).toEqual({ text: `и о${ACUTE}блако`, caret: 0 })
  })
})

describe('which vowel a gesture means', () => {
  it('reads a vowel at an index, or under a mark there', () => {
    expect(vowelAt('кот', 1)).toBe(1)
    expect(vowelAt('кот', 0)).toBeNull()
    expect(vowelAt(`о${ACUTE}блако`, 1)).toBe(0)
  })

  it('takes the vowel just typed for the shortcut, or the first at a word start', () => {
    expect(vowelNear('замо', 4)).toBe(3)
    expect(vowelNear('замок', 5)).toBe(3)
    expect(vowelNear('замок', 2)).toBe(1)
    expect(vowelNear('мой замок', 4)).toBe(5)
    expect(vowelNear('мой  ', 5)).toBeNull()
  })
})

const note = (fields: Partial<StressNote> & Pick<StressNote, 'kind' | 'word' | 'start'>) =>
  ({ end: fields.start + fields.word.length, options: [], ...fields }) as StressNote

describe('answering a note', () => {
  it('writes an option over exactly the run the note names', () => {
    const text = 'мой замок стоит'
    const homograph = note({
      kind: 'homograph',
      word: 'замок',
      start: 4,
      options: ['зАмок', 'замОк'],
    })
    expect(answer(text, homograph, 'замОк')).toBe('мой замОк стоит')
  })

  it('writes nothing once the text has moved under the note', () => {
    const stale = note({ kind: 'yo', word: 'еще', start: 0, options: ['ещё'] })
    expect(answer('и еще', stale, 'ещё')).toBeNull()
  })

  it("keeps the word's own case: a first-letter stress stays lowercase mid-line", () => {
    const atlas = note({ kind: 'against', word: 'атлас', start: 2, options: [`А${ACUTE}тлас`] })
    expect(answer('и атлас', atlas, `А${ACUTE}тлас`)).toBe(`и а${ACUTE}тлас`)
  })

  it('applies every note with one answer, and leaves the choices', () => {
    const text = 'еще Марсель и замок'
    const notes = [
      note({ kind: 'yo', word: 'еще', start: 0, options: ['ещё'] }),
      note({ kind: 'unsung', word: 'Марсель', start: 4, options: ['МарсЭль'] }),
      note({ kind: 'homograph', word: 'замок', start: 14, options: ['зАмок', 'замОк'] }),
    ]
    expect(notes.map(hasOneAnswer)).toEqual([true, true, false])
    expect(answerAll(text, notes)).toEqual({
      text: 'ещё МарсЭль и замок',
      replaced: [
        { start: 0, end: 3, length: 3 },
        { start: 4, end: 11, length: 7 },
      ],
    })
  })

  it('writes one answer per run when a word carries two notes', () => {
    const notes = [
      note({ kind: 'yo', word: 'еще', start: 0, options: ['ещё'] }),
      note({ kind: 'against', word: 'еще', start: 0, options: ['ещЕ'] }),
    ]
    expect(answerAll('еще', notes).replaced).toHaveLength(1)
  })

  it('writes one answer at every place a word stands, each in its own case', () => {
    const text = 'Теплый вечер, теплый дом'
    const places = [
      note({ kind: 'yo', word: 'Теплый', start: 0, options: ['тёплый'] }),
      note({ kind: 'yo', word: 'теплый', start: 14, options: ['тёплый'] }),
    ]
    expect(answerEach(text, places, () => 'тёплый')).toEqual({
      text: 'Тёплый вечер, тёплый дом',
      replaced: [
        { start: 0, end: 6, length: 6 },
        { start: 14, end: 20, length: 6 },
      ],
    })
  })

  it('carries the caret over the runs written before it', () => {
    const grew = [{ start: 4, end: 7, length: 5 }]
    expect(carryCaret(2, grew)).toBe(2)
    expect(carryCaret(9, grew)).toBe(11)
    // Inside the run: after what was written there.
    expect(carryCaret(5, grew)).toBe(9)
    const two = [
      { start: 0, end: 2, length: 3 },
      { start: 5, end: 6, length: 1 },
    ]
    expect(carryCaret(8, two)).toBe(9)
  })
})

describe('the strip of notes', () => {
  it('asks once about a word said many times, and about each homograph apart', () => {
    const notes = [
      note({ kind: 'unknown', word: 'Лиссабон', start: 0 }),
      note({ kind: 'homograph', word: 'замок', start: 9, options: ['зАмок', 'замОк'] }),
      note({ kind: 'unknown', word: 'лиссабон', start: 15 }),
      note({ kind: 'homograph', word: 'замок', start: 24, options: ['зАмок', 'замОк'] }),
      note({ kind: 'yo', word: 'еще', start: 30, options: ['ещё'] }),
      note({ kind: 'against', word: 'еще', start: 30, options: ['ещЕ'] }),
    ]
    const groups = groupNotes(notes)
    expect(groups.map((group) => group.notes.map((one) => one.start))).toEqual([
      [0, 15],
      [9],
      [24],
      [30],
      [30],
    ])
    expect(new Set(groups.map((group) => group.key)).size).toBe(groups.length)
  })
})

describe('accents over the stressed vowels', () => {
  it('draws an accent as its own run beside unmarked words', () => {
    expect(withAccents([], [1], 'кот')).toEqual([{ start: 1, end: 2, className: 'stress-accent' }])
  })

  it('cuts a marked word around its accent, the cut edges left open', () => {
    const runs = withAccents([{ start: 0, end: 5, className: 'repeat-0' }], [3], 'замок')
    expect(runs).toEqual([
      { start: 0, end: 3, className: 'repeat-0 mark-open-end' },
      { start: 3, end: 4, className: 'repeat-0 stress-accent mark-open-start mark-open-end' },
      { start: 4, end: 5, className: 'repeat-0 mark-open-start' },
    ])
  })

  it('raises the accent over a capital, and draws none over ё or an acute', () => {
    expect(withAccents([], [5], 'пульсАр')[0]?.className).toBe('stress-accent stress-accent-cap')
    expect(withAccents([], [2], 'ещё')).toEqual([])
    expect(withAccents([], [0], `о${ACUTE}блако`)).toEqual([])
  })
})
