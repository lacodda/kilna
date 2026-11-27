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
    sung: [],
    topic: null,
    note: null,
    scope: 'lyrics',
    max_per_month: null,
    cooldown_days: null,
    created_at: NOW,
    updated_at: NOW,
    uses: 0,
    ...extra,
  }
}

const TERMS: RegisterEntry[] = [
  entry('t-two', 'spacesuit, isotope', { uses: 0 }),
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
    phrases: [],
    unknown: [],
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

  it('looks for a term in titles when told so (v0.151)', async () => {
    backend.answer('update_term', ({ patch }) => ({
      ...TERMS[0],
      ...(patch as object),
    }))
    const { client, main } = await register('/register/t-water')
    const scope = await within(main).findByRole('combobox', { name: en.register.scopeLabel })
    expect(scope).toHaveTextContent(en.register.scopes.lyrics)
    fireEvent.click(scope)
    const titles = await screen.findByRole('option', { name: en.register.scopes.title })
    fireEvent.pointerDown(titles, { pointerType: 'mouse' })
    fireEvent.click(titles)
    await settled(client)

    expect(backend.argsOf('update_term')).toEqual([{ id: 't-water', patch: { scope: 'title' } }])
  })
})

describe('a word reshaped in its own editor (v0.171)', () => {
  it('warns of a word of several and splits it at its commas', async () => {
    backend.answer('split_term', () => [entry('t-a', 'spacesuit', {}), entry('t-b', 'isotope', {})])
    const { client, main } = await register('/register/t-two')
    expect(await within(main).findByText(en.register.wordSeparator)).toBeInTheDocument()

    fireEvent.click(within(main).getAllByRole('button', { name: en.register.split })[0]!)
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByRole('textbox')).toHaveValue('spacesuit, isotope')
    fireEvent.click(
      within(dialog).getByRole('button', {
        name: en.register.splitDo_other.replace('{{count}}', '2'),
      }),
    )
    await settled(client)

    expect(backend.argsOf('split_term')).toEqual([{ id: 't-two', into: ['spacesuit', 'isotope'] }])
  })

  it('warns while a word is retyped with a slash, and says nothing of a plain one', async () => {
    const { main } = await register('/register/t-water')
    const word = await within(main).findByRole('textbox', { name: en.register.word })
    expect(within(main).queryByText(en.register.wordSeparator)).toBeNull()

    fireEvent.change(word, { target: { value: 'water / sea' } })
    expect(within(main).getByText(en.register.wordSeparator)).toBeInTheDocument()
  })

  it('merges a word into the one picked', async () => {
    backend.answer('merge_terms', () => entry('t-lantern', 'lantern', { forms: ['water'] }))
    const { client, main } = await register('/register/t-water')
    fireEvent.click(await within(main).findByRole('button', { name: en.register.merge }))
    const dialog = await screen.findByRole('dialog')
    // The word itself is not offered to itself.
    expect(within(dialog).queryByRole('button', { name: /^water/ })).toBeNull()

    fireEvent.change(within(dialog).getByRole('textbox'), { target: { value: 'lant' } })
    fireEvent.click(within(dialog).getByRole('button', { name: /^lantern/ }))
    fireEvent.click(
      within(dialog).getByRole('button', {
        name: en.register.mergeDo.replace('{{word}}', 'water').replace('{{into}}', 'lantern'),
      }),
    )
    await settled(client)

    expect(backend.argsOf('merge_terms')).toEqual([{ id: 't-water', intoId: 't-lantern' }])
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
