import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { Applied, MonthCheck, MonthProposal, QuotaReport } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { ProposedMonth } from '@/features/assistant/ProposedMonth'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'

/*
 * A month's drafts proposed in one pass (v0.153, FR-20 20.4): the card says
 * what the space's quotas find in the month as it would stand, before
 * anything is written, and one press applies all of it.
 */

const MESSAGE = 'm-month'

const REPORT: QuotaReport = {
  rules: 2,
  works: 2,
  findings: [
    {
      rule_id: 'f-rule',
      label: 'Suno prompt length',
      field: 'suno_prompt',
      metric: 'char_len',
      severity: 'error',
      scope: 'item',
      work_id: IDS.song,
      title: 'Paper Lanterns',
      measured: 12,
      value: null,
      expected: '>= 400',
    },
  ],
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    check_month_proposal: () =>
      ({
        quotas: REPORT,
        bank: [
          {
            work_id: IDS.song,
            title: 'Paper Lanterns',
            item: 'Gnawa',
            state: 'locked',
            until: null,
          },
        ],
        register: [{ work_id: IDS.draft, title: 'Harbour at dusk', word: 'dusk', scope: 'title' }],
      }) satisfies MonthCheck,
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, versions: [] }) as unknown as Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a month proposed in one pass', () => {
  it('reads the quotas over the month as it would stand and applies it whole', async () => {
    const proposal: MonthProposal = {
      kind: 'month',
      month: '2026-10',
      items: [
        { work_id: IDS.song, work_title: 'Paper Lanterns', fields: { mood: 'calm' } },
        { work_id: IDS.draft, work_title: 'Harbour Lights', title: 'Harbour at dusk', fields: {} },
      ],
    } as unknown as MonthProposal
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <ProposedMonth messageId={MESSAGE} proposal={proposal} applied={null} dismissed={false} />
        </MemoryRouter>
      </Providers>,
    )

    const apply = await screen.findByRole('button', { name: 'Apply all 2' })
    const card = apply.closest('section')!
    expect(within(card).getByText('A month: 2 works in one pass')).toBeInTheDocument()
    await waitFor(() => expect(backend.argsOf('check_month_proposal')).toHaveLength(1))
    expect(await within(card).findByText('Suno prompt length')).toBeInTheDocument()
    expect(within(card).getByText('12, wants >= 400')).toBeInTheDocument()
    expect(within(card).getByText('Gnawa')).toBeInTheDocument()
    expect(within(card).getByText('dusk')).toBeInTheDocument()

    fireEvent.click(apply)
    await waitFor(() => expect(backend.argsOf('apply_proposal')).toHaveLength(1))
    expect(backend.argsOf('apply_proposal')[0]).toMatchObject({ messageId: MESSAGE })
  })
})
