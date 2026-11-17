import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { IntakeBrought, IntakeFound, IntakeRequest } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Bringing works in (v0.113): the dialog behind the New button's "From files
 * or a text…". What it shows is the backend's reading of what was handed
 * over, and what it sends is what is on the screen - so both are held here:
 * the list says what each thing is and where it stands, and the button sends
 * the text, the title and the kind as they were typed.
 */

let backend: Backend

const pastedPage: IntakeFound = {
  name: null,
  title: 'the tide is out',
  kind: null,
  versions: 1,
  scores: 0,
  files: 0,
  standing: 'new',
  reason: null,
}

const done: IntakeBrought = {
  works: [
    {
      id: 'brought-1',
      title: 'Low water',
      versions: 1,
      scores: 0,
      notes: 0,
      scenes: 0,
      files: 0,
      missing: 0,
    },
  ],
  here: 0,
  deleted: 0,
  refused: [],
}

beforeEach(() => {
  backend = mockBackend({
    ...answersFor(studio()),
    survey_intake: () => [pastedPage],
    bring_intake: () => done,
  })
})

/** The dialog, opened the way a person opens it. */
async function openDialog(): Promise<HTMLElement> {
  const { client } = renderApp('/catalogue')
  await settled(client)
  fireEvent.click(screen.getByRole('button', { name: en.shell.new }))
  fireEvent.click(await screen.findByRole('button', { name: en.works.bringIn }))
  return screen.findByRole('dialog', { name: en.intake.title })
}

describe('bringing works in', () => {
  it('shows what a pasted text would become and sends it as typed', async () => {
    const dialog = await openDialog()
    const bring = within(dialog).getByRole('button', {
      name: en.intake.bring.replace('{{count}}', '0'),
    })
    // Nothing handed over: nothing to bring, and nothing asked of the backend.
    expect(bring).toBeDisabled()
    expect(backend.argsOf('survey_intake')).toHaveLength(0)

    fireEvent.change(within(dialog).getByLabelText(en.intake.paste), {
      target: { value: 'the tide is out\nthe boats lean' },
    })

    // The backend's reading, once the text has settled: its title, and that
    // it is new here.
    expect(await within(dialog).findByText('the tide is out')).toBeInTheDocument()
    expect(within(dialog).getByText(en.intake.standing.new)).toBeInTheDocument()
    const asked = backend.argsOf('survey_intake').at(-1)?.request as IntakeRequest
    expect(asked.text).toBe('the tide is out\nthe boats lean')
    expect(asked.kind).toBe('song')

    // A text with no file is called what the person says.
    fireEvent.change(within(dialog).getByLabelText(en.intake.pastedTitle), {
      target: { value: 'Low water' },
    })
    const ready = await within(dialog).findByRole('button', {
      name: en.intake.bring.replace('{{count}}', '1'),
    })
    await waitFor(() => expect(ready).toBeEnabled())
    fireEvent.click(ready)

    await waitFor(() => expect(backend.argsOf('bring_intake')).toHaveLength(1))
    const sent = backend.argsOf('bring_intake')[0]?.request as IntakeRequest
    expect(sent).toEqual({
      paths: [],
      text: 'the tide is out\nthe boats lean',
      title: 'Low water',
      kind: 'song',
    })
  })

  it('says why a thing cannot come in and keeps the button off', async () => {
    backend.answer('survey_intake', () => [
      {
        ...pastedPage,
        title: '',
        versions: 0,
        standing: null,
        reason: { key: 'refusal.intake.notAWorkPage', params: {} },
      } satisfies IntakeFound,
    ])
    const dialog = await openDialog()
    fireEvent.change(within(dialog).getByLabelText(en.intake.paste), {
      target: { value: '---\nformat: 7\n---\n' },
    })

    expect(await within(dialog).findByText(en.refusal.intake.notAWorkPage)).toBeInTheDocument()
    expect(
      within(dialog).getByRole('button', { name: en.intake.bring.replace('{{count}}', '0') }),
    ).toBeDisabled()
  })
})
