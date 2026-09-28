import { beforeEach, describe, expect, it } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp } from '@/test/render'
import { answersFor, IDS, studio } from '@/test/workspace'

/*
 * A reply is read before it is asked, as a work's action is.
 *
 * The backend could compose a comment's task without sending it since the
 * inbox arrived, and nothing asked it to: drafting a reply sent a prompt
 * nobody had seen. The preview shows what goes, and starting from it saves
 * the typed reply first, because the draft is written against it.
 */

let backend: Backend

beforeEach(() => {
  backend = mockBackend({
    ...answersFor(studio()),
    preview_comment_task: () => ({ prompt: 'Answer the listener in the voice of the channel.' }),
    start_comment_task: () => ({
      chat_id: 'chat-9',
      run_id: 'run-9',
      task_key: 'k',
      title: 'Reply',
    }),
  })
})

describe("a comment's reply", () => {
  it('is shown before it is asked, and asked from the preview', async () => {
    renderApp(`/comments/${IDS.comment}`)
    const preview = await screen.findByRole('button', { name: /^Preview what/ })
    fireEvent.click(preview)

    const dialog = await screen.findByRole('dialog')
    expect(
      await within(dialog).findByText('Answer the listener in the voice of the channel.'),
    ).toBeInTheDocument()
    expect(backend.argsOf('preview_comment_task')[0]).toMatchObject({
      id: IDS.comment,
      action: 'reply-to-comment',
    })
    // A reply's task takes no reference files: that box is a work's.
    expect(within(dialog).queryByRole('textbox')).toBeNull()

    fireEvent.click(within(dialog).getByRole('button', { name: /start/i }))
    await expect.poll(() => backend.argsOf('start_comment_task').length).toBe(1)
    expect(backend.argsOf('start_comment_task')[0]).toMatchObject({ id: IDS.comment })
  })
})
