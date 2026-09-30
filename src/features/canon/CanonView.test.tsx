import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Fact, NewFact } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The canon's card (v0.84): the lens dims what a task may not read - by the
 * backend's word, carried on each fact - and a fact is written into the
 * section it was added under.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    add_fact: ({ fact }) => {
      const asked = fact as NewFact
      const written: Fact = {
        id: 'f-new',
        profile_id: IDS.profile,
        note_id: asked.note_id,
        section: asked.section,
        body: asked.body,
        layer: asked.layer ?? 'public',
        status: asked.status ?? 'canon',
        retired_reason: null,
        source: null,
        when: null,
        scope_work_id: null,
        data: {},
        position: 2,
        created_at: NOW,
        updated_at: NOW,
      }
      workspace.facts.push(written)
      return written
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The lantern keeper's card, open. */
async function keeper() {
  const { client } = renderApp(`/canon/${IDS.character}`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText('A long grey coat', { exact: false })
  return { client, main }
}

/** Whether the row of the fact reading `words` is drawn dimmed. */
function dimmed(main: HTMLElement, words: string): boolean {
  const row = within(main).getByText(words, { exact: false }).closest('.grid')
  expect(row, `no row for ${words}`).not.toBeNull()
  return row!.classList.contains('opacity-25')
}

describe('a card of the canon', () => {
  it('dims, through a cover, what a cover may not read', async () => {
    const { client, main } = await keeper()
    const coat = 'A long grey coat'
    const bridge = 'Has sold lanterns by the bridge'
    expect(dimmed(main, coat)).toBe(false)
    expect(dimmed(main, bridge)).toBe(false)

    fireEvent.click(screen.getByRole('radio', { name: en.canon.lensName.cover }))
    await settled(client)

    expect(dimmed(main, coat), 'a settled public look is what a cover reads').toBe(false)
    expect(dimmed(main, bridge), 'an internal draft of the biography reached a cover').toBe(true)
  })

  it('writes a fact into the section it was added under', async () => {
    const { client, main } = await keeper()
    const looks = within(main).getByRole('region', { name: 'Appearance' })

    fireEvent.click(within(looks).getByRole('button', { name: en.canon.add.facts }))
    const words = within(looks).getByRole('textbox', { name: en.canon.factWords })
    fireEvent.change(words, { target: { value: 'A scar over the left eyebrow.' } })
    await act(async () => {
      fireEvent.keyDown(words, { key: 'Enter', ctrlKey: true })
    })
    await settled(client)

    const [asked] = backend.argsOf('add_fact') as { fact: NewFact }[]
    expect(asked?.fact).toMatchObject({
      note_id: IDS.character,
      section: 'looks',
      body: 'A scar over the left eyebrow.',
    })
  })
})
