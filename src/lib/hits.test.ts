import { describe, expect, it } from 'vitest'
import type { Hit } from '@/lib/api'
import { hrefOfHit } from '@/lib/hits'

const hit = (over: Partial<Hit>): Hit => ({
  kind: 'work',
  entity_id: 'e1',
  work_id: 'w1',
  work_title: 'Harbour lights',
  title: 'Harbour lights',
  detail: '',
  rank: 0,
  ...over,
})

describe('hrefOfHit', () => {
  it('opens a work on its overview and a version on the versions tab', () => {
    expect(hrefOfHit(hit({ kind: 'work' }))).toBe('/works/w1/overview')
    expect(hrefOfHit(hit({ kind: 'version', entity_id: 'v1' }))).toBe('/works/w1/versions')
  })

  it('opens a note on the notes screen by its own id, with or without a work', () => {
    expect(hrefOfHit(hit({ kind: 'note', entity_id: 'n1' }))).toBe('/notes/n1')
    expect(hrefOfHit(hit({ kind: 'note', entity_id: 'n2', work_id: null }))).toBe('/notes/n2')
  })

  it('opens a comment on the comments screen by its own id', () => {
    expect(hrefOfHit(hit({ kind: 'comment', entity_id: 'c1' }))).toBe('/comments/c1')
  })
})
