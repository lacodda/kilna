import { describe, expect, it } from 'vitest'
import type { VersionRole, VersionSummary } from '@/lib/api/types'
import { commentaryOn, isCommentary, subjectOf } from '@/lib/commentary'

const roles: VersionRole[] = [
  { key: 'lyrics', label: 'Lyrics' },
  { key: 'style', label: 'Style' },
  { key: 'review', label: 'Review', comments_on: 'lyrics' },
  { key: 'critique', label: 'Critique', comments_on: 'lyrics' },
]

function version(
  fields: Partial<VersionSummary> & Pick<VersionSummary, 'id' | 'role'>,
): VersionSummary {
  return {
    work_id: 'w',
    revision: 1,
    label: null,
    length: 10,
    parent_version_id: null,
    created_at: '2026-09-01T00:00:00Z',
    is_current: false,
    about_version_id: null,
    ...fields,
  }
}

const lyrics1 = version({ id: 'l1', role: 'lyrics', revision: 1 })
const lyrics2 = version({ id: 'l2', role: 'lyrics', revision: 2 })
const style1 = version({ id: 's1', role: 'style', revision: 1 })
// Imported: says nothing about what it read, so it goes by its number.
const review1 = version({ id: 'r1', role: 'review', revision: 1 })
// Written by an action started on revision 2 while the review lane was
// still at its first revision: the number would pair it with the wrong text.
const critique = version({ id: 'c1', role: 'critique', revision: 1, about_version_id: 'l2' })

const all = [lyrics2, lyrics1, style1, review1, critique]

describe('isCommentary', () => {
  it('is true for a role that names the one it discusses', () => {
    expect(isCommentary(roles, 'review')).toBe(true)
  })

  it('is false for a role that stands alone', () => {
    expect(isCommentary(roles, 'lyrics')).toBe(false)
    expect(isCommentary(roles, 'style')).toBe(false)
  })

  it('is false for a role the profile does not name', () => {
    expect(isCommentary(roles, 'plot')).toBe(false)
  })
})

describe('commentaryOn', () => {
  it('pairs commentary that names its version by that', () => {
    expect(commentaryOn(all, roles, lyrics2).map((v) => v.id)).toEqual(['c1'])
  })

  it('pairs commentary that names nothing by revision number', () => {
    expect(commentaryOn(all, roles, lyrics1).map((v) => v.id)).toEqual(['r1'])
  })

  it('does not pair a named commentary by its number as well', () => {
    // The critique is revision 1 of its role, and it is about revision 2.
    expect(commentaryOn(all, roles, lyrics1).map((v) => v.id)).not.toContain('c1')
  })

  it('finds nothing written about a role nobody comments on', () => {
    expect(commentaryOn(all, roles, style1)).toEqual([])
  })

  it('finds nothing with nothing open', () => {
    expect(commentaryOn(all, roles, null)).toEqual([])
  })
})

describe('subjectOf', () => {
  it('follows a named version', () => {
    expect(subjectOf(all, roles, critique)?.id).toBe('l2')
  })

  it('goes by revision number when nothing is named', () => {
    expect(subjectOf(all, roles, review1)?.id).toBe('l1')
  })

  it('has no subject for a version that is not commentary', () => {
    expect(subjectOf(all, roles, lyrics2)).toBeNull()
  })

  it('has no subject once the text it read is gone', () => {
    expect(subjectOf([review1, critique], roles, critique)).toBeNull()
    expect(subjectOf([review1, critique], roles, review1)).toBeNull()
  })

  it('has no subject for nothing', () => {
    expect(subjectOf(all, roles, null)).toBeNull()
  })
})
