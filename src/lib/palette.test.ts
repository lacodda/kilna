import { describe, expect, it } from 'vitest'
import type { PromptTemplate } from '@/lib/api'
import { matching, reading } from '@/lib/palette'

const action = (key: string, label: string): PromptTemplate => ({
  key,
  label,
  template: `do ${key}`,
})

const ACTIONS: PromptTemplate[] = [
  action('critique', 'Critique the lyrics'),
  action('title', 'Suggest a title'),
  action('style', 'Describe the style'),
  action('score', 'Score the work'),
]

const labels = (found: PromptTemplate[]) => found.map((one) => one.label)

describe('reading', () => {
  it('opens on a draft that is only a slash command', () => {
    const palette = reading('/cri', ACTIONS)

    expect(palette).not.toBeNull()
    expect(labels(palette?.matches ?? [])).toEqual(['Critique the lyrics'])
  })

  it('offers everything on a bare slash', () => {
    expect(labels(reading('/', ACTIONS)?.matches ?? [])).toHaveLength(ACTIONS.length)
  })

  it('ignores a slash inside a sentence', () => {
    // Paths, dates and "and/or" all carry one; stealing the keyboard from
    // someone writing about C:/Media would be worse than having no palette.
    expect(reading('the file at C:/Media', ACTIONS)).toBeNull()
    expect(reading('cut it and/or rewrite', ACTIONS)).toBeNull()
  })

  it('closes once the draft grows past one line', () => {
    expect(reading('/critique\nand be hard on it', ACTIONS)).toBeNull()
  })

  it('is not open on an empty draft', () => {
    expect(reading('', ACTIONS)).toBeNull()
  })
})

describe('matching', () => {
  it('prefers a name that starts the way you typed over a later word', () => {
    const found = matching('sty', [
      action('critique', 'Critique the lyrics'),
      action('lyrics', 'Style the lyrics'),
      action('style', 'Describe the style'),
    ])

    expect(labels(found)).toEqual(['Style the lyrics', 'Describe the style'])
  })

  it('never matches inside a word', () => {
    // "Describe" contains "cri". Offering it for a query that plainly means
    // "critique" is worse than offering nothing.
    const found = matching('cri', [
      action('critique', 'Critique the lyrics'),
      action('style', 'Describe the style'),
    ])

    expect(labels(found)).toEqual(['Critique the lyrics'])
  })

  it('matches a word inside the label', () => {
    expect(labels(matching('lyr', ACTIONS))).toEqual(['Critique the lyrics'])
  })

  it('matches the key a profile document uses', () => {
    expect(labels(matching('critique', ACTIONS))).toEqual(['Critique the lyrics'])
  })

  it('ignores case on both sides', () => {
    expect(labels(matching('CRI', ACTIONS))).toEqual(['Critique the lyrics'])
  })

  it('keeps the profile order among equally good matches', () => {
    const found = matching('the', ACTIONS)

    expect(labels(found)).toEqual([
      'Critique the lyrics',
      'Describe the style',
      'Score the work',
    ])
  })

  it('returns nothing when nothing matches', () => {
    expect(matching('zzz', ACTIONS)).toEqual([])
  })

  it('returns everything for a blank query', () => {
    expect(matching('   ', ACTIONS)).toHaveLength(ACTIONS.length)
  })
})
