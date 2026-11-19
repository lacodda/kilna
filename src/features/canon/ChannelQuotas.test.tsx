import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Fact, NewFact, QuotaReport } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The channel's quotas and its bank (v0.151, FR-20 20.2): a rule is a
 * predicate written in boxes and checked from where it is written; an item
 * of a bank says where it stands today and is narrowed by it.
 */

let backend: Backend

const fact = (id: string, section: string, body: string, data: Fact['data']): Fact => ({
  id,
  profile_id: IDS.profile,
  note_id: 'n-channel',
  section,
  body,
  layer: 'public',
  status: 'canon',
  retired_reason: null,
  source: null,
  when: null,
  scope_work_id: null,
  data,
  position: 1,
  created_at: NOW,
  updated_at: NOW,
})

const REPORT: QuotaReport = {
  rules: 1,
  works: 2,
  findings: [
    {
      rule_id: 'f-rule',
      label: 'Short titles',
      field: 'title',
      metric: 'char_len',
      severity: 'error',
      scope: 'item',
      work_id: IDS.song,
      title: 'Paper Lanterns',
      measured: 14,
      value: null,
      expected: '<= 10',
    },
  ],
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.notes.push({
    ...workspace.notes.find((note) => note.id === IDS.character)!,
    id: 'n-channel',
    work_id: null,
    kind: 'channel',
    title: 'The channel',
    body: '',
  })
  workspace.facts.push(
    fact('f-rule', 'quotas', 'Short titles', {
      field: 'title',
      metric: 'char_len',
      op: 'max',
      value: 10,
      scope: 'item',
      severity: 'error',
      active: true,
    }),
    fact('f-free', 'bank', 'Fado', { group: 'Iberia' }),
    fact('f-resting', 'bank', 'Mande', {
      group: 'West Africa',
      cooldown_months: 6,
      last_used_at: NOW.slice(0, 10),
      used_in: [{ work_id: IDS.song, title: 'Paper Lanterns', date: NOW.slice(0, 10) }],
    }),
    fact('f-locked', 'bank', 'Gnawa', { locked: true }),
  )
  backend = mockBackend({
    ...answersFor(workspace),
    add_fact: ({ fact: asked }) => ({ ...(asked as NewFact), id: 'f-new' }),
    check_quotas: () => REPORT,
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('the quotas', () => {
  it('reads a rule as what it measures, and checks the space from the section', async () => {
    const { client } = renderApp('/canon/n-channel')
    await settled(client)
    const main = await screen.findByRole('main')
    const quotas = await within(main).findByRole('region', { name: 'Quotas' })
    expect(within(quotas).getByText('`title` · characters <= 10 · each work')).toBeInTheDocument()

    fireEvent.click(within(quotas).getByRole('button', { name: en.quota.checkSpace }))
    await settled(client)
    const dialog = await screen.findByRole('dialog', { name: en.quota.titleSpace })
    expect(backend.argsOf('check_quotas')).toEqual([{ subject: {} }])
    expect(within(dialog).getByRole('button', { name: 'Paper Lanterns' })).toBeInTheDocument()
    expect(within(dialog).getByText('14, wants <= 10')).toBeInTheDocument()
  })

  it('writes a rule from its boxes', async () => {
    const { client } = renderApp('/canon/n-channel')
    await settled(client)
    const main = await screen.findByRole('main')
    const quotas = await within(main).findByRole('region', { name: 'Quotas' })

    fireEvent.click(within(quotas).getByRole('button', { name: en.canon.add.quotas }))
    fireEvent.change(within(quotas).getByRole('textbox', { name: en.canon.factWords }), {
      target: { value: 'No neon' },
    })
    fireEvent.change(within(quotas).getByRole('combobox', { name: en.canon.quota.field }), {
      target: { value: 'lyrics' },
    })
    fireEvent.change(within(quotas).getByRole('textbox', { name: en.canon.quota.filterField }), {
      target: { value: 'kind' },
    })
    fireEvent.change(within(quotas).getByRole('textbox', { name: en.canon.quota.filterEquals }), {
      target: { value: 'song' },
    })
    const value = within(quotas).getByRole('textbox', { name: en.canon.quota.value })
    fireEvent.change(value, { target: { value: '300' } })
    fireEvent.blur(value)
    await act(async () => {
      fireEvent.keyDown(within(quotas).getByRole('textbox', { name: en.canon.factWords }), {
        key: 'Enter',
        ctrlKey: true,
      })
    })
    await settled(client)

    const [asked] = backend.argsOf('add_fact') as { fact: NewFact }[]
    expect(asked?.fact).toMatchObject({
      section: 'quotas',
      body: 'No neon',
      data: {
        field: 'lyrics',
        metric: 'char_len',
        op: 'max',
        value: 300,
        filter: { field: 'kind', equals: ['song'] },
      },
    })
  })
})

describe('the bank', () => {
  it('says where each item stands and narrows to one standing', async () => {
    const { client } = renderApp('/canon/n-channel')
    await settled(client)
    const main = await screen.findByRole('main')
    const bank = await within(main).findByRole('region', { name: 'Bank' })
    expect(within(bank).getByText('Fado')).toBeInTheDocument()
    expect(within(bank).getByText(/^resting until /)).toBeInTheDocument()

    fireEvent.click(within(bank).getByRole('radio', { name: en.canon.bank.filter.cooldown }))
    expect(within(bank).getByText('Mande')).toBeInTheDocument()
    expect(within(bank).queryByText('Fado')).toBeNull()
    expect(within(bank).queryByText('Gnawa')).toBeNull()

    fireEvent.click(within(bank).getByRole('radio', { name: en.canon.bank.filter.locked }))
    expect(within(bank).getByText('Gnawa')).toBeInTheDocument()
    expect(within(bank).queryByText('Mande')).toBeNull()
  })
})
