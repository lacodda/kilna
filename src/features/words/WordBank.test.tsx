import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { BlockView, RegisterEntry, WordsPackage } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The bank of words (v0.90, ADR 0052), a chip of the Notes screen: the blocks
 * in the owner's order beside the words of the one on show, words typed or
 * pasted into it, carried onto a block, and the stresses the owner's own
 * texts already write, kept only as far as they are ticked. What the record
 * holds afterwards is the backend's; these hold the window to what it sends.
 */

function entry(id: string, word: string, extra: Partial<RegisterEntry>): RegisterEntry {
  return {
    id,
    profile_id: IDS.profile,
    word,
    forms: [],
    kind: 'noun',
    strictness: null,
    bank: 'fresh',
    sung: [],
    topic: null,
    note: null,
    created_at: NOW,
    updated_at: NOW,
    uses: 0,
    ...extra,
  }
}

function block(id: string, name: string, position: number, termIds: string[]): BlockView {
  return {
    id,
    profile_id: IDS.profile,
    name,
    position,
    term_ids: termIds,
    created_at: NOW,
    updated_at: NOW,
  }
}

const TERMS: RegisterEntry[] = [
  entry('t-pulsar', 'пульсар', { sung: [{ written: 'пульсар', sung: 'пульсАр' }], uses: 2 }),
  entry('t-harbour', 'harbour', { bank: 'parked' }),
  entry('t-lantern', 'lantern', {}),
  entry('t-tide', 'tide', {}),
  // Spent, not banked: the register's, and not the bank's to show.
  entry('t-water', 'water', { strictness: 'limit', bank: null, uses: 5 }),
]

const BLOCKS: BlockView[] = [
  block('b-space', 'space', 0, ['t-pulsar']),
  // In the order they were put there, which is not the alphabet's.
  block('b-sea', 'the sea', 1, ['t-lantern', 't-harbour']),
]

const FOUND: WordsPackage = {
  words: [
    {
      word: 'Марсель',
      sung: [{ written: 'Марсель', sung: 'МарсЭль' }],
      note: 'Sung this way in “Tide”, “Shore”',
    },
    {
      word: 'пульсар',
      sung: [{ written: 'пульсар', sung: 'пульсАр' }],
      note: 'Sung this way in “Paper Lanterns”',
    },
  ],
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    list_terms: () => TERMS,
    list_blocks: () => BLOCKS,
    bank_words: ({ words }) => ({
      terms: (words as string[]).map((word, index) => entry(`t-new-${String(index)}`, word, {})),
      created: (words as string[]).length,
    }),
    add_to_block: () => null,
    move_block: () => BLOCKS,
    keep_words: ({ items }) => (items as string[] | null) ?? ['t-1', 't-2'],
  })
})

afterEach(() => {
  vi.useRealTimers()
  Reflect.deleteProperty(document, 'elementFromPoint')
})

async function bank() {
  const view = renderApp('/notes?words')
  await settled(view.client)
  const main = await screen.findByRole('main')
  await within(main).findByRole('button', { name: /^space/ })
  return { ...view, main }
}

/** The words on show, each row read from its start: the word comes first. */
function wordsShown(name: string): string[] {
  return within(screen.getByRole('group', { name }))
    .getAllByRole('listitem')
    .map((row) => row.textContent ?? '')
}

