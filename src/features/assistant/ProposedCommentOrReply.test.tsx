import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MemoryRouter } from 'react-router'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { Applied, ReplyProposal } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend, type Backend } from '@/test/backend'
import { answersFor, NOW, studio } from '@/test/workspace'
import { ProposedCommentOrReply } from '@/features/assistant/ProposedCommentOrReply'

/*
 * A reply in three ways (v0.162, FR-22 22.3): in the commenter's language,
 * each translated, the comment's meaning said; the one chosen is kept.
 */

let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  backend = mockBackend({
    ...answersFor(studio()),
    apply_proposal: ({ messageId }) =>
      ({ message_id: messageId as string, at: NOW, versions: [] }) as unknown as Applied,
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('a reply in three ways', () => {
  it('shows each way with its translation and keeps the one chosen', async () => {
    const proposal: ReplyProposal = {
      kind: 'reply',
      comment_id: 'c-1',
      language: 'Spanish',
      translation: 'what instrument is that?',
      context: 'a plain question, no irony',
      variants: [
        { reply: 'Es un charango.', translation: 'It is a charango.' },
        { reply: 'Un charango, pequeño y ruidoso.', translation: 'A charango, small and loud.' },
        { reply: '¡Charango! Gracias por escuchar.' },
      ],
    }
    render(
      <Providers client={createQueryClient()}>
        <MemoryRouter>
          <ProposedCommentOrReply
            messageId="m-reply"
            proposal={proposal}
            applied={null}
            dismissed={false}
          />
        </MemoryRouter>
      </Providers>,
    )

    expect(screen.getByText('what instrument is that?')).toBeInTheDocument()
    expect(screen.getByText('a plain question, no irony')).toBeInTheDocument()
    expect(screen.getByText('It is a charango.')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('radio', { name: /Un charango, pequeño y ruidoso/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Keep as the reply' }))

    await waitFor(() => expect(backend.argsOf('apply_proposal')).toHaveLength(1))
    expect(backend.argsOf('apply_proposal')[0]).toMatchObject({
      messageId: 'm-reply',
      overrides: { reply: 'Un charango, pequeño y ruidoso.' },
    })
  })
})
