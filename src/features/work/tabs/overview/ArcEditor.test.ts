import { describe, expect, it } from 'vitest'
import { fieldText } from '@/lib/fieldValue'
import { arcPointsOf } from './ArcEditor'

// v0.146, FR-10 10.1: the emotional arc of a work, read from what it holds.
describe('the emotional arc', () => {
  it('reads its points, intensity kept from 1 to 10, and skips what is no point', () => {
    expect(
      arcPointsOf([
        { section: 'Verse', emotion: 'longing', intensity: 3 },
        { section: 'Chorus', emotion: 'anger', intensity: 14 },
        'stray',
        { section: 'Bridge' },
      ]),
    ).toEqual([
      { section: 'Verse', emotion: 'longing', intensity: 3 },
      { section: 'Chorus', emotion: 'anger', intensity: 10 },
      { section: 'Bridge', emotion: '', intensity: 5 },
    ])
    expect(arcPointsOf('not a list')).toEqual([])
  })

  it('says itself section by section where a value is only read', () => {
    const field = { key: 'emotional_arc', label: 'Arc', type: 'arc' as const }
    expect(
      fieldText(field, [
        { section: 'Verse', emotion: 'longing', intensity: 3 },
        { section: 'Chorus', emotion: 'anger', intensity: 8 },
      ]),
    ).toBe('Verse: longing 3 · Chorus: anger 8')
  })
})
