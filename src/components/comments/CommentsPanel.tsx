import { useState } from 'react'
import { CommentBoard } from '@/components/comments/CommentBoard'

/**
 * A work's comments, on its card: the inbox narrowed to one work, with a
 * screenshot pasted here filed under it.
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
