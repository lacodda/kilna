import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Release, ReleasePatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The calendar's release dialog (v0.80): the Releases tab's form, written
 * with a Save.
 *
 * Everything the form holds waits for Save, the pin included - until v0.80
 * the pin was written the moment it was ticked while the date beside it
 * waited, and Cancel took back one and not the other. The kinds offered are
 * the work's own, and an action pressed with an edit in the form keeps the
 * edit rather than throwing it away.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  // Wider than `Partial<Release>`: a patch's `Option<Option<T>>` fields
  // accept an explicit `null` to clear them, which a plain `Partial` does not.
  const kept = (id: unknown, patch: { [K in keyof Release]?: Release[K] | null }) => {
    const release = workspace.releases.find((r) => r.id === id)!
    Object.assign(release, patch)
    return release satisfies Release
  }
  backend = mockBackend({
    ...answersFor(workspace),
    update_release: ({ id, patch }) => kept(id, patch as ReleasePatch),
    set_slot_pin: ({ id, pinned }) => kept(id, { slot_pinned_at: pinned === true ? NOW : null }),
    unschedule_release: ({ id }) => kept(id, { scheduled_at: null, slot_pinned_at: null }),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

/** The calendar, with the booked song's release open in its dialog. */
async function editing() {
  const { client } = renderApp('/calendar')
  await settled(client)
  const main = await screen.findByRole('main')
  fireEvent.click(await within(main).findByText('Paper Lanterns', { exact: true }))
  const dialog = await screen.findByRole('dialog', { name: 'Paper Lanterns' })
  return { client, dialog }
}

describe("the calendar's release dialog", () => {
  it('offers only the kinds its own work goes out as', async () => {
    const { dialog } = await editing()

    fireEvent.click(within(dialog).getByRole('combobox', { name: en.releases.kind }))
    const offered = (await screen.findAllByRole('option')).map((option) => option.textContent)
    // A song's one door - not the clip's YouTube or premiere.
    expect(offered).toEqual(['Audio release'])
  })

  it('keeps the date only when Save is pressed, with the rest of the form', async () => {
    const { client, dialog } = await editing()

    fireEvent.click(
      within(dialog).getByRole('checkbox', {
        name: (name) => name.startsWith(en.calendar.pinSlot),
      }),
    )
    await settled(client)
    expect(backend.argsOf('set_slot_pin')).toEqual([])

    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: en.dialog.save }))
    })
    await settled(client)

    // The pin alone moved: nothing else is written.
    expect(backend.argsOf('set_slot_pin')).toEqual([{ id: IDS.audio, pinned: true }])
    expect(backend.argsOf('update_release')).toEqual([])
    // The toast that says so is a dialog too; the editor is the one named for
    // the work.
    expect(screen.queryByRole('dialog', { name: 'Paper Lanterns' })).toBeNull()
  })

  it('keeps a link typed before the release is returned to the queue', async () => {
    const { client, dialog } = await editing()

    fireEvent.change(within(dialog).getByRole('textbox', { name: en.calendar.urlPrompt }), {
      target: { value: 'https://example.com/listen' },
    })
    await act(async () => {
      fireEvent.click(within(dialog).getByRole('button', { name: en.calendar.unschedule }))
    })
    await settled(client)

    expect(backend.argsOf('update_release')).toEqual([
      { id: IDS.audio, patch: { url: 'https://example.com/listen' } },
    ])
    expect(backend.argsOf('unschedule_release')).toEqual([{ id: IDS.audio }])
  })
})
