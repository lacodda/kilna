import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import type { Tab } from '@/components/card/tabs'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Every screen and every tab of a card, opened once on a small studio.
 *
 * Not a test of what any of them does - a test that each one opens: that it
 * asks the backend only questions the backend answers, draws what the studio
 * holds, falls into no crash panel, and says nothing to the console. The
 * refactor this net was laid for moves every file of the frontend; a screen
 * that stops opening is the failure it has to catch the moment it happens,
 * rather than on the owner's machine.
 *
 * Each case names a line of the studio the screen must show. Without one, a
 * screen stuck on its skeleton - or the splash - would pass every other check.
 */

const SCREENS: [path: string, shows: string][] = [
  ['/dashboard', 'Harbour Lights'],
  ['/catalogue', 'Harbour Lights'],
  ['/calendar', 'Paper Lanterns (clip)'],
  ['/notes', 'A song about tides'],
  [`/notes/${IDS.note}`, 'Something about the tide going out and taking the day with it.'],
  ['/comments', 'The shot on the bridge is beautiful.'],
  [`/comments/${IDS.comment}`, 'Paper Lanterns (clip)'],
  ['/styles', 'Dusk over water'],
  ['/journal', '“Harbour Lights” added.'],
  ['/trash', 'An old idea'],
  ['/settings', en.nav.data],
]

/** The tabs a kind draws, each with a line of the studio it must show. */
const TABS: [workId: string, tab: Tab, shows: string][] = [
  [IDS.song, 'overview', 'BPM'],
  [IDS.song, 'versions', 'Second pass'],
  [IDS.song, 'score', 'Does the chorus stay with you after one listen?'],
  [IDS.song, 'releases', '2026-09-22'],
  [IDS.song, 'files', 'Paper Lanterns'],
  [IDS.song, 'links', 'Paper Lanterns (clip)'],
  [IDS.song, 'notes', 'Paper Lanterns'],
  [IDS.song, 'comments', 'Paper Lanterns'],
  [IDS.song, 'assistant', 'Tighten the chorus'],
  [IDS.song, 'history', '“Paper Lanterns” scored 7.5.'],
  [IDS.video, 'overview', 'Paper Lanterns (clip)'],
  [IDS.video, 'versions', 'A girl lets a paper lantern go at dusk'],
  [IDS.video, 'scenes', 'The lantern drifts under the bridge.'],
  [IDS.video, 'cuts', 'Paper Lanterns'],
  [IDS.video, 'score', 'Paper Lanterns (clip)'],
  [IDS.video, 'releases', 'Paper Lanterns (clip)'],
  [IDS.video, 'files', 'Paper Lanterns (clip)'],
  [IDS.video, 'links', 'soundtrack'],
  [IDS.video, 'notes', 'An old man who sells lanterns by the bridge.'],
  [IDS.video, 'comments', 'The shot on the bridge is beautiful.'],
  [IDS.video, 'assistant', 'Paper Lanterns (clip)'],
  [IDS.video, 'history', 'Paper Lanterns (clip)'],
  [IDS.short, 'cuts', 'Paper Lanterns (clip)'],
]

let backend: Backend
let complaints: string[]

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend(answersFor(studio()))
  complaints = []
  // React reports a broken render, a missing key or an update on an
  // unmounted component here, and nowhere else.
  vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => {
    complaints.push(args.map(String).join(' '))
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** Open `path` and hold it to the checks every screen shares. */
async function open(path: string, shows: string) {
  const { client, container } = renderApp(path)
  await settled(client)
  const area = await waitForArea(container)
  await within(area).findAllByText(shows, { exact: false })
  await settled(client)

  expect(backend.unanswered, `${path} asked what the test backend does not answer`).toEqual([])
  expect(screen.queryByText(en.crash.title), `${path} fell over`).toBeNull()
  expect(complaints, `${path} complained to the console`).toEqual([])
  return area
}

/** The content area beside the rail: there once the workspace has answered. */
async function waitForArea(container: HTMLElement): Promise<HTMLElement> {
  await vi.waitFor(() => {
    if (container.querySelector('#main-area') === null) throw new Error('no content area yet')
  })
  return container.querySelector<HTMLElement>('#main-area')!
}

describe('every screen opens', () => {
  for (const [path, shows] of SCREENS) test(path, () => open(path, shows))
})

describe('every tab of a card opens', () => {
  for (const [workId, tab, shows] of TABS) {
    test(`${workId}, on ${tab}`, async () => {
      const area = await open(`/works/${workId}/${tab}`, shows)
      // The address named this tab; the bar must agree, or a tab the kind
      // does not have fell back to another and the case tested that one.
      const current = within(area).getByRole('link', { current: 'page' })
      expect(current).toHaveAttribute('href', `/works/${workId}/${tab}`)
    })
  }
})
