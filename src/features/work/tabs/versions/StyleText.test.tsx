import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { StyleBrick, TextCheck } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A style prompt is read against the dictionary of sound (v0.94): the
 * phrases it knows are marked and named with what they mean, the tags it
 * does not are offered to the dictionary, and the dictionary stands beside
 * the text to write from - a press puts a phrase in where the caret is.
 */

const STYLE = 'warm synth pop, slow build, airy female vocal'

const brick = (id: string, type: string, phrase: string, meaning: string): StyleBrick => ({
  trial_id: null,
  id,
  profile_id: IDS.profile,
  type_key: type,
  name: phrase,
  description: phrase,
  hint: null,
  status: 'ready',
  created_at: NOW,
  updated_at: NOW,
  reference_count: 0,
  label: null,
  family: null,
  when_to_use: null,
  explanation: { en: meaning, ru: meaning },
  colours: [],
  sample: null,
  set_key: null,
  origin: 'own',
})

/** What the backend says of the style: one phrase it knows, one it does not. */
function checkOf(text: string): TextCheck {
  const known = text.indexOf('slow build')
  const unknown = text.indexOf('airy female vocal')
  return {
    repeats: [],
    terms: [],
    stress: [],
    accents: [],
    phrases:
      known < 0
        ? []
        : [
            {
              brick_id: 'b-slow',
              type_key: 'dynamics',
              phrase: 'slow build',
              explanation: { en: 'Grows towards the end.', ru: 'Растёт к концу.' },
              house: false,
              count: 1,
            },
          ],
    unknown: unknown < 0 ? [] : [{ phrase: 'airy female vocal', count: 1 }],
    marks: [
      ...(known < 0 ? [] : [{ start: known, end: known + 10, phrase: 0 }]),
      ...(unknown < 0 ? [] : [{ start: unknown, end: unknown + 17, unknown: 0 }]),
    ],
  }
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.bricks.push(
    brick('b-slow', 'dynamics', 'slow build', 'Grows towards the end.'),
    brick('b-amen', 'groove', 'chopped amen break', 'A fast cut-up break.'),
  )
  backend = mockBackend(answersFor(workspace))
  backend.answer('check_text', ({ text }) => checkOf(text as string))
})

afterEach(() => {
  vi.useRealTimers()
})

async function openStyle() {
  const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.style}`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText(/warm synth pop/)
  return main
}

describe('a style prompt', () => {
  it('is read against the dictionary, by its work and role', async () => {
    const main = await openStyle()
    await waitFor(() =>
      expect(backend.argsOf('check_text')).toContainEqual(
        expect.objectContaining({ text: STYLE, workId: IDS.song, role: 'style' }),
      ),
    )
    expect(
      await within(main).findByText(en.phrases.known_one.replace('{{count}}', '1')),
    ).toBeVisible()
    // The phrase it knows is named with what it means; the one it does not
    // is offered to the dictionary.
    const known = within(main).getAllByText('slow build')
    expect(known.some((one) => one.getAttribute('title')?.includes('Grows towards the end.'))).toBe(
      true,
    )
    fireEvent.click(within(main).getByRole('button', { name: 'airy female vocal' }))
    expect(await screen.findByRole('menuitem', { name: en.phrases.addByHand })).toBeVisible()
  })

  it('marks the phrase it knows in the text', async () => {
    const main = await openStyle()
    await waitFor(() => expect(main.querySelector('mark.phrase-mark')).not.toBeNull())
    expect(main.querySelector('mark.phrase-mark')).toHaveTextContent('slow build')
    expect(main.querySelector('mark.phrase-unknown')).toHaveTextContent('airy female vocal')
  })

  it('takes a phrase from the dictionary beside it, at the end of the text read', async () => {
    const main = await openStyle()
    fireEvent.click(within(main).getByRole('button', { name: en.phrases.dictionary }))
    const panel = await within(main).findByRole('complementary', { name: en.phrases.dictionary })
    fireEvent.click(await within(panel).findByRole('button', { name: /chopped amen break/ }))
    const box = await screen.findByRole('textbox', { name: en.versions.edit })
    expect(box).toHaveValue(`${STYLE}, chopped amen break`)
  })

  it('is not a lyric: the lyric lane is checked without a dictionary', async () => {
    const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.lyrics}`)
    await settled(client)
    await screen.findByText('[Chorus]')
    await waitFor(() => expect(backend.argsOf('check_text').length).toBeGreaterThan(0))
    expect(
      within(screen.getByRole('main')).queryByRole('button', { name: en.phrases.dictionary }),
    ).toBeNull()
  })
})
