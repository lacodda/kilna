import { useState } from 'react'
import { hasDoors, useProfile, useWorkKind } from '@/lib/useProfile'
import { CommentBoard } from '@/features/comments/CommentBoard'
import { PublicationComments } from '@/features/comments/PublicationComments'

/**
 * A work's comments, on its card: the inbox narrowed to one work, with a
 * screenshot pasted here filed under it.
 *
 * A work that does not go out itself - a song - has no comments of its own
 * since v0.86: the audience comments under its publications, and its tab sums
 * those up instead, each group leading to the publication where the replies
 * are written.
 */
export function CommentsPanel({ workId }: { workId: string }) {
  const { config } = useProfile()
  const kind = useWorkKind(workId)
  const [selected, setSelected] = useState<string | undefined>(undefined)

  // The card has the work before it draws a tab; until the kind is known
  // there is no telling a board from a summary, and drawing the wrong one
  // first would flash an inbox on a song.
  if (kind === undefined) return null
  if (!hasDoors(config, kind)) return <PublicationComments workId={workId} />

  return (
    <CommentBoard
      workId={workId}
      selectedId={selected}
      onSelect={(id) => setSelected(id ?? undefined)}
    />
  )
}
