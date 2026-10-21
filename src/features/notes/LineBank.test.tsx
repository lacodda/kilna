import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { LineRow, Note, ShelfView } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The phrases (v0.98, ADR 0064): the first of the two shelves, a row each,
 * sorted onto shelves - the bank of words and the bank of phrases made one.
 * A phrase attached to a work is kept for it, not spent by it; a list pasted
 * keeps one phrase per line; a row says how its word is sung and how many
 * works already say it.
 */

function phrase(id: string, body: string, state: Note['state'] = 'fresh'): Note {
  return {
    id,
    profile_id: IDS.profile,
    work_id: null,
    kind: 'phrase',
    title: null,
    body,
    attribution: null,
    medium: null,
    year: null,
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

const SHELF: ShelfView = {
  id: 's-sea',
  profile_id: IDS.profile,
  name: 'The sea',
  position: 0,
  note_ids: ['n-bell', 'n-tide'],
  created_at: NOW,
  updated_at: NOW,
}

let backend: Backend
let workspace: Studio

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  workspace.notes.push(
    phrase('n-tide', 'The tide keeps its own time'),
    { ...phrase('n-joy', 'sorrow is worn-out joy'), attribution: 'Old Joy' },
    phrase('n-marseille', 'Марсель'),
    phrase('n-bell', 'A bell for nobody', 'dropped'),
  )
  const answers = answersFor(workspace)
  const lines = answers.list_lines
  if (lines === undefined) throw new Error('the test backend lists no phrases')
  backend = mockBackend({
    ...answers,
    // What the record knows about the phrases: one stands on a shelf, one
    // is sung its own way and said by two works.
    list_lines: (args) =>
      (lines(args) as LineRow[]).map((row) =>
        row.id === 'n-marseille'
          ? { ...row, uses: 2, sung: [{ written: 'Марсель', sung: 'МарсЭль' }] }
          : row.id === 'n-tide' || row.id === 'n-bell'
            ? { ...row, shelf_ids: [SHELF.id] }
            : row,
      ),
    list_shelves: () => [SHELF],
    update_note: ({ id }) => workspace.notes.find((note) => note.id === id),
    keep_lines: () => ({ notes: [workspace.notes[0]], created: 1 }),
    add_to_shelf: () => null,
    list_tags: () => [['sea', 4]],
    retag_notes: () => 4,
  })
})

afterEach(() => {
  vi.useRealTimers()
})

async function phrases() {
  const { client } = renderApp('/notes')
  await settled(client)
  const main = await screen.findByRole('main')
  return { client, main }
}

describe('the phrases', () => {
  it('opens on the phrases, fresh ones as rows, and keeps the notes on their own chip', async () => {
    const { client, main } = await phrases()

    expect(await within(main).findByText('The tide keeps its own time')).toBeInTheDocument()
    expect(within(main).queryByText('A bell for nobody'), 'fresh first').toBeNull()
    expect(within(main).queryByText('A song about tides')).toBeNull()

    fireEvent.click(within(main).getByRole('button', { name: /^Note/ }))
    await settled(client)

    expect(await within(main).findByText('A song about tides')).toBeInTheDocument()
    expect(within(main).queryByText('The tide keeps its own time')).toBeNull()
  })

  it('says whose words a quote is, how a word is sung and how many works say it', async () => {
    const { main } = await phrases()

    expect(await within(main).findByText('— Old Joy')).toBeInTheDocument()
    expect(within(main).getByText('МарсЭль')).toBeInTheDocument()
    expect(within(main).getByText('sung in 2 works')).toBeInTheDocument()
  })

  it('attaches a phrase to a work: kept for it, not spent by it', async () => {
    const { client, main } = await phrases()
    await within(main).findByText('The tide keeps its own time')
    const row = within(main).getByText('The tide keeps its own time').closest('li')
    if (row === null) throw new Error('the phrase is not a row')

    fireEvent.click(within(row).getByRole('button', { name: en.notes.toWork }))
    const box = await screen.findByRole('dialog')
    fireEvent.click(await within(box).findByRole('option', { name: /Harbour Lights/ }))
    await settled(client)

    await waitFor(() => {
      expect(backend.argsOf('update_note')).toEqual([
        { id: 'n-tide', patch: { work_id: IDS.draft } },
      ])
    })
  })

  it('keeps a pasted list one phrase per line, commas and all', async () => {
    const { client, main } = await phrases()
    const field = await within(main).findByRole('textbox', { name: en.notes.linePlaceholder })

    fireEvent.change(field, { target: { value: 'кофе, пульсар и море\nприбой\nПрибой' } })
    await act(async () => {
      fireEvent.keyDown(field, { key: 'Enter' })
    })
    await settled(client)

    expect(backend.argsOf('keep_lines')).toEqual([
      { lines: ['кофе, пульсар и море', 'прибой'], kind: 'phrase', shelfId: null, tags: [] },
    ])
  })

  it('reads a shelf in the order its phrases were put there', async () => {
    const { client, main } = await phrases()

    fireEvent.click(await within(main).findByRole('button', { name: /^The sea/ }))
    await settled(client)

    // Only the fresh are on show, and on this shelf: the tide; and a phrase
    // typed now goes onto the shelf.
    expect(within(main).getByText('The tide keeps its own time')).toBeInTheDocument()
    expect(within(main).queryByText('sorrow is worn-out joy')).toBeNull()
    expect(
      within(main).getByRole('textbox', {
        name: en.notes.linePlaceholderOn.replace('{{shelf}}', 'The sea'),
      }),
    ).toBeInTheDocument()
  })

  it('takes a tag off every note at once, from the tag on show', async () => {
    const { client, main } = await phrases()

    fireEvent.click(await within(main).findByRole('combobox', { name: en.notes.tag }))
    // Base UI picks on the press, as a pointer makes it.
    const sea = await screen.findByRole('option', { name: /^sea/ })
    fireEvent.pointerDown(sea, { pointerType: 'mouse' })
    fireEvent.click(sea)
    await settled(client)
    fireEvent.click(within(main).getByRole('button', { name: en.notes.tagMenu }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.notes.untag }))
    await settled(client)

    expect(backend.argsOf('retag_notes')).toEqual([{ from: 'sea', to: null }])
  })
})
