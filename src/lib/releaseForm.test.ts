import { describe, expect, it } from 'vitest'
import type { Release } from '@/lib/api/types'
import { changesOf, draftOf, rebase, sameDraft } from '@/lib/releaseForm'

function release(fields: Partial<Release> = {}): Release {
  return {
    id: 'r1',
    work_id: 'w1',
    kind: 'youtube',
    status: 'planned',
    title: null,
    scheduled_at: '2026-10-02',
    released_at: null,
    url: null,
    slot_pinned_at: null,
    scheduled_time: null,
    time_zone: null,
    meta: {},
    created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-01T10:00:00Z',
    ...fields,
  }
}

describe('draftOf', () => {
  it('reads an empty date and an empty link as empty boxes', () => {
    expect(draftOf(release({ scheduled_at: null }))).toEqual({
      kind: 'youtube',
      date: '',
      url: '',
      pinned: false,
    })
  })

  it('reads a pinned date as kept', () => {
    expect(draftOf(release({ slot_pinned_at: '2026-09-02T10:00:00Z' })).pinned).toBe(true)
  })
})

describe('changesOf', () => {
  // A form left the way it was found must write nothing: no journal line, and
  // no undo offered for a change that did not happen.
  it('is null for a draft nobody changed', () => {
    const r = release({ url: 'https://example.com/v' })
    expect(changesOf(r, draftOf(r))).toBeNull()
  })

  it('sends only the field that changed', () => {
    const r = release()
    expect(changesOf(r, { ...draftOf(r), date: '2026-10-05' })).toEqual({
      patch: { scheduled_at: '2026-10-05' },
      pin: null,
    })
  })

  it('reads an emptied date as a return to the queue', () => {
    const r = release()
    expect(changesOf(r, { ...draftOf(r), date: '' })?.patch).toEqual({ scheduled_at: null })
  })

  it('trims a pasted link, and counts spaces alone as no link', () => {
    const r = release({ url: 'https://example.com/v' })
    expect(changesOf(r, { ...draftOf(r), url: '  https://example.com/v  ' })).toBeNull()
    expect(changesOf(r, { ...draftOf(r), url: '   ' })?.patch).toEqual({ url: null })
  })

  it('does not clear the kind when the profile offers none', () => {
    const r = release()
    expect(changesOf(r, { ...draftOf(r), kind: '' })).toBeNull()
  })

  it('asks for the pin on its own when only the pin moved', () => {
    const r = release()
    expect(changesOf(r, { ...draftOf(r), pinned: true })).toEqual({ patch: null, pin: true })
  })

  // A pin describes a date that was decided; the backend drops it with the
  // date, and refuses one on a release that has none.
  it('asks for no pin when the date is being cleared', () => {
    const r = release({ slot_pinned_at: '2026-09-02T10:00:00Z' })
    expect(changesOf(r, { ...draftOf(r), date: '', pinned: false })).toEqual({
      patch: { scheduled_at: null },
      pin: null,
    })
    expect(
      changesOf(release({ scheduled_at: null }), { ...draftOf(r), date: '', pinned: true }),
    ).toBeNull()
  })

  it('pins a date given in the same edit', () => {
    const r = release({ scheduled_at: null })
    expect(changesOf(r, { ...draftOf(r), date: '2026-10-05', pinned: true })).toEqual({
      patch: { scheduled_at: '2026-10-05' },
      pin: true,
    })
  })
})

describe('sameDraft', () => {
  it('tells a moved date from the release it was drawn from', () => {
    const r = release()
    expect(sameDraft(draftOf(r), draftOf(r))).toBe(true)
    expect(sameDraft(draftOf(r), { ...draftOf(r), date: '2026-10-03' })).toBe(false)
  })
})

describe('rebase', () => {
  const from = draftOf(release())

  // The date picked a moment ago comes back saved while a link is still being
  // typed: the date follows the store, the link stays as typed.
  it('keeps what is being typed in a field the store did not move', () => {
    const typing = { ...from, url: 'https://exa' }
    const to = { ...from, date: '2026-10-05' }
    expect(rebase(typing, from, to)).toEqual({ ...from, date: '2026-10-05', url: 'https://exa' })
  })

  it('takes the stored value of a field the store moved', () => {
    const typing = { ...from, date: '2026-10-09' }
    const to = { ...from, date: '2026-10-05' }
    expect(rebase(typing, from, to).date).toBe('2026-10-05')
  })
})
