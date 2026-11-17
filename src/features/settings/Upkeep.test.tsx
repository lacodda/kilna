import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Examination } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Looking after the space, in Settings → Data (v0.114): the copies kilna
 * keeps and putting one back, and the examination. The backend's own tests
 * hold what a restore and an examination do; these hold that the window asks
 * for the copy the person pointed at, only after they agreed, and says what
 * an examination found by name.
 */

let backend: Backend

const found: Examination = {
  damage: [],
  dangling: 0,
  ownerless: { count: 0, names: [] },
  lost: { count: 1, names: ['cover-final.png'] },
  stray: { count: 2, names: ['a.bin', 'b.bin'] },
  strayBytes: 2048,
}

beforeEach(() => {
  backend = mockBackend({
    ...answersFor(studio()),
    restore_backup: () => 'C:/workspace/backups/before-restore-2026-09-16T09-00-00.db',
    examine_workspace: () => found,
  })
})

async function openData(): Promise<void> {
  const { client } = renderApp('/settings/data')
  await settled(client)
  await screen.findByText(en.data.backups)
}

describe('looking after the space', () => {
  it('puts back the copy that was pointed at, once it is agreed', async () => {
    await openData()

    fireEvent.click(await screen.findByRole('button', { name: en.data.restoreThis }))
    // Asked first: nothing is sent by the press that opens the question.
    const question = await screen.findByRole('alertdialog')
    expect(backend.argsOf('restore_backup')).toHaveLength(0)

    fireEvent.click(within(question).getByRole('button', { name: en.data.restoreConfirm }))
    await waitFor(() => expect(backend.argsOf('restore_backup')).toHaveLength(1))
    expect(backend.argsOf('restore_backup')[0]).toEqual({
      path: 'C:/workspace/backups/daily-2026-09-15T08-00-00.db',
    })
  })

  it('names what an examination found', async () => {
    await openData()

    fireEvent.click(screen.getByRole('button', { name: en.data.examineAction }))

    expect(
      await screen.findByText(en.data.examined.lost.replace('{{count}}', '1')),
    ).toBeInTheDocument()
    expect(screen.getByText('cover-final.png')).toBeInTheDocument()
    expect(screen.getByText('a.bin')).toBeInTheDocument()
    expect(screen.queryByText(en.data.examineClean)).toBeNull()
  })
})
