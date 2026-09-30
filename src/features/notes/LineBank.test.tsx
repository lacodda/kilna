import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Note } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The bank of phrases (v0.85, ADR 0045): a phrase is a row of its own table,
 * not a page under "All", and sending one to a work ties it to the work and
 * marks it used in one gesture - it stays in the bank.
 */

function phrase(id: string, body: string, state: Note['state'] = 'fresh'): Note {
  return {
    id,
    profile_id: IDS.profile,
    work_id: null,
    kind: 'phrase',
    title: null,
    body,
    tags: ['sea'],
    created_at: NOW,
    updated_at: NOW,
    layer: 'public',
    aliases: [],
    prompt: null,
    prompt_basis: null,
    state,
  }
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.notes.push(
    phrase('n-tide', 'The tide keeps its own time'),
    phrase('n-bell', 'A bell for nobody', 'dropped'),
  )
  backend = mockBackend({
    ...answersFor(workspace),
    update_note: ({ id }) => workspace.notes.find((note) => note.id === id),
  })
})

afterEach(() => {
  vi.useRealTimers()
})

async function notes() {
  const { client } = renderApp('/notes')
  await settled(client)
  const main = await screen.findByRole('main')
  return { client, main }
}

describe('the bank of phrases', () => {
  it('keeps the phrases out of "All" and shows the fresh ones as rows of their own', async () => {
    const { client, main } = await notes()
    expect(within(main).queryByText('The tide keeps its own time')).toBeNull()
    expect(within(main).getByText('A song about tides')).toBeInTheDocument()

    fireEvent.click(within(main).getByRole('button', { name: /^Phrase/ }))
    await settled(client)

    expect(await within(main).findByText('The tide keeps its own time')).toBeInTheDocument()
    expect(within(main).queryByText('A bell for nobody'), 'fresh first').toBeNull()
    expect(within(main).queryByText('A song about tides')).toBeNull()
  })

  it('sends a phrase to a work: tied to it and used, in one gesture', async () => {
    const { client, main } = await notes()
    fireEvent.click(within(main).getByRole('button', { name: /^Phrase/ }))
    await settled(client)
    await within(main).findByText('The tide keeps its own time')

    fireEvent.click(within(main).getByRole('button', { name: en.notes.toWork }))
    const box = await screen.findByRole('dialog')
    fireEvent.click(await within(box).findByRole('option', { name: /Harbour Lights/ }))
    await settled(client)

    await waitFor(() => {
      expect(backend.argsOf('update_note')).toEqual([
        { id: 'n-tide', patch: { work_id: IDS.draft, state: 'used' } },
      ])
    })
  })
})
