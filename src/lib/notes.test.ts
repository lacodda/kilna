import { describe, expect, it } from 'vitest'
import { tagsFrom, titleOf } from '@/lib/notes'

describe('titleOf', () => {
  it('prefers the title', () => {
    expect(titleOf({ title: ' Graphite ', body: 'x' })).toBe('Graphite')
  })

  it('falls back to the first line that says something, undressed', () => {
    expect(titleOf({ title: null, body: '\n## A layer of graphite\nmore' })).toBe('A layer of graphite')
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
