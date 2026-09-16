import { describe, expect, it } from 'vitest'
import type { ReleaseFieldValue } from '@/lib/api'
import { batchable, fillable, over, written } from '@/lib/releaseFields'

function field(over: Partial<ReleaseFieldValue> = {}): ReleaseFieldValue {
  return {
    key: 'title',
    label: 'Title',
    type: 'line',
    value: '',
    has_template: false,
    ...over,
  }
}

describe('written', () => {
  it('counts a field with something in it', () => {
    const counted = written([
      field({ key: 'title', value: 'Harbour lights' }),
      field({ key: 'description', value: '' }),
    ])

    expect(counted).toEqual({ written: 1, total: 2, complete: false })
  })

  // A release that calls itself ready on three spaces is a release someone
  // ships with an empty description.
  it('does not count whitespace as written', () => {
    expect(written([field({ value: '   \n ' })]).written).toBe(0)
  })

  it('calls a kind that asks for nothing complete', () => {
    expect(written([])).toEqual({ written: 0, total: 0, complete: true })
  })

  it('is complete when every box has something in it', () => {
    const counted = written([
      field({ key: 'title', value: 'a' }),
      field({ key: 'tags', value: 'b' }),
    ])

    expect(counted.complete).toBe(true)
  })
})

describe('fillable', () => {
  // A button that does nothing teaches the person that it does nothing.
  it('is false when every field is typed by hand', () => {
    expect(fillable([field({ has_template: false })])).toBe(false)
  })

  it('is true as soon as one field has a template', () => {
    expect(fillable([field({ has_template: false }), field({ has_template: true })])).toBe(true)
  })

  it('is false for a kind that asks for nothing', () => {
    expect(fillable([])).toBe(false)
  })
})

describe('over', () => {
  it('says nothing when nobody is counting', () => {
    expect(over(field({ limit: null }), 'any length at all')).toBeNull()
  })

  it('says how far past the limit the text runs', () => {
    expect(over(field({ limit: 5 }), 'seven!!')).toBe(2)
  })

  // Exactly at the limit is within it: a title of exactly 100 characters is a
  // title the platform accepts, and colouring it red would be a lie.
  it('treats the limit itself as within', () => {
    expect(over(field({ limit: 5 }), 'fives')).toBeNull()
  })

  it('counts characters, not words', () => {
    expect(over(field({ limit: 3 }), 'a b c d')).toBe(4)
  })
})

describe('batchable', () => {
  const planned = { id: 'a', status: 'planned' }
  const out = { id: 'b', status: 'released' }

  // A released release's metadata is the record of what it went out under.
  // Rewriting it from today's lyrics would quietly rewrite history.
  it('leaves what has already gone out alone', () => {
    expect(batchable([planned, out])).toEqual(['a'])
  })

  it('takes everything still planned, written or not', () => {
    expect(batchable([planned, { id: 'c', status: 'planned' }])).toEqual(['a', 'c'])
  })

  it('is empty when nothing is planned', () => {
    expect(batchable([out])).toEqual([])
  })
})
