import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ClipboardPaste, LoaderCircle, Plus } from 'lucide-react'
import {
  commentChannels,
  listComments,
  pendingCommentProposals,
  type Comment,
  type CommentProposal,
  type CommentState,
  type PendingCommentProposal,
} from '@/lib/api'
import { commentAction } from '@/lib/actions'
import { standingOf, type Standing } from '@/lib/comments'
import { keys } from '@/lib/query'
import { channelOfTask, commentTaskKey, isScreenshotTask } from '@/lib/tasks'
import { useDebounced } from '@/lib/useDebounced'
import { useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/EmptyState'
import { Input } from '@/components/ui/input'
import { SkeletonList } from '@/components/ui/Skeleton'
import { CommentDetail } from '@/components/comments/CommentDetail'
import { NewCommentDialog } from '@/components/comments/NewCommentDialog'
import { ProposedComment } from '@/components/comments/ProposedComment'
import { ScreenshotDialog } from '@/components/comments/ScreenshotDialog'

interface Props {
  /** The work whose comments these are, on its card; absent is the inbox. */
  workId?: string
  selectedId: string | undefined
  onSelect: (id: string | null) => void
}

/** The states a person narrows the list to, the inbox's first. */
const STATES: CommentState[] = ['open', 'posted', 'archived']

/**
 * The comments, as a list beside the open one: the inbox on the Comments
 * screen, one work's comments on its card.
 *
 * `Ctrl+V` with a picture on the clipboard, anywhere here, reads it as a
 * comment in the background; each reading comes back at the top of the list
 * to be checked and kept. That is how the comments get in — the predecessor's
 * file upload was a button nobody pressed (not one of its 121 comments kept a
 * screenshot).
 */
export function CommentBoard({ workId, selectedId, onSelect }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const reader = commentAction(config.prompts, 'comment')
  const replier = commentAction(config.prompts, 'reply')

  const [channel, setChannel] = useState<string | undefined>(undefined)
  const [state, setState] = useState<CommentState>('open')
  const [text, setText] = useState('')
  const query = useDebounced(text.trim(), 200)
  const [pasted, setPasted] = useState<File | null>(null)
  const [adding, setAdding] = useState(false)

  const filter = {
    work_id: workId,
    channel,
    state,
    search: query === '' ? undefined : query,
  }
  const comments = useQuery({
    queryKey: [...keys.comments, 'list', filter],
    queryFn: () => listComments(filter),
  })
  const channels = useQuery({ queryKey: keys.commentChannels, queryFn: commentChannels })
  const proposals = useQuery({
    queryKey: keys.commentProposals,
    queryFn: pendingCommentProposals,
  })

  // An answer arriving is the moment a reading or a draft becomes something
  // to keep: the list of what waits is asked again then, and not on a timer.
  const onEnded = useCallback(() => {
    void client.invalidateQueries({ queryKey: keys.commentProposals })
  }, [client])
  const running = useRunningTasks(onEnded)

  const known = useMemo(() => (channels.data ?? []).map(([name]) => name), [channels.data])

  // Readings of screenshots: the ones still running, and the ones done and
  // waiting. On a card, only those pasted there — the proposal names its work.
  const readings = (proposals.data ?? []).filter(
    (pending): pending is PendingCommentProposal & { proposal: CommentProposal } =>
      pending.proposal.kind === 'comment' &&
      (workId === undefined || pending.proposal.work_id === workId),
  )
  const reading =
    reader === undefined ? [] : [...running].filter((key) => isScreenshotTask(key, reader.key))
  const draftsOf = (commentId: string) =>
    (proposals.data ?? []).filter(
      (pending) => pending.proposal.kind === 'reply' && pending.proposal.comment_id === commentId,
    )

  // A picture pasted anywhere on the board. Only a picture: pasting text into
  // the reply or the search is left to the box, which is what it means there.
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const file = Array.from(event.clipboardData?.files ?? []).find((one) =>
        one.type.startsWith('image/'),
      )
      if (file === undefined) return
      event.preventDefault()
      setPasted(file)
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [])

  const rows = comments.data ?? []
  const selected: Comment | undefined = rows.find((one) => one.id === selectedId)
  const filtered = channel !== undefined || query !== '' || state !== 'open'

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        {/* The channels as chips, with what waits on each: the question of an
            inbox is where the unanswered ones are. */}
        {known.length > 0 && (
          <div role="group" aria-label={t('comments.channel')} className="flex flex-wrap items-center gap-1.5">
            {[
              { name: undefined as string | undefined, label: t('comments.allChannels'), waiting: undefined },
              ...(channels.data ?? []).map(([name, waiting]) => ({ name, label: name, waiting })),
            ].map((entry) => {
              const active = channel === entry.name
              return (
                <button
                  key={entry.name ?? ''}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setChannel(active ? undefined : entry.name)}
                  className={cn(
                    'flex cursor-pointer items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11.5px] transition-colors',
                    active
                      ? 'border-transparent bg-accent-soft font-semibold text-accent-2'
                      : 'border-line text-dim hover:border-line-2 hover:text-text',
                  )}
                >
                  {entry.label}
                  {entry.waiting !== undefined && entry.waiting > 0 && (
                    <span className="text-[10.5px] text-accent-2 tabular-nums">{entry.waiting}</span>
                  )}
                </button>
              )
            })}
          </div>
        )}
        <div role="group" aria-label={t('comments.state')} className="flex rounded-md bg-soft p-0.5">
          {STATES.map((one) => (
            <button
              key={one}
              type="button"
              aria-pressed={state === one}
              onClick={() => setState(one)}
              className={cn(
                'cursor-pointer rounded px-2.5 py-0.5 text-[11.5px] text-dim transition-colors',
                state === one && 'bg-raise font-semibold text-text shadow-sm',
              )}
            >
              {t(`comments.states.${one}`)}
            </button>
          ))}
        </div>
        <Input
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={t('comments.search')}
          aria-label={t('comments.search')}
          className="w-52"
        />
        <span
          className="ml-auto hidden items-center gap-1 text-[11px] text-faint lg:flex"
          title={t('comments.pasteHint')}
        >
          <ClipboardPaste aria-hidden className="size-3.5" />
          {t('comments.pasteShort')}
        </span>
        <Button variant="primary" onClick={() => setAdding(true)}>
          <Plus aria-hidden />
          {t('comments.new')}
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[300px_minmax(0,1fr)] gap-3">
        <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-raise">
          <div className="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto p-1.5">
            {reading.map((key) => (
              <p
                key={key}
                className="flex items-center gap-2 rounded-lg bg-soft px-2.5 py-2 text-[11.5px] text-dim"
              >
                <LoaderCircle aria-hidden className="size-3.5 animate-spin" />
                {t('comments.readingOn', { channel: channelOfTask(key) ?? '' })}
              </p>
            ))}
            {readings.map((pending) => (
              <ProposedComment
                key={pending.message_id}
                pending={pending}
                channels={known}
                onKept={(id) => onSelect(id)}
              />
            ))}

            {comments.isPending ? (
              <SkeletonList rows={6} />
            ) : comments.isError ? (
              <p role="alert" className="p-3 text-sm text-bad">
                {t('toast.loadFailed')}
              </p>
            ) : rows.length === 0 ? (
              <p className="p-3 text-xs text-faint">
                {filtered ? t('comments.noMatches') : t('comments.none')}
              </p>
            ) : (
              <ul className="flex flex-col gap-0.5">
                {rows.map((comment) => (
                  <li key={comment.id}>
                    <CommentRow
                      comment={comment}
                      active={comment.id === selectedId}
                      drafting={
                        replier !== undefined &&
                        running.has(commentTaskKey(replier.key, comment.id))
                      }
                      drafted={draftsOf(comment.id).length > 0}
                      onOpen={() => onSelect(comment.id)}
                    />
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {selected !== undefined ? (
          <CommentDetail
            key={selected.id}
            comment={selected}
            channels={known}
            drafts={draftsOf(selected.id)}
            drafting={
              replier !== undefined && running.has(commentTaskKey(replier.key, selected.id))
            }
            onWork={workId !== undefined}
            onGone={() => onSelect(null)}
          />
        ) : (
          <EmptyState
            title={rows.length === 0 && !filtered ? t('comments.empty') : t('comments.pick')}
            body={rows.length === 0 && !filtered ? t('comments.emptyBody') : undefined}
            action={
              rows.length === 0 && !filtered ? (
                <Button variant="primary" onClick={() => setAdding(true)}>
                  <Plus aria-hidden />
                  {t('comments.new')}
                </Button>
              ) : undefined
            }
          />
        )}
      </div>

      <ScreenshotDialog
        file={pasted}
        onClose={() => setPasted(null)}
        channels={known}
        channel={channel}
        workId={workId}
      />
      <NewCommentDialog
        open={adding}
        onOpenChange={setAdding}
        channels={known}
        channel={channel}
        workId={workId}
        onCreated={(id) => onSelect(id)}
      />
    </div>
  )
}

const STANDING_DOT: Record<Standing, string> = {
  waiting: 'bg-accent',
  drafted: 'bg-warn',
  posted: 'bg-good',
  archived: 'bg-line-2',
}

function CommentRow({
  comment,
  active,
  drafting,
  drafted,
  onOpen,
}: {
  comment: Comment
  active: boolean
  drafting: boolean
  /** A drafted reply waits to be kept. */
  drafted: boolean
  onOpen: () => void
}) {
  const { t, i18n } = useTranslation()
  const standing = standingOf(comment)
  const day =
    comment.commented_on === null
      ? null
      : new Date(`${comment.commented_on}T12:00:00`).toLocaleDateString(i18n.language, {
          day: 'numeric',
          month: 'short',
        })

  return (
    <button
      type="button"
      onClick={onOpen}
      aria-current={active ? 'true' : undefined}
      className={cn(
        'flex w-full cursor-pointer flex-col gap-0.5 rounded-lg px-2.5 py-2 text-left transition-colors',
        active ? 'bg-accent-soft' : 'hover:bg-soft',
      )}
    >
      <span className="flex min-w-0 items-center gap-1.5 text-[11px] text-faint">
        <span
          aria-hidden
          title={t(`comments.standing.${standing}`)}
          className={cn('size-1.5 shrink-0 rounded-full', STANDING_DOT[standing])}
        />
        <b className="truncate font-semibold text-text">{comment.author ?? t('comments.someone')}</b>
        <span className="truncate">· {comment.channel}</span>
        {day !== null && <span className="ml-auto shrink-0 tabular-nums">{day}</span>}
      </span>
      <span className="line-clamp-2 text-[12.5px] text-dim">{comment.body}</span>
      {(drafting || drafted) && (
        <span className="flex items-center gap-1 text-[10.5px] text-accent-2">
          {drafting && <LoaderCircle aria-hidden className="size-3 animate-spin" />}
          {drafting ? t('comments.draftingShort') : t('comments.draftReady')}
        </span>
      )}
    </button>
  )
}
