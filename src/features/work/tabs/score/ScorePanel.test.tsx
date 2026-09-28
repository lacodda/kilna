import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { NewScore, Score } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Score tab's two states (v0.80): a recorded score is read, and a new
 * one is given on empty scales - never the one dressed as the other.
 *
 * Until then the scales of the recorded score were also the form for the
 * next, so the first mark moved on a reading started a new score from the
 * old one, and a line of small print had to say which state the panel was
 * in. Walked here: the tab opens on the verdict that stands with nothing to
 * move, "New score" opens scales with nothing on them, and what is recorded
 * judges the current draft - offered only among the drafts, not the review
 * or the style prompt beside them.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    score_work: ({ workId, score }) => {
      const given = score as NewScore
      return {
        id: 's-new',
        work_id: workId as string,
        version_id: given.version_id ?? null,
        axes: given.axes as Record<string, number>,
        total: 80,
        tier: 'clip',
        note: given.note ?? null,
        rater: given.rater ?? null,
        scored_at: NOW,
        revision: 2,
      } satisfies Score
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function open() {
  const { client } = renderApp(`/works/${IDS.song}/score`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText('The chorus carries it.')
  await settled(client)
  return { client, main }
}

describe('the score tab', () => {
  it('opens on the recorded score, read rather than editable', async () => {
    const { main } = await open()

    // Every axis a meter holding its recorded mark; nothing to drag.
    expect(within(main).queryAllByRole('slider')).toEqual([])
    const hook = within(main).getByRole('meter', { name: 'Hook' })
    expect(hook).toHaveAttribute('aria-valuenow', '8')

    // Which draft it judged, by its role, and who judged it.
    expect(within(main).getAllByText('Score of Lyrics v2').length).toBeGreaterThan(0)
    expect(within(main).getByText('Lyrics v2 · Second pass')).toBeInTheDocument()
    expect(within(main).getByText(en.score.raterSelf)).toBeInTheDocument()

    // Nothing to record from a reading.
    expect(within(main).queryByRole('button', { name: en.score.save })).toBeNull()
  })

  it('gives a new score on empty scales, for the current draft', async () => {
    const { client, main } = await open()

    fireEvent.click(within(main).getByRole('button', { name: en.score.new }))

    const sliders = within(main).getAllByRole('slider')
    expect(sliders.length).toBe(6)
    for (const slider of sliders) expect(slider).not.toHaveAttribute('aria-valuenow')
    expect(within(main).getByRole('button', { name: en.score.save })).toBeDisabled()

    // The draft being judged defaults to the current one; the style prompt
    // beside it is not a draft of the song and is not offered.
    const picker = within(main).getByRole('combobox', { name: en.score.ofVersion })
    expect(picker).toHaveTextContent('Lyrics · Second pass')
    fireEvent.click(picker)
    const offered = (await screen.findAllByRole('option')).map((option) => option.textContent)
    expect(offered).toEqual(['Lyrics · Second pass · current', 'Lyrics · Revision 1'])
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' })

    const hook = within(main).getByRole('slider', { name: 'Hook' })
    fireEvent.keyDown(hook, { key: 'End' })
    const save = within(main).getByRole('button', { name: en.score.save })
    expect(save).toBeEnabled()

    await act(async () => {
      fireEvent.click(save)
    })
    await settled(client)

    expect(backend.argsOf('score_work')).toEqual([
      {
        workId: IDS.song,
        score: { axes: { hook: 10 }, version_id: IDS.lyrics, note: undefined, rater: undefined },
      },
    ])
  })

  it('judges blind on empty scales, and a peek keeps the marks already set', async () => {
    const { main } = await open()

    fireEvent.click(within(main).getByRole('button', { name: en.score.blindOn }))

    // The recorded score is the past verdict: blind opens the scales, and
    // the rows are held back.
    expect(within(main).queryByText('The chorus carries it.')).toBeNull()
    expect(within(main).getByText(en.score.blindHidden)).toBeInTheDocument()
    const hook = within(main).getByRole('slider', { name: 'Hook' })
    fireEvent.keyDown(hook, { key: 'End' })

    fireEvent.click(within(main).getByRole('button', { name: en.score.blindReveal }))

    // The rows are back beside the scales, and the mark set blind stands.
    expect(within(main).getAllByText('Score of Lyrics v2').length).toBeGreaterThan(0)
    expect(within(main).getByRole('slider', { name: 'Hook' })).toHaveAttribute(
      'aria-valuenow',
      '10',
    )
  })
})
