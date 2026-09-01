import { describe, expect, it } from 'vitest'
import type { ScheduledRelease } from '@/lib/api'
import type { Ghost } from '@/lib/layout'
import { countByKind, filterByKind, filterGhosts } from './calendarFilter'

/** Only the fields the filter looks at; the rest of the row is irrelevant here. */
const slot = (id: string, kind: string) => ({ id, kind }) as ScheduledRelease

const SLOTS = [slot('a', 'clip'), slot('b', 'short'), slot('c', 'clip'), slot('d', 'audio')]

describe('filterByKind', () => {
  it('shows everything when no kind is chosen', () => {
    expect(filterByKind(SLOTS, null)).toEqual(SLOTS)
  })

  it('keeps only the chosen kind', () => {
    expect(filterByKind(SLOTS, 'clip').map((s) => s.id)).toEqual(['a', 'c'])
  })

  it('keeps the order the calendar sorted them into', () => {
    // The backend orders by date; a filter that reshuffled would scatter a day.
    expect(filterByKind(SLOTS, null).map((s) => s.id)).toEqual(['a', 'b', 'c', 'd'])
  })

  // The case that separates "all" from "none": a filter naming a kind that no
  // longer exists must hide everything rather than quietly show everything.
  it('shows nothing for a kind no slot holds', () => {
    expect(filterByKind(SLOTS, 'vinyl')).toEqual([])
  })

  it('does not modify the array it was given', () => {
    const original = [...SLOTS]
    filterByKind(SLOTS, 'clip')
    expect(SLOTS).toEqual(original)
  })
})

describe('countByKind', () => {
  it('counts each kind over everything, not over what is shown', () => {
    const counts = countByKind(SLOTS)
    expect(counts.get('clip')).toBe(2)
    expect(counts.get('short')).toBe(1)
    expect(counts.get('audio')).toBe(1)
  })

  it('says nothing about a kind with no slots', () => {
    expect(countByKind(SLOTS).get('vinyl')).toBeUndefined()
  })

  it('counts an empty calendar as empty', () => {
    expect(countByKind([]).size).toBe(0)
  })
})

describe('filterGhosts', () => {
  const ghost = (releaseId: string, kind: string): Ghost => ({
    releaseId,
    workId: `w-${releaseId}`,
    title: releaseId,
    kind,
  })

  const GHOSTS = new Map<string, Ghost[]>([
    ['2026-09-02', [ghost('a', 'clip'), ghost('b', 'short')]],
    ['2026-09-05', [ghost('c', 'short')]],
  ])

  it('leaves the plan alone when no kind is chosen', () => {
    expect(filterGhosts(GHOSTS, null)).toBe(GHOSTS)
  })

  // The bug this exists for: a filtered month drawing a plan it is not showing.
  it('keeps only the ghosts of the chosen kind', () => {
    const kept = filterGhosts(GHOSTS, 'clip')
    expect([...kept.keys()]).toEqual(['2026-09-02'])
    expect(kept.get('2026-09-02')?.map((g) => g.releaseId)).toEqual(['a'])
  })

  // An empty array in the map is not the same as no entry: the grid reads the
  // map per day, and an empty stack renders as a gap in the plan.
  it('drops a day left with nothing rather than emptying it', () => {
    const kept = filterGhosts(GHOSTS, 'clip')
    expect(kept.has('2026-09-05')).toBe(false)
  })

  it('does not modify the map it was given', () => {
    filterGhosts(GHOSTS, 'clip')
    expect(GHOSTS.get('2026-09-02')).toHaveLength(2)
    expect(GHOSTS.get('2026-09-05')).toHaveLength(1)
  })

  it('keeps nothing for a kind the plan does not hold', () => {
    expect(filterGhosts(GHOSTS, 'audio').size).toBe(0)
  })
})
