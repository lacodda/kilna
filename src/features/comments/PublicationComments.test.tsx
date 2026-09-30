import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, fireEvent, screen, within } from '@testing-library/react'
import type { Comment, CommentPatch } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Comments live at publications (v0.86, ADR 0047). A song does not go out
 * itself, so nobody comments on it: its Comments tab sums up what was said
 * under its clip, its audio and its shorts, each group leading to where the
 * replies are written. A work that goes out keeps its board. And wherever a
 * comment is filed, only what goes out is offered - a comment left on a song
 * from before says where it belongs.
 */

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
  vi.restoreAllMocks()
})

describe("a song's comments", () => {
  it('are a summary of its publications, read only, each leading to its board', async () => {
    const { client } = renderApp(`/works/${IDS.song}/comments`)
    await settled(client)
    const main = await screen.findByRole('main')

    expect(within(main).getByText(en.comments.summary.title)).toBeInTheDocument()
    expect(within(main).getByText(en.comments.summary.hint)).toBeInTheDocument()
    expect(backend.argsOf('list_comments')).toContainEqual({ filter: { under: IDS.song } })

    const group = within(main).getByRole('region', { name: 'Paper Lanterns (clip)' })
    expect(within(group).getByText('1 comment')).toBeInTheDocument()
    expect(within(group).getByText('listener42')).toBeInTheDocument()
    expect(within(group).getByText('The shot on the bridge is beautiful.')).toBeInTheDocument()
    // Read, not answered: no board, no reply box, nothing to file a comment with.
    expect(within(main).queryByRole('button', { name: en.comments.new })).toBeNull()
    expect(within(main).queryByRole('textbox')).toBeNull()

    await act(async () =>
      fireEvent.click(within(group).getByRole('button', { name: en.comments.summary.open })),
    )
    await settled(client)
    // The publication's own board, where the replies are written.
    expect(await within(main).findByRole('button', { name: en.comments.new })).toBeInTheDocument()
    expect(within(main).queryByText(en.comments.summary.title)).toBeNull()
  })

  it('keep the board on a work that goes out', async () => {
    const { client } = renderApp(`/works/${IDS.video}/comments`)
    await settled(client)
    const main = await screen.findByRole('main')

    expect(within(main).getByRole('button', { name: en.comments.new })).toBeInTheDocument()
    expect(within(main).queryByText(en.comments.summary.title)).toBeNull()
  })
})

/** The titles the open work picker offers, each as its row names it. */
async function offered(): Promise<string[]> {
  return (await screen.findAllByRole('option')).map(
    (option) => option.querySelector('b')?.textContent ?? '',
  )
}

describe('filing a comment', () => {
  it('offers only works that go out', async () => {
    const { client } = renderApp('/comments')
    await settled(client)

    fireEvent.click(await screen.findByRole('button', { name: en.comments.new }))
    const dialog = await screen.findByRole('dialog')
    fireEvent.click(within(dialog).getByRole('button', { name: en.comments.pickWork }))

    // The clip, the short and the audio - neither song.
    expect((await offered()).sort()).toEqual([
      'Paper Lanterns (clip)',
      'Paper Lanterns (short)',
      'Paper Lanterns — audio',
    ])
    // Nor is "Song" a kind to narrow to.
    expect(screen.getByRole('button', { name: 'Video' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Song' })).toBeNull()
  })

  it('says where a comment left on a song belongs, and moves it there', async () => {
    const misfiled: Comment = {
      ...workspace.comments[0]!,
      id: 'c-on-song',
      work_id: IDS.song,
      body: 'Is there a lyric video?',
    }
    workspace.comments.push(misfiled)
    backend.answer('update_comment', ({ id, patch }) => ({
      ...misfiled,
      id: id as string,
      ...(patch as CommentPatch),
    }))
    const { client } = renderApp(`/comments/${misfiled.id}`)
    await settled(client)

    expect(
      await screen.findByText(en.comments.notAPublication.replace('{{title}}', 'Paper Lanterns')),
    ).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: en.comments.moveToPublication }))

    expect(await offered()).not.toContain('Paper Lanterns')
    const clip = screen
      .getAllByRole('option')
      .find((option) => option.querySelector('b')?.textContent === 'Paper Lanterns (clip)')!
    await act(async () => fireEvent.click(clip))
    await settled(client)

    expect(backend.argsOf('update_comment')).toEqual([
      { id: misfiled.id, patch: { work_id: IDS.video } },
    ])
  })
})
