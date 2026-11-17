import { useState } from 'react'
import { CommentBoard } from '@/features/comments/CommentBoard'

/**
 * A work's comments, on its card: the inbox narrowed to one work, with a
 * screenshot pasted here filed under it.
 *
 * Only a work that goes out has the tab (v0.117): the audience comments
 * under a clip, an audio release, a short - never under the song they were
 * made from, which summed its publications' comments up here until then.
 */
export function CommentsPanel({ workId }: { workId: string }) {
  const [selected, setSelected] = useState<string | undefined>(undefined)

  return (
    <CommentBoard
      workId={workId}
      selectedId={selected}
      onSelect={(id) => setSelected(id ?? undefined)}
    />
  )
}
