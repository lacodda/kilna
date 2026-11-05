import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The assistant's section of Settings (v0.108): the constant that said three
 * runs since v0.27, the empty directory of ADR 0008, and the model nobody could
 * choose. Each control saves as it is answered; what stands is what the backend
 * says stands.
 */

let backend: Backend
let stored: { parallel_runs: number; model: string | null; run_sees: string }

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  stored = { parallel_runs: 3, model: null, run_sees: 'nothing' }
  backend = mockBackend({
    ...answersFor(studio()),
    assistant_status: () => ({ available: true, version: '2.1.318 (Claude Code)', reason: null }),
    assistant_settings: () => stored,
    set_parallel_runs: ({ runs }) => {
      stored = { ...stored, parallel_runs: runs as number }
      return stored
    },
    set_run_model: ({ model }) => {
      if (model === 'not a name') throw new Error('refused')
      stored = { ...stored, model: model as string | null }
      return stored
    },
    set_run_sees: ({ sees }) => {
      stored = { ...stored, run_sees: sees as string }
      return stored
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function open() {
  const { client } = renderApp('/settings/assistant')
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByRole('radio', { name: en.settings.assistant.seesName.nothing })
  await settled(client)
  return { client, main }
}

describe('the assistant in Settings', () => {
  it('says which Claude Code answered, and asks again when told to', async () => {
    const { client, main } = await open()

    expect(within(main).getByText('2.1.318 (Claude Code)')).toBeVisible()
    const asked = backend.argsOf('assistant_status').length

    backend.answer('assistant_status', () => ({
      available: false,
      version: null,
      reason: 'Claude Code is not on the PATH.',
    }))
    fireEvent.click(within(main).getByRole('button', { name: en.settings.assistant.cliCheck }))
    await settled(client)

    expect(backend.argsOf('assistant_status').length).toBe(asked + 1)
    expect(within(main).getByText(en.settings.assistant.cliMissing)).toBeVisible()
    expect(within(main).getByText('Claude Code is not on the PATH.')).toBeVisible()
  })

  it('sends how many runs may go at once, from a press that can only say a number in bounds', async () => {
    const { client, main } = await open()
    const group = within(main).getByRole('radiogroup', { name: en.settings.assistant.runs })
    const segments = within(group).getAllByRole('radio')

    expect(segments.map((segment) => segment.textContent)).toEqual([
      '1',
      '2',
      '3',
      '4',
      '5',
      '6',
      '7',
      '8',
    ])
    expect(within(group).getByRole('radio', { name: '3' })).toBeChecked()

    fireEvent.click(within(group).getByRole('radio', { name: '5' }))
    await settled(client)

    expect(backend.argsOf('set_parallel_runs')).toEqual([{ runs: 5 }])
    expect(within(group).getByRole('radio', { name: '5' })).toBeChecked()
  })

  it('saves a model on leaving, clears it when emptied, and writes nothing unchanged', async () => {
    const { client, main } = await open()
    const box = () => within(main).getByRole('textbox', { name: en.settings.assistant.model })

    fireEvent.blur(box())
    await settled(client)
    expect(backend.argsOf('set_run_model')).toEqual([])

    fireEvent.change(box(), { target: { value: '  sonnet ' } })
    fireEvent.blur(box())
    await settled(client)
    expect(backend.argsOf('set_run_model')).toEqual([{ model: 'sonnet' }])
    expect(box()).toHaveValue('sonnet')

    fireEvent.change(box(), { target: { value: '' } })
    fireEvent.keyDown(box(), { key: 'Enter' })
    await settled(client)
    expect(backend.argsOf('set_run_model')).toEqual([{ model: 'sonnet' }, { model: null }])
  })

  it('puts back what stands when a name is refused', async () => {
    const { client, main } = await open()
    const box = () => within(main).getByRole('textbox', { name: en.settings.assistant.model })

    fireEvent.change(box(), { target: { value: 'not a name' } })
    fireEvent.blur(box())
    await settled(client)

    expect(box()).toHaveValue('')
    expect(await screen.findByText(en.settings.assistant.saveFailed)).toBeVisible()
  })

  it('says what each answer about a run’s reading means, and sends the one pressed', async () => {
    const { client, main } = await open()

    expect(within(main).getByText(en.settings.assistant.seesHint.nothing)).toBeVisible()

    fireEvent.click(within(main).getByRole('radio', { name: en.settings.assistant.seesName.work }))
    await settled(client)

    expect(backend.argsOf('set_run_sees')).toEqual([{ sees: 'work' }])
    expect(within(main).getByText(en.settings.assistant.seesHint.work)).toBeVisible()
    expect(within(main).queryByText(en.settings.assistant.seesHint.nothing)).toBeNull()
  })

  it('holds the same budget form as the assistant’s head', async () => {
    const { client, main } = await open()

    fireEvent.change(within(main).getByRole('textbox', { name: en.assistant.budget }), {
      target: { value: '15' },
    })
    fireEvent.blur(within(main).getByRole('textbox', { name: en.assistant.budget }))
    fireEvent.click(within(main).getByRole('button', { name: en.assistant.budgetSave }))
    await settled(client)

    expect(backend.argsOf('set_weekly_budget')).toEqual([{ budget: 15 }])
  })
})
