import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Link, Made, Release, StartedTask, Work } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, coverOf, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A song goes out as what is made from it (v0.86, ADR 0047), and its board
 * leads with those: on the fake studio "Paper Lanterns" has a clip, with a
 * release of its own and nothing dated, and an audio booked for 22 September,
 * the studio's day being the 15th. The comment in the studio is under the
 * clip, waiting.
 */

/** What making an audio from the song makes, in the studio's words. */
const MADE = 'w-lanterns-audio-2'
const MADE_RELEASE = 'r-lanterns-audio-2'

let workspace: Studio
let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend(answersFor(workspace))
})

afterEach(() => {
  vi.useRealTimers()
})

/** The publications of `workId`'s overview, once the board has landed. */
async function openPublications(workId: string): Promise<HTMLElement> {
  const { client } = renderApp(`/works/${workId}/overview`)
  await settled(client)
  const main = await screen.findByRole('main')
  const widget = await within(main).findByRole('group', { name: en.publications.caption })
  await settled(client)
  return widget
}

/**
 * Answer `derive_work` as the backend would for an audio made from the song:
 * the work, its link to the song and one release through its first door with
 * no day - written into the studio, so the card it opens can read them.
 */
function answerDerive() {
  backend.answer('derive_work', ({ sourceId, kind }) => {
    const source = workspace.works.find((w) => w.id === sourceId)!
    const work: Work = {
      ...workspace.works.find((w) => w.id === IDS.audio)!,
      id: MADE,
      kind: kind as string,
      title: `${source.title} — audio 2`,
      status: 'draft',
      meta: { bpm: '96', variant: 'original' },
      cover: coverOf(),
    }
    const link: Link = {
      ...workspace.links.find((l) => l.id === IDS.audioLink)!,
      id: 'l-audio-2',
      work_id: MADE,
    }
    const release: Release = {
      ...workspace.releases.find((r) => r.id === IDS.audioRelease)!,
      id: MADE_RELEASE,
      work_id: MADE,
      scheduled_at: null,
    }
    workspace.works.push(work)
    workspace.links.push(link)
    workspace.releases.push(release)
    const made: Made = { work, release_id: MADE_RELEASE, door: release.kind }
    return made
  })
}

describe("a song's publications", () => {
  it('lists the clip and the audio, each with the fact it stands on', async () => {
    const widget = await openPublications(IDS.song)

    const clip = within(widget).getByRole('link', { name: /^Paper Lanterns \(clip\)/ })
    expect(clip).toHaveAttribute('href', `/works/${IDS.video}`)
    // Nothing out and nothing dated: the status word says where it stands.
    expect(within(clip).getByText('Video')).toBeInTheDocument()
    expect(within(clip).getByText('Draft')).toBeInTheDocument()
    // Its release's own door, not every door the kind has.
    expect(within(clip).getByText('YouTube · 16:9 · 1 comment')).toBeInTheDocument()

    const audio = within(widget).getByRole('link', { name: /^Paper Lanterns — audio/ })
    expect(audio).toHaveAttribute('href', `/works/${IDS.audio}`)
    expect(within(audio).getByText('Audio')).toBeInTheDocument()
    expect(within(audio).getByText('Booked for Sep 22')).toBeInTheDocument()
    expect(within(audio).getByText('YouTube · 16:9')).toBeInTheDocument()

    // Under the list: what the song stands on, and what was said under all of it.
    expect(
      within(widget).getByText('Booked as “Paper Lanterns — audio” for Sep 22'),
    ).toBeInTheDocument()
    expect(
      within(widget).getByRole('link', { name: '1 comment under its publications · 1 waiting' }),
    ).toHaveAttribute('href', `/works/${IDS.song}/comments`)
  })

  it('says what a publication went out as, and one that is late', async () => {
    const clip = workspace.releases.find((r) => r.id === IDS.youtube)!
    clip.status = 'released'
    clip.released_at = '2026-09-02T09:00:00Z'
    workspace.releases.find((r) => r.id === IDS.audioRelease)!.scheduled_at = '2026-09-10'

    const widget = await openPublications(IDS.song)

    const out = within(widget).getByRole('link', { name: /^Paper Lanterns \(clip\)/ })
    expect(within(out).getByText('Out Sep 2')).toBeInTheDocument()
    const late = within(widget).getByRole('link', { name: /^Paper Lanterns — audio/ })
    expect(within(late).getByText('Late: due Sep 10')).toBeInTheDocument()
  })

  it('invites the first publication where nothing is made yet, with the same menu', async () => {
    const widget = await openPublications(IDS.draft)

    expect(within(widget).getByText(en.publications.none)).toBeInTheDocument()
    fireEvent.click(within(widget).getByRole('button', { name: en.publications.makeMenu }))
    const menu = await screen.findByRole('menu')
    // A kind with several places is a group of one item per place; one with a
    // single place is one item.
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map((item) => item.textContent),
    ).toEqual([
      'YouTube · 16:9',
      'Premiere · 16:9',
      'YouTube · 16:9',
      'Streaming · 1:1',
      'Make a shortShort · 9:16',
    ])
    expect(within(menu).getByRole('group', { name: 'Make a video' })).toBeInTheDocument()
    expect(within(menu).getByRole('group', { name: 'Make an audio' })).toBeInTheDocument()
  })
})

