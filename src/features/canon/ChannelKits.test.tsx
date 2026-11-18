import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { NewFact } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The channel's dressing (v0.147, FR-10 10.3): a kit is an avatar, a banner
 * or a watermark - its name, the prompt it is drawn from, what it keeps out,
 * and whether it is the look before a rebranding - written into the root
 * card's section of kits.
 */

let backend: Backend

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
  backend = mockBackend({
    ...answersFor(workspace),
    add_fact: ({ fact }) => ({ ...(fact as NewFact), id: 'f-kit' }),
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('a kit of the dressing', () => {
  it('is written with what it is, its prompt, and whether it is the old look', async () => {
    const { client } = renderApp('/canon/n-channel')
    await settled(client)
    const main = await screen.findByRole('main')
    const kits = await within(main).findByRole('region', { name: 'Dressing' })

    fireEvent.click(within(kits).getByRole('button', { name: en.canon.add.branding }))
    fireEvent.click(within(kits).getByRole('radio', { name: en.canon.kit.avatar }))
    fireEvent.change(within(kits).getByRole('textbox', { name: en.canon.factWords }), {
      target: { value: 'Eyes closed' },
    })
    fireEvent.change(within(kits).getByRole('textbox', { name: en.canon.kitPrompt }), {
      target: { value: 'a girl with closed eyes, soft window light' },
    })
    fireEvent.click(within(kits).getByRole('switch', { name: en.canon.kitArchived }))
    await act(async () => {
      fireEvent.keyDown(within(kits).getByRole('textbox', { name: en.canon.factWords }), {
        key: 'Enter',
        ctrlKey: true,
      })
    })
    await settled(client)

    const [asked] = backend.argsOf('add_fact') as { fact: NewFact }[]
    expect(asked?.fact).toMatchObject({
      note_id: 'n-channel',
      section: 'branding',
      body: 'Eyes closed',
      data: {
        kind: 'avatar',
        prompt: 'a girl with closed eyes, soft window light',
        archived: true,
      },
    })
  })
})
