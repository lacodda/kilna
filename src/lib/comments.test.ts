import { describe, expect, it } from 'vitest'
import { standingOf, workByTitle } from '@/lib/comments'

describe('standingOf', () => {
  it('splits an open comment by whether its reply is drafted', () => {
    expect(standingOf({ state: 'open', reply: null })).toBe('waiting')
    expect(standingOf({ state: 'open', reply: '   ' })).toBe('waiting')
    expect(standingOf({ state: 'open', reply: 'thanks!' })).toBe('drafted')
  })

  it('reads posted and archived as they are, reply or not', () => {
    expect(standingOf({ state: 'posted', reply: null })).toBe('posted')
    expect(standingOf({ state: 'archived', reply: 'x' })).toBe('archived')
  })
})

describe('workByTitle', () => {
  const works = [
    { id: 'a', title: 'Harbour Lights' },
    { id: 'b', title: 'Twin' },
    { id: 'c', title: 'Twin' },
  ]

  it('finds the one work of that title, whatever its quotes and case', () => {
    expect(workByTitle('«harbour  lights»', works)).toBe('a')
    expect(workByTitle('"Harbour Lights"', works)).toBe('a')
  })

  it('does not guess between two works of one title, or at nothing', () => {
    expect(workByTitle('Twin', works)).toBeUndefined()
    expect(workByTitle('Unknown', works)).toBeUndefined()
    expect(workByTitle(undefined, works)).toBeUndefined()
    expect(workByTitle('  ', works)).toBeUndefined()
  })
})