describe('the bank of words', () => {
  it('lists the blocks in the owner’s order and a block’s words in the order put there', async () => {
    const { client, main, container } = await bank()

    expect(
      [...container.querySelectorAll('[data-block-drop]')].map((row) => row.textContent),
    ).toEqual([expect.stringMatching(/^space/), expect.stringMatching(/^the sea/)])

    // The whole bank, fresh first: the spent word that is not banked is not here.
    const all = wordsShown(en.words.bank)
    expect(all).toHaveLength(4)
    expect(all[3]).toMatch(/^harbour/)
    expect(all.some((row) => row.startsWith('water'))).toBe(false)
    // How it is sung, and how many works sing it.
    const pulsar = all.find((row) => row.startsWith('пульсар'))
    expect(pulsar).toContain('пульсАр')
    expect(pulsar).toContain('sung in 2 works')

    fireEvent.click(within(main).getByRole('button', { name: /^the sea/ }))
    await settled(client)
    expect(wordsShown('the sea')).toEqual([
      expect.stringMatching(/^lantern/),
      expect.stringMatching(/^harbour/),
    ])

    fireEvent.click(within(main).getByRole('button', { name: /^In no block/ }))
    await settled(client)
    expect(wordsShown(en.words.bank)).toEqual([expect.stringMatching(/^tide/)])
  })

  it('banks a list typed or pasted by lines into the block on show', async () => {
    const { client, main } = await bank()
    fireEvent.click(within(main).getByRole('button', { name: /^the sea/ }))
    await settled(client)

    const field = within(main).getByRole('textbox', {
      name: en.words.addTo.replace('{{block}}', 'the sea'),
    })
    // A list pasted by lines would arrive glued into one word; it becomes
    // commas, to be read before Enter keeps it.
    fireEvent.paste(field, { clipboardData: { getData: () => 'маяк\nприбой\n\nмаяк' } })
    expect(field).toHaveValue('маяк, прибой')
    fireEvent.change(field, { target: { value: 'маяк, прибой, on the edge' } })
    await act(async () => fireEvent.keyDown(field, { key: 'Enter' }))
    await settled(client)

    expect(backend.argsOf('bank_words')).toEqual([
      { words: ['маяк', 'прибой', 'on the edge'], blockId: 'b-sea' },
    ])
    expect(field).toHaveValue('')
  })

  it('banks into no block from the whole bank', async () => {
    const { client, main } = await bank()
    const field = within(main).getByRole('textbox', { name: en.words.add })
    fireEvent.change(field, { target: { value: 'маяк' } })
    await act(async () => fireEvent.keyDown(field, { key: 'Enter' }))
    await settled(client)

    expect(backend.argsOf('bank_words')).toEqual([{ words: ['маяк'], blockId: null }])
  })

  it('puts a word carried onto a block there, and writes nothing over a block that has it', async () => {
    const { client, main, container } = await bank()

    /** Pick up `word` and let go of it over the block `blockId`. */
    const carry = async (word: string, blockId: string) => {
      const target = container.querySelector(`[data-block-drop="${blockId}"]`)
      expect(target, `the screen shows ${blockId}`).not.toBeNull()
      // jsdom lays nothing out: where the word lands is answered here.
      Object.defineProperty(document, 'elementFromPoint', {
        value: () => target,
        configurable: true,
      })
      fireEvent.pointerDown(within(main).getByTitle(word), { button: 0, clientX: 300, clientY: 40 })
      await act(async () => {
        fireEvent.pointerMove(window, { clientX: 60, clientY: 60 })
        fireEvent.pointerUp(window, { clientX: 60, clientY: 60 })
      })
      await settled(client)
    }

    await carry('пульсар', 'b-space')
    expect(backend.argsOf('add_to_block'), 'already there').toEqual([])

    await carry('lantern', 'b-space')
    expect(backend.argsOf('add_to_block')).toEqual([{ blockId: 'b-space', termId: 't-lantern' }])
  })

  it('puts a word in a block from its menu, and moves a block from its own', async () => {
    const { client, main } = await bank()
    const tide = within(screen.getByRole('group', { name: en.words.bank }))
      .getAllByRole('listitem')
      .find((row) => row.textContent?.startsWith('tide'))!

    fireEvent.click(within(tide).getByRole('button', { name: en.words.putIn }))
    const put = await screen.findByRole('menu')
    await act(async () => fireEvent.click(within(put).getByRole('menuitem', { name: 'the sea' })))
    await settled(client)
    expect(backend.argsOf('add_to_block')).toEqual([{ blockId: 'b-sea', termId: 't-tide' }])

    // The first block can only go down; the menu offers no way up.
    fireEvent.click(within(main).getAllByRole('button', { name: en.words.blockMenu })[0]!)
    const menu = await screen.findByRole('menu')
    expect(within(menu).queryByRole('menuitem', { name: en.words.moveUp })).toBeNull()
    await act(async () =>
      fireEvent.click(within(menu).getByRole('menuitem', { name: en.words.moveDown })),
    )
    await settled(client)
    expect(backend.argsOf('move_block')).toEqual([{ id: 'b-space', index: 1 }])
  })

  it('keeps only the words found in the texts that stay ticked', async () => {
    backend.answer('words_from_texts', () => FOUND)
    const { client, main } = await bank()

    fireEvent.click(within(main).getByRole('button', { name: en.words.fromTexts }))
    const dialog = await screen.findByRole('dialog')
    await within(dialog).findByText('МарсЭль')
    expect(within(dialog).getByText('Sung this way in “Tide”, “Shore”')).toBeInTheDocument()
    expect(within(dialog).getByRole('button', { name: 'Keep 2' })).toBeInTheDocument()

    fireEvent.click(within(dialog).getByRole('checkbox', { name: 'Keep “пульсар”' }))
    await act(async () => fireEvent.click(within(dialog).getByRole('button', { name: 'Keep 1' })))
    await settled(client)

    expect(backend.argsOf('keep_words')).toEqual([{ package: FOUND, items: ['word:0'] }])
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
  })

  it('says so when the texts know nothing the dictionary does not', async () => {
    const { main } = await bank()

    fireEvent.click(within(main).getByRole('button', { name: en.words.fromTexts }))
    const dialog = await screen.findByRole('dialog')

    expect(await within(dialog).findByText(en.words.fromTextsNone)).toBeInTheDocument()
    expect(within(dialog).queryByRole('button', { name: /^Keep/ })).toBeNull()
    expect(backend.unanswered).toEqual([])
  })
})
