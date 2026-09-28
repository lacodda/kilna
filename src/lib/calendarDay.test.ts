import { describe, expect, it } from 'vitest'
import { CHIP_GAP, CHIP_HEIGHT, foldDay, linesThatFit } from './calendarDay'

describe('how many chips a day has room for', () => {
  it('counts whole chips with the gaps between them', () => {
    // One chip needs its own height and no gap; every one after it a gap more.
    expect(linesThatFit(CHIP_HEIGHT)).toBe(1)
    expect(linesThatFit(CHIP_HEIGHT * 2 + CHIP_GAP)).toBe(2)
    expect(linesThatFit(CHIP_HEIGHT * 2 + CHIP_GAP - 1)).toBe(1)
    expect(linesThatFit(CHIP_HEIGHT * 4 + CHIP_GAP * 3)).toBe(4)
  })

  it('has room for nothing in a day too short for one chip', () => {
    expect(linesThatFit(CHIP_HEIGHT - 1)).toBe(0)
    expect(linesThatFit(0)).toBe(0)
    // What an element that is not laid out reports, and what a broken read
    // would: neither may turn into a day that draws a negative number of chips.
    expect(linesThatFit(-5)).toBe(0)
    expect(linesThatFit(Number.NaN)).toBe(0)
  })
})

describe('folding a day into its lines', () => {
  it('shows everything when everything fits', () => {
    expect(foldDay(0, 3)).toEqual({ shown: 0, more: 0 })
    expect(foldDay(3, 3)).toEqual({ shown: 3, more: 0 })
  })

  it('gives the count a line of its own', () => {
    // Four chips in three lines: two of them and "+2 more" - not three chips
    // with the count pushed below the bottom of the day.
    expect(foldDay(4, 3)).toEqual({ shown: 2, more: 2 })
    expect(foldDay(9, 4)).toEqual({ shown: 3, more: 6 })
  })

  it('says only the count when there is room for one line', () => {
    expect(foldDay(1, 1)).toEqual({ shown: 1, more: 0 })
    expect(foldDay(2, 1)).toEqual({ shown: 0, more: 2 })
  })

  it('never shows a negative number of chips', () => {
    expect(foldDay(2, 0)).toEqual({ shown: 0, more: 2 })
  })

  it('accounts for every chip, shown or counted', () => {
    for (let lines = 0; lines <= 6; lines++) {
      for (let count = 0; count <= 12; count++) {
        const { shown, more } = foldDay(count, lines)
        expect(shown + more).toBe(count)
        // What is drawn fits: the chips shown, plus the count when there is one.
        if (lines > 0) expect(shown + (more > 0 ? 1 : 0)).toBeLessThanOrEqual(lines)
      }
    }
  })
})
