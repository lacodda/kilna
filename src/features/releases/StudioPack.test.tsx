import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { Release, StudioPack as Pack } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import { StudioPack } from '@/features/releases/StudioPack'
import { isVideoLink } from '@/lib/api/studio'

/*
 * A batch put out on YouTube by hand (v0.159, FR-22 22.1 p.1): a step per
 * release, every field copied and ticked, the link checked before it is
 * kept, and on to the next.
 */

const RELEASE = {
  id: 'r-1',
  work_id: IDS.song,
  kind: 'youtube',
  status: 'planned',
  scheduled_at: '2027-07-15',
  released_at: null,
  url: null,
  slot_pinned_at: null,
  scheduled_time: '18:00',
  time_zone: 'America/New_York',
  meta: {},
  created_at: NOW,
  updated_at: NOW,
} as unknown as Release

const PACK: Pack = {
  items: [
    {
      release: RELEASE,
      work_title: 'Paper Lanterns',
      date: '2027-07-15',
      time: '18:00',
      time_zone: 'America/New_York',
      offset: '-04:00',
      offset_now: '-05:00',
      video: 'C:/media/w-lanterns/Paper Lanterns.mp4',
      cover: null,
      folder: 'C:/media/w-lanterns',
      fields: [
        {
          key: 'title',
          label: { en: 'Title' },
          field_type: 'text',
          value: 'Paper Lanterns (lyric video)',
          hint: null,
          limit: 100,
          has_template: false,
        },
      ] as unknown as Pack['items'][number]['fields'],
    },
  ],
  skipped: [{ work_id: IDS.draft, title: 'Harbour Lights', reason: 'noVideo' }],
}

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  Object.assign(navigator, { clipboard: { writeText: vi.fn(() => Promise.resolve()) } })
  backend = mockBackend({
    ...answersFor(studio()),
    studio_pack: () => PACK,
    studio_link: () => ({
      release: { ...RELEASE, url: 'https://youtu.be/dQw4w9WgXcQ' },
      released: false,
    }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('the YouTube Studio pack', () => {
  it('knows a link to a video from any other link', () => {
    expect(isVideoLink('https://youtu.be/dQw4w9WgXcQ')).toBe(true)
    expect(isVideoLink('youtube.com/shorts/dQw4w9WgXcQ')).toBe(true)
    expect(isVideoLink('https://youtube.com/@channel')).toBe(false)
  })

  it('walks a release: its fields copied and ticked, its link kept, then what was left out', async () => {
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <StudioPack workIds={[IDS.song, IDS.draft]} onClose={() => undefined} />
        </MemoryRouter>
      </Providers>,
    )
    const dialog = await screen.findByRole('dialog', { name: 'YouTube Studio · 1 of 1' })
    expect(within(dialog).getByText('Paper Lanterns')).toBeInTheDocument()
    expect(
      within(dialog).getByText('Summer time: that day the offset is -04:00, not -05:00 as today.'),
    ).toBeInTheDocument()

    fireEvent.click(within(dialog).getByRole('button', { name: 'Copy Title' }))
    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Paper Lanterns (lyric video)'),
    )
    expect(await within(dialog).findByText('Copied')).toBeInTheDocument()

    const link = within(dialog).getByRole('textbox', { name: 'The link YouTube gave' })
    fireEvent.change(link, { target: { value: 'https://youtube.com/@channel' } })
    expect(
      within(dialog).getByText('This is not a link to a video on YouTube.'),
    ).toBeInTheDocument()
    fireEvent.change(link, { target: { value: 'https://youtu.be/dQw4w9WgXcQ' } })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Keep the link and go on →' }))

    await waitFor(() => expect(backend.argsOf('studio_link')).toHaveLength(1))
    expect(backend.argsOf('studio_link')[0]).toEqual({
      releaseId: 'r-1',
      url: 'https://youtu.be/dQw4w9WgXcQ',
    })
    expect(await screen.findByText('Done: 1 link kept.')).toBeInTheDocument()
    expect(screen.getByText(/its video is not made yet/)).toBeInTheDocument()
  })
})
