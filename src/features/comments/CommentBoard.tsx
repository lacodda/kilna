import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ClipboardPaste, LoaderCircle, MessageSquareReply, Plus } from 'lucide-react'
import type {
  Comment,
  CommentProposal,
  CommentState,
  PendingCommentProposal,
} from '@/lib/api/types'
import { commentAction } from '@/lib/actions'
import { standingOf, type Standing } from '@/lib/comments'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { channelOfTask, commentTaskKey, isScreenshotTask } from '@/lib/tasks'
import { useDebounced } from '@/lib/useDebounced'
import { useProfile } from '@/lib/useProfile'
import { useRunningTasks } from '@/lib/useRunningTasks'
import { cn } from '@/lib/utils'
import { formatDay } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { CommentDetail } from '@/features/comments/CommentDetail'
import { NewCommentDialog } from '@/features/comments/NewCommentDialog'
import { ProposedComment } from '@/features/comments/ProposedComment'
import { ScreenshotDialog } from '@/features/comments/ScreenshotDialog'

interface Props {
  /** The work whose comments these are, on its card; absent is the inbox. */
  workId?: string
  selectedId: string | undefined
  onSelect: (id: string | null) => void
}

/** The states a person narrows the list to, the inbox's first. */
const STATES: CommentState[] = ['open', 'posted', 'archived']

/*
 * What a channel chip is called in its group. "All" is a chip of its own, and
 * a channel is free text, so the channels carry a prefix: one someone named
 * "all" cannot be taken for it.
 */
const ALL_CHANNELS = 'all'
const chipOf = (channel: string | undefined) =>
  channel === undefined ? ALL_CHANNELS : `channel:${channel}`
const channelOf = (chip: string | undefined) =>
  chip === undefined || chip === ALL_CHANNELS ? undefined : chip.slice('channel:'.length)

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
  const comments = useQuery(queries.commentsMatching(filter))
  const channels = useQuery(queries.commentChannels())
  const proposals = useQuery(queries.commentProposals())

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
    <Frame
      head={
        <>
          {/* The channels as chips, with what waits on each: the question of an
              inbox is where the unanswered ones are. */}
          {known.length > 0 && (
            <ChipGroup
              aria-label={t('comments.channel')}
              value={[chipOf(channel)]}
              onValueChange={(next) => setChannel(channelOf(next[0]))}
            >
              <Chip value={chipOf(undefined)}>{t('comments.allChannels')}</Chip>
              {(channels.data ?? []).map(([name, waiting]) => (
                <Chip key={name} value={chipOf(name)} count={waiting > 0 ? waiting : undefined}>
                  {name}
                </Chip>
              ))}
            </ChipGroup>
          )}
          {/* One state at a time, always one: the inbox is the open ones. */}
          <SegmentedControl
            aria-label={t('comments.state')}
            value={state}
            onValueChange={(next) => setState(next as CommentState)}
          >
            {STATES.map((one) => (
              <Segment key={one} value={one}>
                {t(`comments.states.${one}`)}
              </Segment>
            ))}
          </SegmentedControl>
          <Input
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder={t('comments.search')}
            aria-label={t('comments.search')}
            className="w-52"
          />
          <span
            className="ml-auto hidden items-center gap-1 text-xs text-faint lg:flex"
            title={t('comments.pasteHint')}
          >
            <ClipboardPaste aria-hidden className="size-3.5" />
            {t('comments.pasteShort')}
          </span>
          <Button variant="primary" onClick={() => setAdding(true)}>
            <Plus aria-hidden />
            {t('comments.new')}
          </Button>
        </>
      }
    >
      <ListDetail
        list={
          <Pane label={t('nav.comments')} bodyClassName="flex flex-col gap-1.5 p-1.5">
            {reading.map((key) => (
              <p
                key={key}
                className="flex items-center gap-2 rounded-lg bg-soft px-2.5 py-2 text-xs text-dim"
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

            <Loaded
              query={comments}
              skeleton={<SkeletonList rows={6} />}
              isEmpty={(data) => data.length === 0}
              emptyState={
                <EmptyState
                  plain
                  variant={filtered ? 'filtered' : 'empty'}
                  title={filtered ? t('comments.noMatches') : t('comments.none')}
                  className="p-2"
                />
              }
              plain
            >
              {() => (
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
            </Loaded>
          </Pane>
        }
        detail={
          selected !== undefined ? (
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
              className="flex-1"
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
          )
        }
      />

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
    </Frame>
  )
}

/** The dot a comment's standing is drawn with, here and in a song's summary. */
export const STANDING_DOT: Record<Standing, string> = {
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
  const { t } = useTranslation()
  const standing = standingOf(comment)
  const day = comment.commented_on === null ? null : formatDay(comment.commented_on)

  // A reply being drafted, or one drafted and waiting: a glyph beside the day,
  // with its words for a reader and for the pointer.
  const draft = drafting ? t('comments.draftingShort') : drafted ? t('comments.draftReady') : null

  // A row of the list, so a line each: who and where, then the opening of
  // what they said. The whole comment is one click away.
  return (
    <RowButton
      selected={active}
      onClick={onOpen}
      start={
        <span
          aria-hidden
          title={t(`comments.standing.${standing}`)}
          className={cn('size-1.5 rounded-full', STANDING_DOT[standing])}
        />
      }
      description={comment.body}
      end={
        draft === null && day === null ? undefined : (
          <>
            {draft !== null && (
              <span title={draft} className="flex items-center text-accent-2">
                {drafting ? (
                  <LoaderCircle aria-hidden className="size-3 animate-spin" />
                ) : (
                  <MessageSquareReply aria-hidden className="size-3" />
                )}
                <span className="sr-only">{draft}</span>
              </span>
            )}
            {day}
          </>
        )
      }
    >
      {comment.author ?? t('comments.someone')}
      <span className="font-normal text-faint"> · {comment.channel}</span>
    </RowButton>
  )
}
