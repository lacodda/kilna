import { useNavigate, useParams } from 'react-router'
import { CommentBoard } from '@/components/comments/CommentBoard'

/**
 * Every comment the audience left, from every channel: the inbox.
 *
 * The open comment is part of the address, so the back button walks between
 * them and a search hit lands on one directly.
 */
export function CommentsView() {
  const navigate = useNavigate()
  const { commentId } = useParams()

  return (
    <CommentBoard
      selectedId={commentId}
      onSelect={(id) => void navigate(id === null ? '/comments' : `/comments/${id}`)}
    />
  )
}
