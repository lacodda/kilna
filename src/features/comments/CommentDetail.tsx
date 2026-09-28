import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Archive,
  ArchiveRestore,
  ArrowUpRight,
  Check,
  Copy,
  LoaderCircle,
  MessageSquareReply,
  RotateCcw,
  Send,
  Trash2,
  X,
} from 'lucide-react'
import { applyProposal, dismissProposal } from '@/lib/api/assistant'
import { deleteComment, startCommentTask, updateComment } from '@/lib/api/comments'
import type { Comment, CommentPatch, PendingCommentProposal } from '@/lib/api/types'
import { commentAction } from '@/lib/actions'
import { standingOf } from '@/lib/comments'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { DatePicker } from '@/components/DatePicker'
import { Input } from '@/components/ui/input'
import { Markdown } from '@/components/Markdown'
import { SaveState, useSaveStatus } from '@/components/ui/save-state'
import { Textarea } from '@/components/ui/textarea'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { ChannelField } from '@/features/comments/ChannelField'

interface Props {
  comment: Comment
  channels: string[]
  /** Drafted replies to this comment that wait to be kept, oldest first. */
  drafts: PendingCommentProposal[]
  /** Whether a reply is being drafted right now. */
  drafting: boolean
  /** Hide the work line: the comment is shown on its work's own card. */
  onWork?: boolean
  onGone: () => void
}

/**
 * One comment, open: who wrote it where and when, what they said, and the
 * reply — typed, or drafted by the assistant in the channel's voice.
 *
 * Posting is by hand, on purpose: kilna does not speak on anyone's channel.
 * "Copy" puts the reply on the clipboard to paste where the comment is, and
 * "Mark as posted" is the person saying it went.
 */
