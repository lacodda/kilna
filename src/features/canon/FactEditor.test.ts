import { describe, expect, it } from 'vitest'
import { linesOf } from './FactEditor'

// v0.142, FR-18 18.3: a pasted list is one fact a line.
describe('a list of facts', () => {
  it('is one fact a line, its bullets and numbers taken off', () => {
    expect(
      linesOf(
        '- Gradient hair.\n* Freckles.\n\n• A grey coat.\n1. Counts pauses.\n2) Hums.\r\n  plain  ',
      ),
    ).toEqual(['Gradient hair.', 'Freckles.', 'A grey coat.', 'Counts pauses.', 'Hums.', 'plain'])
  })

  it('keeps a dash inside a line, and a number that is the fact', () => {
    expect(linesOf('Born 1990 - in the north\n1990 was the year')).toEqual([
      'Born 1990 - in the north',
      '1990 was the year',
    ])
  })
})