describe('making a publication', () => {
  it('makes it in the window’s language, starts its release meta, and opens its cover', async () => {
    answerDerive()
    const started: StartedTask = {
      chatId: 'chat-meta',
      runId: 'run-meta',
      taskKey: `release-meta:release:${MADE_RELEASE}`,
      title: 'Release meta',
    }
    backend.answer('start_release_task', () => started)
    const widget = await openPublications(IDS.song)

    fireEvent.click(within(widget).getByRole('button', { name: en.publications.makeMenu }))
    const menu = await screen.findByRole('menu')
    const audio = within(menu).getByRole('group', { name: 'Make an audio' })
    fireEvent.click(within(audio).getByRole('menuitem', { name: /^YouTube/ }))

    await waitFor(() =>
      expect(backend.argsOf('start_release_task')).toEqual([
        { id: MADE_RELEASE, action: 'release-meta' },
      ]),
    )
    expect(backend.argsOf('derive_work')).toEqual([
      { sourceId: IDS.song, kind: 'audio', title: null, locale: 'en', door: 'youtube' },
    ])

    // The new work's card, open on its cover: the next decision is there.
    await screen.findByRole('heading', { level: 1, name: /^Paper Lanterns — audio 2/ })
    const tabs = await screen.findByRole('navigation', { name: en.card.tabs })
    await waitFor(() =>
      expect(within(tabs).getByRole('link', { current: 'page' })).toHaveAttribute(
        'href',
        `/works/${MADE}/cover`,
      ),
    )
    expect(await screen.findByText('Made “Paper Lanterns — audio 2”')).toBeInTheDocument()
    expect(
      screen.getByText(
        'Linked, with a YouTube release that has no day yet. Its meta is being written in the background.',
      ),
    ).toBeInTheDocument()
  })

  it('keeps the work made when its release meta cannot start, and says so', async () => {
    answerDerive()
    backend.answer('start_release_task', () => Promise.reject(new Error('no assistant here')))
    const widget = await openPublications(IDS.song)

    fireEvent.click(within(widget).getByRole('button', { name: en.publications.makeMenu }))
    const menu = await screen.findByRole('menu')
    const audio = within(menu).getByRole('group', { name: 'Make an audio' })
    fireEvent.click(within(audio).getByRole('menuitem', { name: /^YouTube/ }))

    expect(await screen.findByText(en.publications.metaNotStarted)).toBeInTheDocument()
    expect(
      screen.getByText('Linked, with a YouTube release that has no day yet.'),
    ).toBeInTheDocument()
    await screen.findByRole('heading', { level: 1, name: /^Paper Lanterns — audio 2/ })
  })
})

describe('the places of a kind', () => {
  it('offers the audio its two places, and makes it for the one picked', async () => {
    answerDerive()
    backend.answer('start_release_task', () => Promise.reject(new Error('no assistant here')))
    const widget = await openPublications(IDS.song)

    fireEvent.click(within(widget).getByRole('button', { name: en.publications.makeMenu }))
    const menu = await screen.findByRole('menu')
    const audio = within(menu).getByRole('group', { name: 'Make an audio' })
    expect(
      within(audio)
        .getAllByRole('menuitem')
        .map((item) => item.textContent),
    ).toEqual(['YouTube · 16:9', 'Streaming · 1:1'])
    fireEvent.click(within(audio).getByRole('menuitem', { name: /^Streaming/ }))

    await waitFor(() =>
      expect(backend.argsOf('derive_work')).toEqual([
        { sourceId: IDS.song, kind: 'audio', title: null, locale: 'en', door: 'streaming' },
      ]),
    )
  })
})
