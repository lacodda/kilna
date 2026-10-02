import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { RegisterEntry, TermUse, TextCheck } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The register of repeats (v0.85, ADR 0044): the list reads by how many works
 * carry a term, the strictness narrows it, a meaning's named works can be let
 * go - and under a version, the terms its text takes are listed and marked
 * where they stand, by the backend's word.
 */

const LYRIC =
  '[Verse]\nLanterns on the water\nfloating out of reach\n\n[Chorus]\nPaper, paper, carry the light'

function entry(id: string, word: string, extra: Partial<RegisterEntry>): RegisterEntry {
  return {
    id,
    profile_id: IDS.profile,
    word,
    forms: [],
    kind: 'noun',
    strictness: 'limit',
    bank: null,
    sung: [],
    topic: null,
    note: null,
    created_at: NOW,
    updated_at: NOW,
    uses: 0,
    ...extra,
  }
}

const TERMS: RegisterEntry[] = [
  entry('t-water', 'water', { uses: 2, topic: 'the harbour' }),
  entry('t-lantern', 'lantern', { strictness: 'ban', uses: 7 }),
  entry('t-drink', 'a lighthouse nobody keeps', { kind: 'image', uses: 1 }),
]

const USES: TermUse[] = [
  { work_id: IDS.song, title: 'Paper Lanterns', kind: 'song', found: 0, named: true },
]

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const at = LYRIC.indexOf('Lanterns')
  const check: TextCheck = {
    repeats: [],
    terms: [{ term_id: 't-lantern', word: 'lantern', kind: 'noun', strictness: 'ban', count: 1 }],
    marks: [{ start: at, end: at + 'Lanterns'.length, term: 0 }],
    stress: [],
    accents: [],
  }
  backend = mockBackend({
    ...answersFor(studio()),
    list_terms: () => TERMS,
    term_uses: () => USES,
    check_text: () => check,
    unlink_term: () => null,
  })
})

afterEach(() => {
  vi.useRealTimers()
})

async function register(path = '/register') {
  const { client } = renderApp(path)
  await settled(client)
  const main = await screen.findByRole('main')
  return { client, main }
}

describe('the register', () => {
  it('reads by how many works carry a term, and narrows by strictness', async () => {
    const { client, main } = await register()
    const rows = within(main)
      .getAllByRole('button')
      .map((row) => row.textContent ?? '')
      .filter((text) => TERMS.some((term) => text.includes(term.word)))
    expect(rows[0]).toContain('lantern')
    expect(rows[1]).toContain('water')

    // The chip, not a row: a row's dot says its strictness too.
    fireEvent.click(within(main).getByRole('button', { name: /^Banned\s*\d+$/ }))
    await settled(client)

    expect(within(main).queryByText('water')).toBeNull()
    expect(within(main).getByText('lantern')).toBeInTheDocument()
  })

  it('lets a meaning go of a work named for it', async () => {
    const { client, main } = await register('/register/t-drink')
    expect(await within(main).findByText(en.register.whereMeaning)).toBeInTheDocument()
    expect(within(main).getByText('Paper Lanterns')).toBeInTheDocument()

    fireEvent.click(within(main).getByRole('button', { name: en.register.letGo }))
    await settled(client)

    expect(backend.argsOf('unlink_term')).toEqual([{ termId: 't-drink', workId: IDS.song }])
  })
})

describe('the register under a version', () => {
  it('lists the terms the text takes and marks them where they stand', async () => {
    const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.lyrics}`)
    await settled(client)
    const main = await screen.findByRole('main')

    const chip = await within(main).findByTitle(/lantern: ×1 here, in 7 works/)
    expect(chip).toHaveTextContent('×1')
    await waitFor(() => {
      expect(within(main).getByText('Lanterns').closest('mark')).toHaveClass('term-ban')
    })

    fireEvent.click(chip)
    await settled(client)
    expect(await screen.findByDisplayValue('lantern')).toBeInTheDocument()
  })
})
