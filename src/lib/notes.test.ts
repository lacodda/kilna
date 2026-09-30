import { describe, expect, it } from 'vitest'
import type { ProfileConfig } from '@/lib/api/types'
import { isLine, isMaterial, tagsFrom, titleOf } from '@/lib/notes'

describe('titleOf', () => {
  it('prefers the title', () => {
    expect(titleOf({ title: ' Graphite ', body: 'x' })).toBe('Graphite')
  })

  it('falls back to the first line that says something, undressed', () => {
    expect(titleOf({ title: null, body: '\n## A layer of graphite\nmore' })).toBe(
      'A layer of graphite',
    )
    expect(titleOf({ title: '', body: '- [ ] check the layer' })).toBe('check the layer')
    expect(titleOf({ title: null, body: '> quoted thought' })).toBe('quoted thought')
  })

  it('is empty for an empty note, and short for a long line', () => {
    expect(titleOf({ title: null, body: '   \n' })).toBe('')
    expect(titleOf({ title: null, body: 'x'.repeat(200) })).toHaveLength(80)
  })
})

describe('tagsFrom', () => {
  it('splits, trims and drops empties and repeats', () => {
    expect(tagsFrom(' geology, time,, Geology ,time ')).toEqual(['geology', 'time'])
  })
})

describe('material and line kinds', () => {
  const config = {
    note_kinds: [
      { key: 'idea', label: 'Idea', material: true },
      { key: 'phrase', label: 'Phrase', material: true, line: true },
      { key: 'note', label: 'Note' },
    ],
  } as unknown as ProfileConfig

  it('reads the flags off the profile, and nothing off a kind it does not name', () => {
    expect(isMaterial(config, 'idea')).toBe(true)
    expect(isLine(config, 'idea')).toBe(false)
    expect(isLine(config, 'phrase')).toBe(true)
    expect(isMaterial(config, 'note')).toBe(false)
    expect(isMaterial(config, 'limerick')).toBe(false)
    expect(isLine(config, undefined)).toBe(false)
  })
})