export function CommentDetail({
  comment,
  channels,
  drafts,
  drafting,
  onWork = false,
  onGone,
}: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const { config } = useProfile()
  const replier = commentAction(config.prompts, 'reply')

  const [reply, setReply] = useState(comment.reply ?? '')
  const [lastSaved, setLastSaved] = useState(comment.reply ?? '')
  const [editingBody, setEditingBody] = useState(false)
  const [body, setBody] = useState(comment.body)
  const [author, setAuthor] = useState(comment.author ?? '')
  const [moving, setMoving] = useState(false)
  const [channel, setChannel] = useState(comment.channel)
  const [picking, setPicking] = useState(false)

  // A reply kept from a draft arrives from outside this box; the box follows
  // it unless something typed here is still unsaved.
  if ((comment.reply ?? '') !== lastSaved) {
    if (reply === lastSaved) setReply(comment.reply ?? '')
    setLastSaved(comment.reply ?? '')
  }

  const patch = useAppMutation({
    mutationFn: (change: CommentPatch) => updateComment(comment.id, change),
    failure: 'comments.saveFailed',
    refresh: refresh.comment,
  })
  const saved = useSaveStatus(patch.isPending, patch.isError)

  const remove = useAppMutation({
    mutationFn: () => deleteComment(comment.id),
    failure: 'comments.saveFailed',
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('comments.deleted'),
        refresh: refresh.comment,
      })
      onGone()
    },
  })

  const draft = useAppMutation({
    mutationFn: async () => {
      // What is typed goes in first: the draft is written against it.
      if (reply !== (comment.reply ?? '')) {
        await updateComment(comment.id, { reply: reply.trim() === '' ? null : reply })
      }
      return startCommentTask(comment.id, replier!.key)
    },
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: () => {
      say.info(t('comments.drafting'))
    },
  })

  const keepDraft = useAppMutation({
    mutationFn: (messageId: string) => applyProposal(messageId),
    failure: 'comments.saveFailed',
    refresh: refresh.keptComment,
  })
  const dropDraft = useAppMutation({
    mutationFn: (messageId: string) => dismissProposal(messageId),
    failure: 'assistant.dismissFailed',
    refresh: [keys.comments, keys.pendingProposals],
  })

  const work = useQuery({
    ...queries.work(comment.work_id ?? ''),
    enabled: comment.work_id !== null && !onWork,
  })

  const saveReply = () => {
    if (reply === (comment.reply ?? '')) return
    patch.mutate({ reply: reply.trim() === '' ? null : reply })
  }

  const copy = () => {
    navigator.clipboard.writeText(reply.trim()).then(
      () => say.ok(t('comments.copied')),
      (cause: unknown) => say.failedTo(t('comments.copyFailed'), cause),
    )
  }

  const standing = standingOf(comment)
  const busy = drafting || draft.isPending

  return (
    <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise">
      <header className="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-3 py-2">
        <Input
          value={author}
          onChange={(event) => setAuthor(event.target.value)}
          onBlur={() => {
            if (author.trim() !== (comment.author ?? '')) {
              patch.mutate({ author: author.trim() === '' ? null : author.trim() })
            }
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
          }}
          placeholder={t('comments.authorPlaceholder')}
          aria-label={t('comments.author')}
          className="min-w-32 flex-1 border-transparent bg-transparent px-1.5 text-sm font-semibold hover:border-line focus:border-line"
        />
        <SaveState savingLabel={t('save.saving')} savedLabel={t('save.saved')} status={saved} />
        {/* The channel is the switch for moving it: on while the field that
            moves it is open. */}
        <Chip pressed={moving} onPressedChange={setMoving} title={t('comments.moveChannel')}>
          {comment.channel}
        </Chip>
        <DatePicker
          value={comment.commented_on ?? ''}
          onChange={(next) => patch.mutate({ commented_on: next === '' ? null : next })}
          placeholder={t('comments.dayPlaceholder')}
          aria-label={t('comments.day')}
          className="w-32"
        />
        <Button
          size="icon-sm"
          variant="danger"
          title={t('comments.delete')}
          aria-label={t('comments.delete')}
          disabled={remove.isPending}
          onClick={() => remove.mutate()}
        >
          <Trash2 aria-hidden />
        </Button>
      </header>

      {moving && (
        <div className="flex shrink-0 flex-col gap-2 border-b border-line bg-soft px-3 py-2">
          <ChannelField value={channel} onChange={setChannel} known={channels} autoFocus />
          <div className="flex justify-end gap-1.5">
            <Button size="sm" onClick={() => setMoving(false)}>
              {t('dialog.cancel')}
            </Button>
            <Button
              size="sm"
              variant="primary"
              disabled={channel.trim() === '' || channel.trim() === comment.channel}
              onClick={() => {
                patch.mutate({ channel: channel.trim() })
                setMoving(false)
              }}
            >
              {t('comments.move')}
            </Button>
          </div>
        </div>
      )}

      {!onWork && (
        <div className="flex shrink-0 items-center gap-1.5 border-b border-line px-3 py-1.5 text-xs text-dim">
          {comment.work_id === null ? (
            <Button variant="link" onClick={() => setPicking(true)}>
              {t('comments.attach')}
            </Button>
          ) : (
            <>
              <Button
                variant="link"
                className="min-w-0"
                onClick={() => void navigate(`/works/${comment.work_id ?? ''}/comments`)}
              >
                <span className="truncate">{work.data?.title ?? '…'}</span>
                <ArrowUpRight aria-hidden />
              </Button>
              <Button
                size="icon-xs"
                variant="icon"
                title={t('comments.detach')}
                aria-label={t('comments.detach')}
                onClick={() => patch.mutate({ work_id: null })}
              >
                <X aria-hidden />
              </Button>
            </>
          )}
        </div>
      )}

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        {/* What they said. Read as they wrote it; a click corrects a
            misreading, the one edit a comment usually needs. */}
        {editingBody ? (
          <Textarea
            autoFocus
            autoResize
            maxRows={16}
            value={body}
            onChange={(event) => setBody(event.target.value)}
            onBlur={() => {
              setEditingBody(false)
              if (body.trim() !== '' && body.trim() !== comment.body) {
                patch.mutate({ body: body.trim() })
              } else {
                setBody(comment.body)
              }
            }}
            aria-label={t('comments.body')}
            className="text-sm"
          />
        ) : (
          <blockquote
            className="cursor-text border-l-2 border-line-2 pl-3"
            onClick={() => setEditingBody(true)}
            title={t('comments.editBody')}
          >
            <Markdown body={comment.body} className="text-sm" />
          </blockquote>
        )}

        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <span className="caption">{t('comments.reply')}</span>
            <span
              className={cn(
                'rounded-full px-2 py-px text-2xs',
                standing === 'posted' && 'bg-good-soft text-good',
                standing === 'drafted' && 'bg-warn-soft text-warn',
                standing === 'waiting' && 'bg-accent-soft text-accent-2',
                standing === 'archived' && 'bg-soft text-faint',
              )}
            >
              {t(`comments.standing.${standing}`)}
            </span>
          </div>
          <Textarea
            autoResize
            maxRows={14}
            rows={3}
            value={reply}
            onChange={(event) => setReply(event.target.value)}
            onBlur={saveReply}
            placeholder={t('comments.replyPlaceholder')}
            aria-label={t('comments.reply')}
            className="text-sm"
          />

          {/* Drafts the assistant wrote, each kept or dropped on its own:
              keeping one puts it in the box above, where it can still be
              changed before it is posted. */}
          {drafts.map((pending) => (
            <div
              key={pending.message_id}
              className="flex flex-col gap-2 rounded-lg border border-accent/40 bg-accent-soft/40 p-2.5"
            >
              <p className="text-xs font-medium text-accent-2">{t('comments.draftReady')}</p>
              <p className="selectable text-sm whitespace-pre-wrap">{pending.body}</p>
              <div className="flex justify-end gap-1.5">
                <Button
                  size="sm"
                  disabled={dropDraft.isPending}
                  onClick={() => dropDraft.mutate(pending.message_id)}
                >
                  <X aria-hidden />
                  {t('comments.discard')}
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  disabled={keepDraft.isPending}
                  onClick={() => keepDraft.mutate(pending.message_id)}
                >
                  <Check aria-hidden />
                  {t('comments.useDraft')}
                </Button>
              </div>
            </div>
          ))}

          <div className="flex flex-wrap items-center gap-1.5">
            {replier !== undefined && comment.state !== 'archived' && (
              <Button size="sm" disabled={busy} onClick={() => draft.mutate()}>
                {busy ? (
                  <LoaderCircle aria-hidden className="animate-spin" />
                ) : (
                  <MessageSquareReply aria-hidden />
                )}
                {busy ? t('comments.draftingShort') : t('comments.draft')}
              </Button>
            )}
            <Button size="sm" disabled={reply.trim() === ''} onClick={copy}>
              <Copy aria-hidden />
              {t('comments.copy')}
            </Button>
            <span className="flex-1" />
            {comment.state === 'open' && (
              <>
                <Button
                  size="sm"
                  onClick={() => patch.mutate({ state: 'archived' })}
                  title={t('comments.archiveHint')}
                >
                  <Archive aria-hidden />
                  {t('comments.archive')}
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  onClick={() =>
                    // One write: the reply as it stands and the word that it
                    // went, so neither overtakes the other.
                    patch.mutate({
                      state: 'posted',
                      ...(reply !== (comment.reply ?? '')
                        ? { reply: reply.trim() === '' ? null : reply }
                        : {}),
                    })
                  }
                  title={t('comments.postedHint')}
                >
                  <Send aria-hidden />
                  {t('comments.markPosted')}
                </Button>
              </>
            )}
            {comment.state === 'posted' && (
              <Button size="sm" onClick={() => patch.mutate({ state: 'open' })}>
                <RotateCcw aria-hidden />
                {t('comments.reopen')}
              </Button>
            )}
            {comment.state === 'archived' && (
              <Button size="sm" onClick={() => patch.mutate({ state: 'open' })}>
                <ArchiveRestore aria-hidden />
                {t('comments.unarchive')}
              </Button>
            )}
          </div>
        </div>
      </div>

      <PickWorkDialog
        open={picking}
        onOpenChange={setPicking}
        title={t('comments.pickWorkTitle')}
        onPick={(picked) => patch.mutate({ work_id: picked.work_id })}
      />
    </section>
  )
}
