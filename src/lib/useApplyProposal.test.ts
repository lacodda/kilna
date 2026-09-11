import { describe, expect, it } from 'vitest'
import { writesOf } from '@/lib/useApplyProposal'

describe('writesOf', () => {
  it('counts one operation for a single version, score or note', () => {
    expect(writesOf({ message_id: 'm', at: 't', versions: ['v1'] })).toBe(1)
    expect(writesOf({ message_id: 'm', at: 't', score: 's1' })).toBe(1)
    expect(writesOf({ message_id: 'm', at: 't', notes: ['n1'] })).toBe(1)
  })

  it('counts a package as every row it wrote, so no single undo is offered for it', () => {
    expect(
      writesOf({
        message_id: 'm',
        at: 't',
        work_id: 'w',
        created_work: true,
        versions: ['v1', 'v2'],
        score: 's1',
        notes: ['n1'],
        fields: ['premise'],
      }),
    ).toBe(5)
    expect(writesOf({ message_id: 'm', at: 't', work_id: 'w', fields: ['tagline'] })).toBe(1)
    expect(writesOf({ message_id: 'm', at: 't', work_id: 'w', fields: ['tagline'], notes: ['n'] })).toBe(2)
  })
})
