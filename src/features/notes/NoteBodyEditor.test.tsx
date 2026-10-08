import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import type { Asset, Note } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A picture in a note (v0.93, ADR 0058): pasted into the text being written,
 * copied into the workspace as the note's own, written where the caret stood
 * as `![](media/<id>.png)` - and drawn as the picture when the note is read.
 */

const MEDIA = 'C:\\Users\\someone\\AppData\\Roaming\\kilna\\media'

const PASTED: Asset = {
  id: 'a-pasted',
  profile_id: IDS.profile,
  work_id: null,
  release_id: null,
  kind: 'inline',
  path: `${MEDIA}\\a-pasted.png`,
  label: null,
  original_name: 'board.png',
  style_brick_id: null,
  note_id: IDS.note,
  canon_fact_id: null,
  created_at: NOW,
}

let backend: Backend
let workspace: ReturnType<typeof studio>

beforeEach(() => {
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    paste_asset: () => PASTED,
    update_note: ({ id, patch }) => {
      const note = workspace.notes.find((one) => one.id === id)!
      Object.assign(note, patch as Partial<Note>)
      return note
    },
  })
})

describe('a picture pasted into a note', () => {
  it('is copied in as the note’s own and written where the caret stood', async () => {
    const { client } = renderApp(`/notes/${IDS.note}`)
    await settled(client)
    fireEvent.click(await screen.findByRole('button', { name: en.notes.edit }))
    const box = await screen.findByRole('textbox', { name: en.notes.bodyLabel })
    const before = (box as HTMLTextAreaElement).value
    ;(box as HTMLTextAreaElement).setSelectionRange(before.length, before.length)

    const picture = new File([new Uint8Array([137, 80, 78, 71])], 'board.png', {
      type: 'image/png',
    })
    fireEvent.paste(box, { clipboardData: { files: [picture], types: ['Files'] } })

    await waitFor(() => expect(backend.argsOf('paste_asset')).toHaveLength(1))
    const [sent] = backend.argsOf('paste_asset')
    expect(sent).toMatchObject({
      name: 'board.png',
      asset: { note_id: IDS.note, kind: 'inline' },
      bytes: [137, 80, 78, 71],
    })

    await waitFor(() =>
      expect((box as HTMLTextAreaElement).value).toBe(`${before}\n![board](media/a-pasted.png)`),
    )
    await waitFor(
      () =>
        expect(backend.argsOf('update_note').at(-1)).toEqual({
          id: IDS.note,
          patch: { body: `${before}\n![board](media/a-pasted.png)` },
        }),
      { timeout: 3000 },
    )
  })

  it('leaves a paste of text to be text', async () => {
    const { client } = renderApp(`/notes/${IDS.note}`)
    await settled(client)
    fireEvent.click(await screen.findByRole('button', { name: en.notes.edit }))
    const box = await screen.findByRole('textbox', { name: en.notes.bodyLabel })
    fireEvent.paste(box, { clipboardData: { files: [], types: ['text/plain'] } })
    expect(backend.argsOf('paste_asset')).toEqual([])
  })
})

describe('a picture in a note being read', () => {
  it('is drawn from the workspace’s own files, and a link elsewhere is left as written', async () => {
    workspace.notes.find((one) => one.id === IDS.note)!.body =
      'The board.\n\n![board](media/a-pasted.png)\n\n![x](media/../kilna.db)'
    const { client } = renderApp(`/notes/${IDS.note}`)
    await settled(client)
    await screen.findByRole('img', { name: 'board' })
    // Looked up again each time: the rendered body is replaced, not patched,
    // once the workspace's folder is known.
    await waitFor(() =>
      expect(screen.getByRole('img', { name: 'board' }).getAttribute('src')).toBe(
        `http://asset.localhost/${encodeURIComponent(`${MEDIA}\\a-pasted.png`)}`,
      ),
    )
    expect(screen.getByRole('img', { name: 'x' }).getAttribute('src')).toBe('media/../kilna.db')
  })
})
