import { describe, expect, it } from 'vitest'
import { shortKind } from './kind'

describe('shortKind', () => {
  it('takes the initials of a two-word kind', () => {
    expect(shortKind('Video clip')).toBe('VC')
  })

  it('takes two letters of a single-word kind', () => {
    expect(shortKind('Short')).toBe('SH')
  })

  // The property that matters on the chip: whatever the profile calls a kind,
  // the label never grows past two characters, because the title's width is
  // what is left over.
  it('never returns more than two characters', () => {
    const labels = [
      'Video clip',
      'Short',
      'Audio release',
      'Аудио-релиз',
      'Beta readers',
      'Submission',
      'Newsletter',
      'Feed episode',
      'A very long name for a release kind',
      'エピソード',
    ]
    for (const label of labels) {
      expect(shortKind(label).length).toBeLessThanOrEqual(2)
    }
  })

  it('splits on a hyphen, a dash and a slash, not only on a space', () => {
    expect(shortKind('Аудио-релиз')).toBe('АР')
    expect(shortKind('Audio–release')).toBe('AR')
    expect(shortKind('Audio/release')).toBe('AR')
  })

  // A label that is only separators, or empty, must not throw: profiles are
  // edited by hand, and a half-typed label reaches the calendar.
  it('survives a label with nothing in it', () => {
    expect(shortKind('')).toBe('')
    expect(shortKind('   ')).toBe('')
    expect(shortKind('-')).toBe('')
  })

  it('ignores repeated separators rather than reading them as words', () => {
    expect(shortKind('Video   clip')).toBe('VC')
    expect(shortKind('  Video clip')).toBe('VC')
  })

  it('keeps a one-letter word as it is', () => {
    expect(shortKind('A')).toBe('A')
  })
})
