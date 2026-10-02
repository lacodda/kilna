import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Axis, Score, Work, WorkPatch } from '@/lib/api/types'
import { mockBackend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'
import { axisBar } from '@/features/work/tabs/overview/MiniBars'

/*
 * The overview as a board of widgets (v0.82), on the fake studio: "Paper
 * Lanterns" is a scored song with lyrics and a style prompt, going out as the
 * clip and the audio made from it (v0.86); "Harbour Lights" a song nothing
 * has judged; the clip a video with a storyboard and a release of its own.
 */

let workspace: Studio
let written: WorkPatch[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  written = []
})

afterEach(() => {
  vi.useRealTimers()
})

/** Open the overview of `workId` once everything it asks has answered. */
async function openOverview(workId: string) {
  mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => {
      written.push(patch as WorkPatch)
      const work = workspace.works.find((w) => w.id === id)!
      return { ...work, ...(patch as Partial<Work>) }
    },
  })
  const view = renderApp(`/works/${workId}/overview`)
  await settled(view.client)
  const main = await screen.findByRole('main')
  // Every kind draws its links, so the board has landed when they have.
  await within(main).findByRole('group', { name: en.card.tab.links })
  await settled(view.client)
  return { ...view, main }
}

/** The names of the widgets drawn, in the order they stand. */
function widgetsIn(main: HTMLElement): string[] {
  return within(main)
    .getAllByRole('group')
    .map((group) => group.getAttribute('aria-labelledby'))
    .filter((id): id is string => id !== null)
    .map((id) => document.getElementById(id)?.textContent ?? '')
}

describe('the overview', () => {
  it("draws the owner's layout, the lead column and its rail, when the profile names none", async () => {
    const { main } = await openOverview(IDS.song)

    const board = main.querySelector('[data-layout]')
    expect(board).toHaveAttribute('data-layout', 'lead')
    const names = widgetsIn(main)
    // What the song goes out as leads, the text under it, and what a song has
    // no use for is not drawn at all: no storyboard, and no releases of its
    // own - it goes out as its publications (ADR 0047).
    expect(names[0]).toBe(en.publications.caption)
    expect(names[1]).toMatch(/^Lyrics · Revision 2/)
    expect(names).toContain('Style prompt · Revision 1')
    expect(names).toContain(en.card.tab.score)
    expect(names).not.toContain(en.card.tab.scenes)
    expect(names).not.toContain(en.releases.caption)
  })

  it('draws a storyboard on a clip, and no style prompt where the kind keeps none', async () => {
    const { main } = await openOverview(IDS.video)

    const names = widgetsIn(main)
    expect(names).toContain(en.card.tab.scenes)
    expect(names.some((name) => name.startsWith('Style prompt'))).toBe(false)
    // A clip goes out itself: its release stands on the board (there is no
    // Releases tab since v0.90), not publications of its own.
    expect(names).toContain(en.releases.caption)
    expect(names).not.toContain(en.publications.caption)
    const board = within(main).getByRole('group', { name: en.card.tab.scenes })
    expect(within(board).getByText('2')).toBeInTheDocument()
  })

  it('invites a score where there is none, with the way to give one', async () => {
    const { main } = await openOverview(IDS.draft)

    const score = within(main).getByRole('group', { name: en.card.tab.score })
    expect(within(score).getByText(en.score.none)).toBeInTheDocument()
    expect(within(score).getByRole('link', { name: en.overview.scoreIt })).toHaveAttribute(
      'href',
      `/works/${IDS.draft}/score`,
    )
  })

  it('writes the one field that changed, and nothing beside it', async () => {
    const { main } = await openOverview(IDS.song)

    const bpm = within(main).getByRole('textbox', { name: 'BPM' })
    fireEvent.focus(bpm)
    fireEvent.change(bpm, { target: { value: '104' } })
    // Enter hands the box its blur, which is where a field is kept; the test
    // window has no focus to take away, so the blur is sent the same way.
    fireEvent.keyDown(bpm, { key: 'Enter' })
    fireEvent.blur(bpm)

    await waitFor(() => expect(written).toEqual([{ meta: { bpm: 104 } }]))
  })

  it('goes to the tab that owns a widget when the widget is clicked', async () => {
    const { main } = await openOverview(IDS.song)

    fireEvent.click(await within(main).findByText(/Paper, paper, carry the light/))

    const tabs = await screen.findByRole('navigation', { name: en.card.tabs })
    await waitFor(() =>
      expect(within(tabs).getByRole('link', { current: 'page' })).toHaveAttribute(
        'href',
        `/works/${IDS.song}/versions`,
      ),
    )
  })

  it('lays the same widgets out the way the profile says', async () => {
    // A copy: the studio's vocabulary is the fixture's one object, shared by
    // every studio this file makes.
    workspace.profile.config = {
      ...workspace.profile.config,
      overview: { layout: 'sheet', widgets: [] },
    }
    const { main } = await openOverview(IDS.song)

    expect(main.querySelector('[data-layout]')).toHaveAttribute('data-layout', 'sheet')
    expect(widgetsIn(main)).toContain(en.publications.caption)
  })
})

describe('an axis bar', () => {
  it('says a half mark as it is, and a whole mark whole', () => {
    // 9.5 read as 10 on the board claimed a mark the score did not give.
    const axis = { key: 'hook', label: 'Hook', weight: 1, scale: 10 } as unknown as Axis
    const scored = (mark: number) => ({ axes: { hook: mark } }) as unknown as Score
    expect(axisBar(axis, scored(9.5)).shown).toBe('9.5')
    expect(axisBar(axis, scored(9)).shown).toBe('9')
  })
})
