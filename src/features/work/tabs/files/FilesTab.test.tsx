import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Asset } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Files tab: a grid of tiles, the cover tagged, and a removal that asks
 * first - the one removal in the card nothing brings back.
 */

const COVER: Asset = {
  id: 'a-cover',
  profile_id: IDS.profile,
  work_id: IDS.song,
  release_id: null,
  kind: 'cover',
  path: 'C:/ws/media/a-cover.png',
  label: null,
  original_name: 'lanterns-cover.png',
  style_brick_id: null,
  created_at: NOW,
}

let backend: Backend

beforeEach(() => {
  backend = mockBackend({
    ...answersFor(studio()),
    list_work_assets: ({ workId }) => (workId === IDS.song ? [COVER] : []),
    detach_asset: () => null,
  })
})

describe('a file on the card', () => {
  it('is tagged as the cover when it is one', async () => {
    renderApp(`/works/${IDS.song}/files`)
    const tile = (await screen.findByText('lanterns-cover.png')).closest('li')!
    expect(within(tile).getByText(en.files.cover)).toBeVisible()
  })

  it('is removed only once the question is answered', async () => {
    const { client } = renderApp(`/works/${IDS.song}/files`)
    await screen.findByText('lanterns-cover.png')
    await settled(client)

    fireEvent.click(screen.getByRole('button', { name: en.files.detach }))
    const question = await screen.findByRole('alertdialog')
    expect(
      within(question).getByText(en.files.detachTitle.replace('{{name}}', 'lanterns-cover.png')),
    ).toBeVisible()
    expect(backend.argsOf('detach_asset')).toEqual([])

    fireEvent.click(within(question).getByRole('button', { name: en.files.detachConfirm }))
    await waitFor(() => expect(backend.argsOf('detach_asset')).toEqual([{ id: COVER.id }]))
  })
})
