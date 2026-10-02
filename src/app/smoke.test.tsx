import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { screen, within } from '@testing-library/react'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { CARD_TABS, SCREENS } from '@/test/places'
import { regionsHaveHeight } from '@/test/regions'
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
 * And each is held to the rules that make the window an application rather
 * than a web page (below, `keepsTheWindow`). They were checked as source text
 * by path until v0.77, which a move of `App.tsx` would have blinded; here
 * they are checked on what every screen actually draws.
 */

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
  return { area, screen: keepsTheWindow(container, path) }
}

/** The content area beside the rail: there once the workspace has answered. */
async function waitForArea(container: HTMLElement): Promise<HTMLElement> {
  await vi.waitFor(() => {
    if (container.querySelector('#main-area') === null) throw new Error('no content area yet')
  })
  return container.querySelector<HTMLElement>('#main-area')!
}

/**
 * The window has a bottom edge, and the content stops at it.
 *
 * The box the screens are drawn into does not scroll: it clips, and hands its
 * height down. When it scrolled, a long screen ran past the edge the way a
 * web page does - no end in sight, and a wide table's sideways bar parked
 * under two hundred rows. Since v0.78 no screen scrolls as a whole either:
 * each `<Screen>` (a `<main>`) is held against the height it was given, and
 * what it draws takes that height and scrolls inside - a head that stands, a
 * part that moves (`components/frame`). Until then five screens still flowed,
 * and their heads went up with their content.
 *
 * A held box clips, so a root that grows with its content instead of taking
 * the height would be cut off at the window's edge with nothing to scroll
 * it. That is the second half of the rule: the root hands the height on.
 */
function keepsTheWindow(container: HTMLElement, path: string): HTMLElement {
  const areas = container.querySelectorAll<HTMLElement>('[data-screen-area]')
  expect(areas, `${path}: one screen area`).toHaveLength(1)
  const area = areas[0]!
  expect(area, `${path}: the screen area scrolls again; it must clip`).toHaveClass(
    'overflow-hidden',
  )
  expect(area, `${path}: the screen area stopped handing its height down`).toHaveClass(
    'min-h-0',
    'flex-1',
  )

  const screens = area.querySelectorAll<HTMLElement>('main')
  expect(screens, `${path}: drawn through one <Screen>`).toHaveLength(1)
  const main = screens[0]!
  expect(main, `${path}: the screen does not hold its height`).toHaveClass('overflow-hidden')
  expect(main, `${path}: the screen scrolls as a whole again`).not.toHaveClass('overflow-y-auto')
  takesTheHeight(main, `${path}: the screen`)
  return main
}

/** What a held box draws takes the height it is given, rather than growing
 *  past it into the clip. */
function takesTheHeight(box: HTMLElement, what: string) {
  const roots = [...box.children].filter((child) => child.tagName !== 'SCRIPT')
  expect(roots, `${what} draws one root`).toHaveLength(1)
  expect(
    roots[0],
    `${what}'s root grows with its content instead of taking the height`,
  ).toHaveClass('min-h-0', 'flex-1')
}

describe('every screen opens', () => {
  for (const [path, shows] of SCREENS) {
    test(path, async () => {
      const { screen: main } = await open(path, shows)
      regionsHaveHeight(main, path)
    })
  }

  test('a note is read in text that can be copied', async () => {
    // Selection is off across the shell and handed back to text; the prose
    // renderer is where a note's body gets it back.
    await open(`/notes/${IDS.note}`, 'Something about the tide going out')
    const body = screen.getByText('Something about the tide going out', { exact: false })
    expect(body.closest('.selectable'), 'a note body that cannot be copied').not.toBeNull()
  })
})

describe('every tab of a card opens', () => {
  for (const [workId, tab, shows] of CARD_TABS) {
    test(`${workId}, on ${tab}`, async () => {
      const { area, screen: main } = await open(`/works/${workId}/${tab}`, shows)
      // The address named this tab; the bar must agree, or a tab the kind
      // does not have fell back to another and the case tested that one.
      const current = within(area).getByRole('link', { current: 'page' })
      expect(current).toHaveAttribute('href', `/works/${workId}/${tab}`)

      // The card is held: its header stands still and the open tab scrolls
      // inside the rest. A flowing card scrolls the header away again, and a
      // `sticky` header is the sign someone wrote the old shape back in.
      expect(main, 'the open work is no longer a held screen').toHaveClass('overflow-hidden')

      // And the tab does not scroll inside the card as a page: the box it is
      // drawn into clips, and the tab takes its height and scrolls inside.
      const body = main.querySelector<HTMLElement>('[data-tab-body]')
      expect(body, 'the card has no tab body').not.toBeNull()
      expect(body, 'the tab body scrolls as a whole again').toHaveClass('overflow-hidden')
      expect(body).not.toHaveClass('overflow-y-auto')
      takesTheHeight(body!, `the ${tab} tab`)
      regionsHaveHeight(body!, `the ${tab} tab`)

      const header = main.querySelector('header')
      expect(header, 'the card has no header').not.toBeNull()
      expect(header!.closest('.sticky, [class*="sticky"]'), 'the card header is sticky').toBeNull()
    })
  }
})
