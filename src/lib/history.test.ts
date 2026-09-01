import { describe, expect, it } from 'vitest'
import { neighbour, predecessor } from '@/lib/history'

/** Newest first, the order the panel lists them in. */
const list = [{ id: 'v3' }, { id: 'v2' }, { id: 'v1' }]

describe('neighbour', () => {
  it('steps toward the older end', () => {
    expect(neighbour(list, 'v3', 1)?.id).toBe('v2')
  })

  it('steps toward the newer end', () => {
    expect(neighbour(list, 'v2', -1)?.id).toBe('v3')
  })

  it('stops at the oldest rather than wrapping to the newest', () => {
    expect(neighbour(list, 'v1', 1)).toBeNull()
  })

  it('stops at the newest rather than wrapping to the oldest', () => {
    expect(neighbour(list, 'v3', -1)).toBeNull()
  })

  it('has nowhere to go from nothing open', () => {
    expect(neighbour(list, null, 1)).toBeNull()
  })

  it('has nowhere to go from a version no longer in the list', () => {
    // The open one can vanish underneath: deleted, or the role switched.
    expect(neighbour(list, 'gone', -1)).toBeNull()
  })

  it('has nowhere to go in an empty history', () => {
    expect(neighbour([], 'v1', 1)).toBeNull()
  })
})

describe('predecessor', () => {
  it('is the revision directly below the open one', () => {
    expect(predecessor(list, 'v3')?.id).toBe('v2')
  })

  it('is nothing for the oldest revision, which follows nothing', () => {
    expect(predecessor(list, 'v1')).toBeNull()
  })

  it('is nothing for a lone revision', () => {
    expect(predecessor([{ id: 'only' }], 'only')).toBeNull()
  })
})
