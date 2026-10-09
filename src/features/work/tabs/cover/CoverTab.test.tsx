import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Asset, CoverView, Scheme, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, coverOf, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The cover constructor (v0.88): what a cover is built from on the left,
 * the prompt written from it on the right. The prompt itself is the
 * backend's (`cover_view`), so these hold the window to what it sends: the
 * whole cover with each choice, the shape it asks the prompt for, the copy
 * it records, the picture it makes the cover.
 */

let backend: Backend
let workspace: Studio

const SCHEME: Scheme = {
  width: 100,
  height: 100,
  colours: { background: '#EFEBE3', figure: '#57525F', ink: '#121114', accent: '#9D9A94' },
  shapes: [{ shape: 'rect', rect: { x: 0, y: 0, w: 100, h: 100 }, paint: 'background' }],
  hero: { x: 30, y: 20, w: 40, h: 60 },
  zones: [],
  mark: null,
}

/** The view of a built cover, with the layouts and a detail to switch. */
function builtView(over: Partial<CoverView> = {}): CoverView {
  return {
    built: true,
    prompts: { picture: 'FRAME: a wide 16:9 cover.', negative: 'No watermark.', typography: null },
    format: '16:9',
    formats: [
      { door: 'youtube', label: 'YouTube', format: '16:9', held: true },
      { door: 'streaming', label: 'Streaming', format: '1:1', held: false },
    ],
    scheme: SCHEME,
    layouts: [
      {
        framing: {
          layout: 'emblemRight',
          column: 'right',
          row: 'middle',
          size: 'emblem',
          crop: 'bust',
          place: 'left',
        },
        scheme: SCHEME,
      },
    ],
    title: 'Paper Lanterns',
    source_title: 'Paper Lanterns',
    slots: [],
    details: [
      {
        id: 'detail-cat',
        name: 'A cat hides in the picture',
        template: 'a small black cat hides somewhere',
        places: ['cover', 'frame'],
        default_on: true,
        on: true,
      },
    ],
    marks: [],
    palette: [],
    house_styles: [],
    hero: null,
    references: [],
    problems: [],
    sent_differs: false,
    mark_box: null,
    ...over,
  }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => {
      const work = workspace.works.find((one) => one.id === id)!
      Object.assign(work, patch as WorkPatch)
      return work
    },
    choose_cover: ({ id }) => ({ id }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openCover(workId: string) {
  const { client } = renderApp(`/works/${workId}/cover`)
  await screen.findByRole('region', { name: en.cover.idea.title })
  await settled(client)
}

/** The covers the window sent, oldest first. */
const sent = () =>
  backend
    .argsOf('update_work')
    .map((args) => (args.patch as WorkPatch).cover)
    .filter((cover) => cover !== undefined)

describe('a cover written by hand', () => {
  it('is its own words, and an edit sends the whole cover with them', async () => {
    await openCover(IDS.audio)
    expect(screen.getByText(en.cover.prompt.byHand)).toBeInTheDocument()

    const box = screen.getByRole('textbox', { name: en.cover.prompt.part.picture })
    expect(box).toHaveValue('a paper lantern on dark water, seen from above')
    fireEvent.focus(box)
    fireEvent.change(box, { target: { value: 'a paper lantern at dawn' } })
    fireEvent.blur(box)

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(sent()[0]).toEqual(coverOf({ picture: 'a paper lantern at dawn' }))
  })
})

describe('the constructor', () => {
  it('sets the frame a layout starts from when one is picked', async () => {
    backend.answer('cover_view', () => builtView())
    await openCover(IDS.audio)

    fireEvent.click(screen.getByRole('button', { name: en.cover.layout.emblemRight }))

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(sent()[0]?.framing).toEqual(builtView().layouts[0]!.framing)
    // The rest of the cover travels with it, as it was.
    expect(sent()[0]?.picture).toBe('a paper lantern on dark water, seen from above')
  })

  it('keeps what the scene keeps out on the cover, beside the scene', async () => {
    backend.answer('cover_view', () => builtView())
    await openCover(IDS.audio)

    const box = screen.getByRole('textbox', { name: en.cover.avoid.title })
    fireEvent.focus(box)
    fireEvent.change(box, { target: { value: 'gallows, blood on the snow' } })
    fireEvent.blur(box)

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(sent()[0]?.avoid).toBe('gallows, blood on the snow')
    // The person's own negative is a word of its own and stays as it was.
    expect(sent()[0]?.negative).toBe('')
  })

  it('asks for the prompt in the shape picked among the doors', async () => {
    backend.answer('cover_view', ({ format }) =>
      builtView({ format: (format as string | null) ?? '16:9' }),
    )
    await openCover(IDS.audio)

    fireEvent.click(screen.getByRole('radio', { name: /1:1/ }))

    await waitFor(() =>
      expect(backend.argsOf('cover_view').map((args) => args.format)).toContain('1:1'),
    )
    expect(await screen.findByText(en.cover.formatNotHeld)).toBeInTheDocument()
  })

  // v0.90.3: accents of the dictionary sit beside the channel's palette, and
  // a gradient one lands on the cover with every stop.
  it('offers the accents of the dictionary beside the palette, and keeps a gradient whole', async () => {
    workspace.bricks.push({
      trial_id: null,
      id: 'accent-sunset',
      profile_id: IDS.profile,
      type_key: 'accent',
      name: 'Sunset gradient',
      description: 'ACCENT: a gradient from coral (#FF5F6D) to amber (#FFC371).',
      hint: null,
      status: 'ready',
      created_at: NOW,
      updated_at: NOW,
      reference_count: 0,
      label: { en: 'Sunset gradient', ru: 'Закатный градиент' },
      family: null,
      when_to_use: null,
      explanation: null,
      colours: ['#FF5F6D', '#FFC371'],
      sample: null,
      set_key: 'accent-grad-sunset',
      origin: 'set',
    })
    backend.answer('cover_view', () =>
      builtView({ palette: [{ id: 'fact-pink', name: 'Hot pink', color: '#FF2E63' }] }),
    )
    await openCover(IDS.audio)

    expect(screen.getByRole('list', { name: en.cover.accent.fromChannel })).toBeInTheDocument()
    fireEvent.click(
      within(screen.getByRole('list', { name: en.cover.accent.fromDictionary })).getByRole(
        'button',
        { name: 'Sunset gradient · #FF5F6D → #FFC371' },
      ),
    )

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(sent()[0]?.accent).toEqual({
      name: 'Sunset gradient',
      color: '#FF5F6D',
      stops: ['#FF5F6D', '#FFC371'],
    })
  })

  it('remembers a detail switched off on this cover, the channel keeping its own', async () => {
    backend.answer('cover_view', () => builtView())
    await openCover(IDS.audio)

    fireEvent.click(screen.getByRole('switch', { name: 'A cat hides in the picture' }))

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(sent()[0]?.details).toEqual({ 'detail-cat': false })
  })

  it('keeps the prompt on the cover when it is copied out', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    backend.answer('cover_view', () => builtView())
    await openCover(IDS.audio)

    fireEvent.click(screen.getByRole('button', { name: en.cover.prompt.copyAll }))

    await waitFor(() => expect(sent()).toHaveLength(1))
    expect(writeText).toHaveBeenCalledWith(
      'FRAME: a wide 16:9 cover.\n\n---\n\nNEGATIVE: No watermark.',
    )
    expect(sent()[0]?.sent).toMatchObject({
      picture: 'FRAME: a wide 16:9 cover.',
      negative: 'No watermark.',
      typography: null,
    })
  })
})

describe('the result', () => {
  it('shows the cover apart from the candidates, and makes a candidate the cover', async () => {
    const picture = (id: string, kind: string, created_at: string): Asset => ({
      trial_id: null,
      id,
      profile_id: IDS.profile,
      work_id: IDS.audio,
      release_id: null,
      kind,
      path: `C:/media/${id}.png`,
      label: null,
      original_name: `${id}.png`,
      style_brick_id: null,
      note_id: null,
      canon_fact_id: null,
      created_at,
    })
    backend.answer('list_work_assets', () => [
      picture('first', 'cover', '2026-09-01T10:00:00Z'),
      picture('second', 'candidate', '2026-09-02T10:00:00Z'),
    ])
    await openCover(IDS.audio)

    const result = await screen.findByRole('region', { name: en.cover.result.title })
    expect(await within(result).findByText(en.cover.result.final)).toBeInTheDocument()
    fireEvent.click(within(result).getByRole('button', { name: en.cover.result.choose }))

    await waitFor(() => expect(backend.argsOf('choose_cover')).toEqual([{ id: 'second' }]))
  })
})
