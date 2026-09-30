import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Frame, WorkPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Frame tab (v0.86): the still and the loop an audio release plays
 * under. Every change sends the whole frame; the prompts on the right are the
 * backend's, each copied on its own.
 */

let backend: Backend
let workspace: Studio

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
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The frame as the studio holds it before a test changes anything. */
const STORED: Frame = studio().works.find((work) => work.id === IDS.audio)!.frame

async function openFrame() {
  const { client } = renderApp(`/works/${IDS.audio}/frame`)
  await screen.findByRole('textbox', { name: en.frame.still })
  await settled(client)
  return client
}

/** The frames sent, oldest first. */
const sent = () =>
  backend.argsOf('update_work').map((args) => (args.patch as WorkPatch).frame as Frame)

describe('an edit of the frame', () => {
  it('sends the whole frame with the text changed, on blur, and only when it changed', async () => {
    await openFrame()
    const still = screen.getByRole('textbox', { name: en.frame.still })

    // Through the box and out again: nothing was written.
    fireEvent.focus(still)
    fireEvent.blur(still)
    expect(backend.argsOf('update_work')).toEqual([])

    fireEvent.change(still, { target: { value: 'a lantern caught in reeds' } })
    // Typing writes nothing yet: a save per keystroke writes half-words.
    expect(backend.argsOf('update_work')).toEqual([])
    fireEvent.blur(still)

    await waitFor(() => expect(sent()).toEqual([{ ...STORED, still: 'a lantern caught in reeds' }]))
    expect(backend.argsOf('update_work')[0]!.id).toBe(IDS.audio)
  })

  it('sends a switch and a length at once, each over the change before it', async () => {
    const client = await openFrame()

    fireEvent.click(screen.getByRole('checkbox', { name: en.frame.stillCamera }))
    await waitFor(() => expect(sent()).toHaveLength(1))
    await settled(client)
    fireEvent.click(screen.getByRole('radio', { name: '8 s' }))

    await waitFor(() => expect(sent()).toHaveLength(2))
    expect(sent()).toEqual([
      { ...STORED, still_camera: false },
      { ...STORED, still_camera: false, seconds: 8 },
    ])
  })

  it('keeps two switches pressed in a row, the second carrying the first', async () => {
    await openFrame()

    fireEvent.click(screen.getByRole('checkbox', { name: en.frame.stillCamera }))
    fireEvent.click(screen.getByRole('checkbox', { name: en.frame.seamless }))

    await waitFor(() => expect(sent()).toHaveLength(2))
    expect(sent()[1]).toEqual({ ...STORED, still_camera: false, seamless: false })
  })

  it('shows a length it does not offer as the one chosen', async () => {
    workspace.works.find((work) => work.id === IDS.audio)!.frame.seconds = 5
    await openFrame()

    const length = screen.getByRole('radiogroup', { name: en.frame.length })
    expect(
      within(length)
        .getAllByRole('radio')
        .map((one) => one.textContent),
    ).toEqual(['4 s', '5 s', '6 s', '8 s', '10 s'])
    expect(within(length).getByRole('radio', { name: '5 s' })).toHaveAttribute(
      'aria-checked',
      'true',
    )
  })
})

describe("the frame's prompts", () => {
  it('are the three blocks the backend writes, under the shape of the first door', async () => {
    await openFrame()

    expect(screen.getByText(en.frame.shape.replace('{{format}}', '16:9'))).toBeInTheDocument()
    const still = await screen.findByRole('region', { name: en.frame.prompt.still })
    expect(
      within(still).getByText('a paper lantern on dark water at dusk, seen from above'),
    ).toBeInTheDocument()
    const loop = screen.getByRole('region', { name: en.frame.prompt.loop })
    expect(
      within(loop).getByText('LOOP (6 s): the lantern turns slowly on the current.'),
    ).toBeInTheDocument()
    const negative = screen.getByRole('region', { name: en.frame.prompt.negative })
    expect(within(negative).getByText('no people')).toBeInTheDocument()
  })

  it('are each copied on their own', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    await openFrame()

    const name = en.frame.copy.replace('{{part}}', en.frame.prompt.loop)
    fireEvent.click(await screen.findByRole('button', { name }))

    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(
        'LOOP (6 s): the lantern turns slowly on the current.',
      ),
    )
  })

  it('are asked for again once the frame is saved', async () => {
    const client = await openFrame()
    const before = backend.argsOf('frame_prompts').length

    const motion = screen.getByRole('textbox', { name: en.frame.motion })
    fireEvent.change(motion, { target: { value: 'the reeds sway' } })
    fireEvent.blur(motion)

    await waitFor(() => expect(sent()).toHaveLength(1))
    await settled(client)
    expect(backend.argsOf('frame_prompts').length).toBeGreaterThan(before)
    expect(await screen.findByText('LOOP (6 s): the reeds sway.')).toBeInTheDocument()
  })
})
