import { describe, expect, it } from 'vitest'
import { findRepeats } from '@/lib/repeats'

describe('findRepeats', () => {
  it('counts a word across its forms', () => {
    const { groups, marks } = findRepeats('Лестница вела вверх.\nПо лестнице шёл дождь.\nЛестницей к небу.')
    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ word: 'Лестница', count: 3 })
    expect(marks.map((mark) => mark.group)).toEqual([0, 0, 0])
  })

  it('marks the exact places, in text order', () => {
    const text = 'rain on the roof, rain in the street'
    const { marks } = findRepeats(text)
    expect(marks.map((mark) => text.slice(mark.start, mark.end))).toEqual(['rain', 'rain'])
    expect(marks[0]!.start).toBe(0)
    expect(marks[1]!.start).toBe(text.lastIndexOf('rain'))
  })

  it('leaves function words alone', () => {
    const { groups } = findRepeats('и снова, и снова, и снова — the end and the end')
    expect(groups.map((group) => group.word)).toEqual(['снова', 'end'])
  })

  it('leaves section labels alone', () => {
    const { groups } = findRepeats('[Verse 1]\nverse of the night\n[Verse 2]\nanother night')
    expect(groups.map((group) => group.word)).toEqual(['night'])
  })

  it('orders groups by count, most repeated first', () => {
    const { groups } = findRepeats('дом дорога дом дорога дом')
    expect(groups.map((group) => [group.word, group.count])).toEqual([
      ['дом', 3],
      ['дорога', 2],
    ])
  })

  it('finds nothing in a text that repeats nothing', () => {
    expect(findRepeats('каждое слово здесь единственное')).toEqual({ groups: [], marks: [] })
    expect(findRepeats('')).toEqual({ groups: [], marks: [] })
  })
})
