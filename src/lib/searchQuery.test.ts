import { describe, expect, it } from 'vitest'
import { formatQuery, parseQuery, type Vocabulary } from './searchQuery'
import type { CatalogueFilter } from '@/lib/catalogue'

const vocabulary: Vocabulary = {
  statuses: [
    { key: 'draft', label: 'Draft' },
    { key: 'ready', label: 'Ready' },
  ],
  kinds: [{ key: 'song', label: 'Song' }],
  tiers: [
    { key: 'clip', label: 'Clip' },
    { key: 'pic', label: 'Picture' },
  ],
}

describe('parseQuery', () => {
  it('reads a bare line as text and nothing else', () => {
    const parsed = parseQuery('harbour lights', vocabulary)

    expect(parsed.text).toBe('harbour lights')
    expect(parsed.filter).toEqual({})
    expect(parsed.unknown).toEqual([])
  })

  it('reads operators and free text from one line', () => {
    const parsed = parseQuery('winter tier:clip status:draft', vocabulary)

    expect(parsed.filter).toEqual({ tier: 'clip', status: 'draft' })
    expect(parsed.text).toBe('winter')
  })

  it('keeps a quoted phrase whole', () => {
    const parsed = parseQuery('"paper boats" tier:clip', vocabulary)

    expect(parsed.text).toBe('paper boats')
    expect(parsed.filter.tier).toBe('clip')
  })

  it('accepts the label a person can see, not only the key', () => {
    // The screen says "Picture"; the key is `pic`. Typing what is on screen is
    // not a mistake.
    expect(parseQuery('tier:Picture', vocabulary).filter.tier).toBe('pic')
    expect(parseQuery('status:READY', vocabulary).filter.status).toBe('ready')
  })

  it('takes a tag as written, since tags have no fixed list', () => {
    expect(parseQuery('tag:зима', vocabulary).filter.tag).toBe('зима')
  })

  it('resolves a value regardless of case, in Russian too', () => {
    const russian: Vocabulary = {
      statuses: [{ key: 'draft', label: 'Черновик' }],
      kinds: [],
      tiers: [],
    }
    expect(parseQuery('status:ЧЕРНОВИК', russian).filter.status).toBe('draft')
  })

  // The reason unknown fields are not an error: a title can contain a colon,
  // and refusing it would make the box worse than the plain one it replaced.
  it('leaves a colon it does not recognise inside the text', () => {
    const parsed = parseQuery('Ratio: a love song', vocabulary)

    expect(parsed.text).toBe('Ratio: a love song')
    expect(parsed.filter).toEqual({})
  })

  it('reports a value this profile does not have instead of silently dropping it', () => {
    const parsed = parseQuery('tier:gold', vocabulary)

    expect(parsed.filter.tier).toBeUndefined()
    expect(parsed.unknown).toEqual([{ field: 'tier', value: 'gold' }])
    // Not swept into the text either — that would search titles for "tier:gold".
    expect(parsed.text).toBe('')
  })

  it('ignores a field being typed, before its value exists', () => {
    const parsed = parseQuery('tier:', vocabulary)

    expect(parsed.filter).toEqual({})
    expect(parsed.unknown).toEqual([])
  })

  it('keeps the last of a repeated field', () => {
    expect(parseQuery('tier:clip tier:pic', vocabulary).filter.tier).toBe('pic')
  })

  it('ignores the case of the field name', () => {
    expect(parseQuery('TIER:clip', vocabulary).filter.tier).toBe('clip')
  })

  it('holds a value with a space in it when quoted', () => {
    const spaced: Vocabulary = { ...vocabulary, tiers: [{ key: 'b', label: 'B side' }] }
    expect(parseQuery('tier:"B side"', spaced).filter.tier).toBe('b')
  })

  it('is empty for an empty line', () => {
    const parsed = parseQuery('   ', vocabulary)

    expect(parsed.text).toBe('')
    expect(parsed.filter).toEqual({})
  })
})

describe('formatQuery', () => {
  it('writes a filter back as a line the parser reads the same way', () => {
    const filter: CatalogueFilter = {
      status: 'draft',
      tier: 'clip',
      tag: 'winter',
      search: 'harbour',
    }

    const line = formatQuery(filter)
    const parsed = parseQuery(line, vocabulary)

    expect(parsed.filter.status).toBe('draft')
    expect(parsed.filter.tier).toBe('clip')
    expect(parsed.filter.tag).toBe('winter')
    expect(parsed.text).toBe('harbour')
  })

  it('quotes a value that would otherwise come back as two terms', () => {
    const line = formatQuery({ tag: 'late winter' })

    expect(line).toBe('tag:"late winter"')
    expect(parseQuery(line, vocabulary).filter.tag).toBe('late winter')
  })

  it('quotes free text with a space in it', () => {
    const line = formatQuery({ search: 'paper boats' })

    expect(parseQuery(line, vocabulary).text).toBe('paper boats')
  })

  it('is empty when nothing narrows', () => {
    expect(formatQuery({})).toBe('')
  })

  // The gap chips are not part of the line: they have their own row of
  // controls, and writing them into the box would give two ways to unset one
  // thing that disagree.
  it('leaves the gap out of the line', () => {
    expect(formatQuery({ gap: 'unscored' })).toBe('')
  })
})
