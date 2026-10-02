import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { mockBackend } from '@/test/backend'
import { noScrollerInAnother } from '@/test/regions'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The shell around every screen: the rail on dowel's NavRail, and the palette
 * with its Actions and Screens. The smoke test opens every screen through it;
 * this presses the shell itself - the parts a refactor of it could quietly
 * disconnect while every screen still opened.
 */

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  mockBackend(answersFor(studio()))
  // The rail's width and the theme are the machine's, kept in storage: each
  // test starts from a full rail and the system theme.
  localStorage.clear()
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
  document.documentElement.classList.remove('light', 'dark')
})

/** The window at `path`, once the rail is drawn. */
async function open(path: string) {
  const view = renderApp(path)
  await settled(view.client)
  await screen.findByRole('navigation', { name: en.nav.screens })
  return view
}

const rail = () => screen.getByRole('navigation', { name: en.nav.screens })

/** The palette, opened the way a hand opens it. */
async function palette() {
  act(() => {
    fireEvent.keyDown(document.body, { key: 'k', ctrlKey: true })
  })
  return screen.findByRole('dialog', { name: en.search.title })
}

describe('the rail', () => {
  it('lights the catalogue while a work is open', async () => {
    await open(`/works/${IDS.song}`)
    expect(within(rail()).getByRole('link', { name: en.nav.catalogue })).toHaveAttribute(
      'aria-current',
      'page',
    )
  })

  it('keeps every name when it folds to icons', async () => {
    await open('/dashboard')
    fireEvent.click(screen.getByRole('button', { name: en.shell.collapseMenu }))

    // The words leave the screen, not the entries: each is still named.
    expect(within(rail()).getByRole('link', { name: en.nav.dashboard })).toHaveAttribute(
      'aria-current',
      'page',
    )
    expect(screen.getByRole('button', { name: en.shell.expandMenu })).toBeInTheDocument()
  })

  it('switches the theme from its own entry, and names the new one', async () => {
    await open('/dashboard')
    const appearance = screen.getByRole('navigation', { name: en.nav.appearance })
    fireEvent.click(within(appearance).getByRole('button', { name: en.themeName.system }))

    expect(document.documentElement).toHaveClass('light')
    expect(within(appearance).getByRole('button', { name: en.themeName.light })).toBeInTheDocument()
  })
})

describe('the palette', () => {
  it('opened on nothing, lists what can be done and where to go', async () => {
    await open('/dashboard')
    const box = await palette()

    expect(within(box).getByText(en.search.group.action)).toBeInTheDocument()
    expect(within(box).getByText(en.search.group.screen)).toBeInTheDocument()
    expect(within(box).getByRole('option', { name: en.works.newTitle })).toBeInTheDocument()
    expect(within(box).getByRole('option', { name: en.nav.trash })).toBeInTheDocument()
  })

  it('scrolls its hits in one box, so the wheel reaches them', async () => {
    // A list too short to scroll, inside a column that did, took the wheel
    // and moved nothing until v0.90.1; only the bar moved the hits.
    await open('/dashboard')
    noScrollerInAnother(await palette(), 'the palette')
  })

  it('opens a screen found by its name', async () => {
    const { client } = await open('/dashboard')
    const box = await palette()
    fireEvent.change(within(box).getByRole('combobox'), { target: { value: 'cal' } })

    // Narrowed once the query settles: a screen whose name does not have the
    // letters goes, and the one that begins with them stays.
    await waitFor(() => {
      expect(within(box).queryByRole('option', { name: en.nav.trash })).toBeNull()
    })
    fireEvent.click(within(box).getByRole('option', { name: en.nav.calendar }))
    await settled(client)

    expect(screen.queryByRole('dialog', { name: en.search.title })).toBeNull()
    expect(within(rail()).getByRole('link', { name: en.nav.calendar })).toHaveAttribute(
      'aria-current',
      'page',
    )
  })

  it('offers the open work its score, and the shortcuts among its actions', async () => {
    await open(`/works/${IDS.song}`)
    const box = await palette()

    const [scoring] = en.search.action.score.split('{{title}}')
    expect(
      within(box).getByRole('option', { name: (name) => name.startsWith(scoring!) }),
    ).toBeInTheDocument()
    fireEvent.click(within(box).getByRole('option', { name: en.keys.title }))

    expect(await screen.findByRole('dialog', { name: en.keys.title })).toBeInTheDocument()
  })
})
