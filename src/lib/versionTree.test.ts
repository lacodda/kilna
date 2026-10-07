import { describe, expect, it } from 'vitest'
import type { Lineage } from '@/lib/history'
import { graphOf } from '@/lib/versionTree'

const v = (revision: number, parent: number | null = null): Lineage => ({
  id: `v${revision}`,
  revision,
  parent_version_id: parent === null ? null : `v${parent}`,
})

describe('graphOf', () => {
  it('draws nothing for a history with no recorded lineage', () => {
    // Dots alone would say "unknown" once a row.
    expect(graphOf([v(3), v(2), v(1)])).toBeNull()
    expect(graphOf([])).toBeNull()
  })

  it('draws a line as one column, each version joined to the one below', () => {
    const graph = graphOf([v(3, 2), v(2, 1), v(1)])!
    expect(graph.columns).toBe(1)
    expect(graph.rows).toEqual([
      { column: 0, up: false, down: true, through: [], joins: [] },
      { column: 0, up: true, down: true, through: [], joins: [] },
      { column: 0, up: true, down: false, through: [], joins: [] },
    ])
  })

  it('opens a second column where two versions were written from the same one', () => {
    // v4 and v3 were both started from v2: the newer keeps the first
    // column, the older takes the next, and the two meet at v2.
    const graph = graphOf([v(4, 2), v(3, 2), v(2, 1), v(1)])!
    expect(graph.columns).toBe(2)
    expect(graph.rows[0]).toMatchObject({ column: 0, down: true })
    expect(graph.rows[1]).toMatchObject({ column: 1, down: true, through: [0] })
    expect(graph.rows[2]).toMatchObject({ column: 0, up: true, joins: [1], through: [] })
    expect(graph.rows[3]).toMatchObject({ column: 0, up: true, down: false })
  })

  it('carries a long branch past the rows between it and its parent', () => {
    // v5 was written from v1, four revisions back.
    const graph = graphOf([v(5, 1), v(4, 3), v(3, 2), v(2, 1), v(1)])!
    expect(graph.rows.map((row) => row.column)).toEqual([0, 1, 1, 1, 0])
    expect(graph.rows[2]!.through).toEqual([0])
    expect(graph.rows[4]!.joins).toEqual([1])
  })

  it('draws a version with no parent as a dot with nothing below it', () => {
    // An alternative started from nothing beside a line of revisions.
    const graph = graphOf([v(3), v(2, 1), v(1)])!
    expect(graph.rows[0]).toEqual({ column: 0, up: false, down: false, through: [], joins: [] })
    expect(graph.rows[1]).toMatchObject({ column: 0, down: true })
    expect(graph.columns).toBe(1)
  })

  it('takes no line from a parent the list does not hold', () => {
    expect(graphOf([v(3, 9), v(2), v(1)])).toBeNull()
  })

  it('frees a column once its branch has met its parent', () => {
    // Two forks one after the other reuse the second column rather than
    // widening the gutter to three.
    const graph = graphOf([v(6, 4), v(5, 4), v(4, 2), v(3, 2), v(2, 1), v(1)])!
    expect(graph.columns).toBe(2)
    expect(graph.rows.map((row) => row.column)).toEqual([0, 1, 0, 1, 0, 0])
  })
})
