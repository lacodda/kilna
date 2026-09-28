import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import type { Deletion } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The trash's actions are words, and a restore that cannot happen says why.
 *
 * Until v0.79 a row ended in two unnamed icons, and the one that could not
 * restore was a native disabled button whose tooltip nobody could raise: a
 * version whose work was also in the trash simply did nothing, with no word
 * about the work it was waiting for. These walk the rows against the mocked
 * backend only - purging for real deletes the files a work names.
 */

const GONE = 'w-low-tide'

/** A work in the trash, and a version of it that cannot come back before it. */
const trashed: Deletion[] = [
  {
    id: 'd-work',
    entity: 'work',
    entity_id: GONE,
    label: 'Low Tide',
    origin: null,
    work_id: GONE,
    reason: 'manual',
    deleted_at: NOW,
    restorable: true,
  },
  {
    id: 'd-draft',
    entity: 'version',
    entity_id: 'v-low-tide-3',
    label: 'Third pass',
    origin: 'Low Tide',
    work_id: GONE,
    reason: 'manual',
    deleted_at: NOW,
    restorable: false,
  },
]

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  const workspace = studio()
  workspace.deletions = [...trashed, ...workspace.deletions]
  backend = mockBackend({
    ...answersFor(workspace),
    restore_deletion: () => null,
    purge_deletion: () => null,
  })
})

afterEach(() => {
  vi.useRealTimers()
})

/** The table row that names `label`. */
async function rowOf(label: string): Promise<HTMLElement> {
  const main = await screen.findByRole('main')
  const cell = await within(main).findByText(label, { exact: true })
  return cell.closest('tr')!
}

describe('the trash', () => {
  it('says where a version came from, and why it cannot come back yet', async () => {
    const { client } = renderApp('/trash')
    await settled(client)
    const row = await rowOf('Third pass')

    expect(within(row).getByText('· from “Low Tide”')).toBeInTheDocument()
    const restore = within(row).getByRole('button', { name: en.trash.restore })
    // Reachable - by Tab and by the pointer - and inert, with the reason as
    // its description rather than a tooltip on a dead button.
    expect(restore).not.toBeDisabled()
    expect(restore).toHaveAttribute('aria-disabled', 'true')
    expect(restore).toHaveAccessibleDescription(en.error.notRestorable)

    fireEvent.click(restore)
    await settled(client)
    expect(backend.argsOf('restore_deletion')).toEqual([])
  })

  it('restores a row that can come back', async () => {
    const { client } = renderApp('/trash')
    await settled(client)
    const row = await rowOf('Low Tide')

    fireEvent.click(within(row).getByRole('button', { name: en.trash.restore }))
    await settled(client)

    expect(backend.argsOf('restore_deletion')).toEqual([{ id: 'd-work' }])
  })

  it('asks before it deletes anything for good', async () => {
    const { client } = renderApp('/trash')
    await settled(client)
    const row = await rowOf('An old idea')

    fireEvent.click(within(row).getByRole('button', { name: en.trash.purge }))
    const question = await screen.findByRole('alertdialog')
    expect(question).toHaveTextContent('Delete “An old idea” for good?')
    expect(backend.argsOf('purge_deletion')).toEqual([])

    fireEvent.click(within(question).getByRole('button', { name: en.trash.purge }))
    await settled(client)
    expect(backend.argsOf('purge_deletion')).toEqual([{ id: 'd-old-note' }])
  })
})
