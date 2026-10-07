import { describe, expect, it } from 'vitest'
import { childrenIn, neighbour, parentIn, predecessor, type Lineage } from '@/lib/history'

/** Newest first, the order the panel lists them in. */
const list = [{ id: 'v3' }, { id: 'v2' }, { id: 'v1' }]

const v = (revision: number, parent: number | null = null): Lineage => ({
  id: `v${revision}`,
  revision,
  parent_version_id: parent === null ? null : `v${parent}`,
})

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
  it('is the version it was written from, though another is numbered between', () => {
    // v4 was started from v2; v3 is a sibling, not where v4 came from.
    const tree = [v(4, 2), v(3, 2), v(2, 1), v(1)]
    expect(predecessor(tree, 'v4')?.id).toBe('v2')
  })

  it('is the revision directly below when no parent was recorded', () => {
    expect(predecessor([v(3), v(2), v(1)], 'v3')?.id).toBe('v2')
  })

  it('falls back to the revision below when the parent is gone from the role', () => {
    // A pruned parent reads as no parent at all (ON DELETE SET NULL), but a
    // list fetched a moment before the prune can still name it.
    expect(predecessor([v(3, 9), v(2), v(1)], 'v3')?.id).toBe('v2')
  })

  it('reads by revision, whatever order the list is in', () => {
    expect(predecessor([v(1), v(3), v(2)], 'v3')?.id).toBe('v2')
  })

  it('is nothing for the oldest revision, which follows nothing', () => {
    expect(predecessor([v(2), v(1)], 'v1')).toBeNull()
  })

  it('is nothing for a lone revision, or for a version not in the list', () => {
    expect(predecessor([v(1)], 'v1')).toBeNull()
    expect(predecessor([v(1)], 'gone')).toBeNull()
  })
})

describe('parentIn and childrenIn', () => {
  const tree = [v(5, 3), v(4, 2), v(3, 2), v(2, 1), v(1)]

  it('go up to the version written from and down to those written from it', () => {
    expect(parentIn(tree, 'v4')?.id).toBe('v2')
    expect(childrenIn(tree, 'v2').map((x) => x.id)).toEqual(['v4', 'v3'])
  })

  it('have nowhere to go at a root or a leaf', () => {
    expect(parentIn(tree, 'v1')).toBeNull()
    expect(childrenIn(tree, 'v5')).toEqual([])
    expect(childrenIn(tree, null)).toEqual([])
  })
})
