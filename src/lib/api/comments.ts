import { invoke } from '@tauri-apps/api/core'
import type {
  Comment,
  CommentFilter,
  CommentPatch,
  NewComment,
  PendingCommentProposal,
  StartedTask,
} from '@/lib/api/types'

// The audience's comments and the replies to them.

export const listComments = (filter?: CommentFilter) =>
  invoke<Comment[]>('list_comments', { filter: filter ?? null })
/** Every channel comments came through, with how many wait on each. */
export const commentChannels = () => invoke<[string, number][]>('comment_channels')
/** A work's comments, and how many of them wait: its tab's counter. */
export const countWorkComments = (workId: string) =>
  invoke<{ total: number; waiting: number }>('count_work_comments', { workId })
export const createComment = (comment: NewComment) => invoke<Comment>('create_comment', { comment })
export const updateComment = (id: string, patch: CommentPatch) =>
  invoke<Comment>('update_comment', { id, patch })
/** Returns the trash entry, for the undo. */
export const deleteComment = (id: string) => invoke<string>('delete_comment', { id })
export const pendingCommentProposals = () =>
  invoke<PendingCommentProposal[]>('pending_comment_proposals')
/** Draft a reply in the background, in the voice of the comment's channel. */
export const startCommentTask = (id: string, action: string) =>
  invoke<StartedTask>('start_comment_task', { id, action })
/** Read a pasted screenshot of a comment in the background. `today` is the
 *  person's own date, for turning "3 weeks ago" into a day. */
export const startScreenshotTask = (args: {
  bytes: Uint8Array
  name: string
  channel: string
  workId: string | null
  action: string
  today: string
}) =>
  invoke<StartedTask>('start_screenshot_task', {
    bytes: Array.from(args.bytes),
    name: args.name,
    channel: args.channel,
    workId: args.workId,
    action: args.action,
    today: args.today,
  })
