import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Compass, Eye, Sparkles } from 'lucide-react'
import { applyProposal, dismissProposal } from '@/lib/api/assistant'
import {
  addOwnCard,
  chipToPhrase,
  deleteCard,
  judgeCard,
  moveCard,
  retopicCards,
  updateCard,
} from '@/lib/api/boards'
import type { Board, CardVerdict, Work } from '@/lib/api/types'
import {
  CARD_FILTERS,
  CARDS_ON_ASK,
  MOST_CARDS,
  counts,
  dropOf,
  elsewhereOf,
  headlineOf,
  kindOf,
  kindsOf,
  libraryOf,
  requestOf,
  shows,
  topicsOf,
  type CardFilter,
  type Grouping,
} from '@/lib/boards'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { useChipDrag } from '@/lib/useChipDrag'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { refresh as refreshing } from '@/lib/query/refresh'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Input } from '@/components/ui/input'
import { Panel } from '@/components/ui/panel'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { useFieldDraft } from '@/components/ui/field-draft'
import { Select } from '@/components/AppSelect'
import { RowMenu } from '@/components/RowMenu'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'
import { clockOf } from '@/features/work/tabs/cover/useCoverIdeas'
import { BoardCardItem, CardSkeleton } from '@/features/work/tabs/conception/BoardCardItem'
import { useBoardTask } from '@/features/work/tabs/conception/useBoardTask'

interface Props {
  work: Work
  board: Board
}

/**
 * One board of a work's conception (v0.99, ADR 0065): the composer - the
 * person's own card, whether to have it worked out, how many of the
 * assistant's own, where to dig - the directions the last run offered, the
 * filters with their counts, and the cards under their topics, skeletons
 * first while cards are written.
 *
 * The mechanism is the cover's board of ideas (ADR 0050) with the craft's
 * words: a card is the board's kind of card in its fields, and a card can be
 * carried to another place or topic by its grip. A topic is renamed by its
 * heading, and renamed onto another topic it merges into it.
 */
export function BoardSection({ work, board }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const library = libraryOf(config)
  const task = useBoardTask(work, board)
  const views = useQuery(queries.boards(work.id))
  const view = (views.data ?? []).find((one) => one.board === board.key)
  const cards = view?.cards ?? []
  const [words, setWords] = useState('')
  /** The kind the person's own card is put on the board as. */
  const [ownKind, setOwnKind] = useState(board.cards[0]?.key ?? '')
  const [refine, setRefine] = useState(true)
  const [count, setCount] = useState(CARDS_ON_ASK)
  const [direction, setDirection] = useState('')
  const [filter, setFilter] = useState<CardFilter>('all')
  const [grouping, setGrouping] = useState<Grouping>('topics')
  const [previewing, setPreviewing] = useState(false)
  /** The card a new topic is being named for. */
  const [naming, setNaming] = useState<string | null>(null)
  const [topicName, setTopicName] = useState('')
  const refresh = [keys.boards(work.id), keys.cardCounts(work.id)]
  const ownCardKind = kindOf(board, { kind: ownKind }) ?? board.cards[0]

  // The phrases kept for the work: a chip among them is sent.
  const notes = useQuery({
    ...queries.notesFor(work.id),
    enabled: board.cards.some((kind) => kind.fields.some((field) => field.shape === 'chips')),
  })
  const lineKinds = new Set(
    config.note_kinds.filter((kind) => kind.line === true).map((kind) => kind.key),
  )
  const kept = new Set(
    (notes.data ?? [])
      .filter((note) => lineKinds.has(note.kind))
      .map((note) => note.body.trim().toLowerCase()),
  )

  const tally = counts(cards)
  const hasWords = words.trim() !== ''
  const withAi = task.action !== undefined
  const request = requestOf(withAi ? count : 0, words, withAi && refine, direction, false)
  const asksAi = withAi && (request.count > 0 || request.refine !== null)
  // The same words twice are one card: a second ask with the composer still
  // holding them asks again without putting them on the board twice.
  // A card of the library is the person's when it names the same work.
  const typed = words.trim()
  const alreadyOn =
    typed !== '' &&
    cards.some(
      (card) =>
        card.source === 'own' &&
        (card.entry != null
          ? card.entry.title.trim().toLowerCase() === typed.toLowerCase()
          : headlineOf(board, card) === typed),
    )

  // On the board, the words leave the composer for the next card; a run
  // that works them out has already taken them into its request.
  const putOwn = useAppMutation({
    mutationFn: () => addOwnCard(work.id, board.key, words, ownCardKind?.key),
    failure: 'board.addFailed',
    refresh,
    onSuccess: () => setWords(''),
  })
  const chip = useAppMutation({
    mutationFn: ({ id, words }: { id: string; words: string }) => chipToPhrase(id, words),
    failure: 'board.chipFailed',
    refresh: refreshing.note,
  })
  const judge = useAppMutation({
    mutationFn: ({ id, verdict }: { id: string; verdict: CardVerdict | null }) =>
      judgeCard(id, verdict),
    failure: 'board.judgeFailed',
    refresh,
  })
  const write = useAppMutation({
    mutationFn: ({ id, key, text }: { id: string; key: string; text: string }) =>
      updateCard(id, { fields: { [key]: text.trim() === '' ? null : text } }),
    failure: 'board.writeFailed',
    refresh,
  })
  const move = useAppMutation({
    mutationFn: ({ id, topic, index }: { id: string; topic: string; index: number }) =>
      moveCard(id, topic, index),
    failure: 'board.moveFailed',
    refresh,
  })
  const retopic = useAppMutation({
    mutationFn: ({ from, to }: { from: string; to: string }) =>
      retopicCards(work.id, board.key, from, to),
    failure: 'board.moveFailed',
    refresh,
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteCard(id),
    failure: 'board.deleteFailed',
    onSuccess: (deletionId) =>
      announceDeleted({
        client,
        deletionId,
        message: t('board.deleted'),
        refresh: [...refresh, keys.deletions],
      }),
  })
  const busy = judge.isPending || move.isPending || retopic.isPending || remove.isPending

  const noGrid = useRef<HTMLElement>(null)
  const carry = useChipDrag({
    gridRef: noGrid,
    onEdge: () => {},
    onDrop: (id, target) => {
      const drop = dropOf(target)
      if (drop === null) return
      const card = cards.find((one) => one.id === id)
      if (card === undefined) return
      move.mutate({ id, topic: drop.topic, index: Math.min(drop.index, cards.length) })
    },
  })

  const generate = () => {
    if (hasWords && !alreadyOn) putOwn.mutate(undefined)
    if (asksAi) task.ask(request)
  }
  const dig = (heading: string) => {
    setDirection(heading)
    task.ask(requestOf(count > 0 ? count : CARDS_ON_ASK, '', false, heading, false))
  }
  const more = () =>
    task.ask(requestOf(count > 0 ? count : CARDS_ON_ASK, '', false, direction, true))

  const topics = topicsOf(cards)
  const named = topics.map((topic) => topic.name).filter((name) => name !== '')
  // Read by kind, the board is a reading: its groups are what the cards
  // are, not where the person put them, so nothing is dragged or renamed.
  const byKind = grouping === 'kinds'
  const groups = byKind ? kindsOf(board, library, cards, sayLabel) : topics
  const carried =
    carry.dragging === null ? null : cards.find((card) => card.id === carry.dragging!.id)
  const ownPlaceholder =
    ownCardKind?.library === true
      ? t('board.entryPlaceholder')
      : ownCardKind?.fields[0]?.hint == null
        ? t('board.ownPlaceholder')
        : sayLabel(ownCardKind.fields[0].hint)

  return (
    <section
      aria-label={sayLabel(board.label)}
      className="flex flex-col gap-2.5 rounded-lg border border-line bg-softer px-3 py-2.5"
    >
      <header className="flex flex-col gap-0.5">
        <h3 className="text-sm font-semibold">{sayLabel(board.label)}</h3>
        {board.description != null && (
          <p className="text-xs text-dim">{sayLabel(board.description)}</p>
        )}
      </header>

      {task.action !== undefined && previewing && (
        <TaskPreviewDialog
          open={previewing}
          onOpenChange={setPreviewing}
          target={{ on: 'board', workId: work.id, board: board.key, request }}
          action={task.action}
          before={() =>
            hasWords && !alreadyOn
              ? addOwnCard(work.id, board.key, words, ownCardKind?.key)
              : Promise.resolve(null)
          }
          onStarted={() => {
            task.started(request)
            setPreviewing(false)
          }}
        />
      )}
      <AgentCards workId={work.id} board={board} />

      <Panel className="grid grid-cols-[minmax(0,1fr)_auto] items-end gap-2.5 px-3 py-2.5 max-[900px]:grid-cols-1">
        <div className="flex flex-col gap-2">
          <div className="flex flex-col gap-1">
            <span className="flex items-center gap-2">
              <span className="caption">{t('board.own')}</span>
              {/* A board of several kinds puts the person's card on as the
                  kind they say: a work of culture, a concept. */}
              {board.cards.length > 1 && (
                <Select
                  value={ownCardKind?.key ?? ''}
                  onChange={(next) => {
                    if (next !== '') setOwnKind(next)
                  }}
                  options={board.cards.map((kind) => ({
                    value: kind.key,
                    label: sayLabel(kind.label),
                  }))}
                  aria-label={t('board.ownKind')}
                  className="h-7 w-44"
                />
              )}
            </span>
            <Textarea
              rows={1}
              autoResize
              maxRows={4}
              value={words}
              aria-label={t('board.own')}
              placeholder={ownPlaceholder}
              onChange={(event) => setWords(event.target.value)}
            />
          </div>
          {withAi && (
            <label className="flex flex-col gap-1">
              <span className="caption">{t('board.direction')}</span>
              <Input
                value={direction}
                placeholder={t('board.directionPlaceholder')}
                onChange={(event) => setDirection(event.target.value)}
              />
            </label>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-3">
          {withAi && (
            <>
              <Checkbox
                checked={refine && hasWords}
                disabled={!hasWords}
                onCheckedChange={(checked) => setRefine(checked === true)}
              >
                {t('board.refine')}
              </Checkbox>
              <span className="caption">{t('board.fromAi')}</span>
              <SegmentedControl
                aria-label={t('board.fromAi')}
                value={String(count)}
                onValueChange={(next) => setCount(Number(next))}
              >
                {Array.from({ length: MOST_CARDS + 1 }, (_, n) => (
                  <Segment key={n} value={String(n)}>
                    {n}
                  </Segment>
                ))}
              </SegmentedControl>
            </>
          )}
          <Button
            size="sm"
            variant="primary"
            disabled={(!asksAi && !(hasWords && !alreadyOn)) || task.writing || task.asking}
            onClick={generate}
          >
            <Sparkles aria-hidden />
            {asksAi ? t('board.generate') : t('board.putOwn')}
          </Button>
          {asksAi && (
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t('board.preview')}
              title={t('board.preview')}
              disabled={task.writing || task.asking}
              onClick={() => setPreviewing(true)}
            >
              <Eye aria-hidden />
            </Button>
          )}
          {withAi && (
            <Button
              size="sm"
              variant="ghost"
              disabled={tally.star === 0 || task.writing || task.asking}
              disabledReason={tally.star === 0 ? t('board.moreNeedsStars') : undefined}
              title={t('board.moreHint')}
              onClick={more}
            >
              {t('board.more', { stars: tally.star })}
            </Button>
          )}
        </div>
      </Panel>

      {withAi && (view?.directions ?? []).length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="caption flex items-center gap-1">
            <Compass aria-hidden className="size-3.5" />
            {t('board.dig')}
          </span>
          {(view?.directions ?? []).map((heading) => (
            <Button
              key={heading}
              size="xs"
              variant="soft"
              disabled={task.writing || task.asking}
              title={t('board.digHint', { direction: heading })}
              onClick={() => dig(heading)}
            >
              {heading}
            </Button>
          ))}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <ChipGroup
          aria-label={t('board.filter')}
          value={[filter]}
          onValueChange={(next) => {
            const [picked] = next
            if (picked !== undefined) setFilter(picked as CardFilter)
          }}
        >
          {CARD_FILTERS.map((one) => (
            <Chip key={one} value={one}>
              {t(`board.filters.${one}`)}
              <span className="font-mono text-2xs">{tally[one]}</span>
            </Chip>
          ))}
        </ChipGroup>
        {/* A board of several kinds is read by its topics - the angles the
            person gave it - or by what its cards are. */}
        {board.cards.length > 1 && cards.length > 0 && (
          <SegmentedControl
            aria-label={t('board.grouping')}
            value={grouping}
            onValueChange={(next) => setGrouping(next as Grouping)}
          >
            <Segment value="topics">{t('board.byTopic')}</Segment>
            <Segment value="kinds">{t('board.byKind')}</Segment>
          </SegmentedControl>
        )}
        {task.writing && (
          <span className="ml-auto flex items-center gap-2 text-xs text-dim" role="status">
            <span aria-hidden className="size-2 animate-pulse rounded-full bg-accent" />
            {t('board.writing', { count: task.coming })}
            {task.since !== null && (
              <span className="font-mono tabular-nums">{clockOf(task.since)}</span>
            )}
            <Button size="xs" variant="ghost" onClick={task.stop}>
              {t('board.stop')}
            </Button>
          </span>
        )}
      </div>

      {naming !== null && (
        <form
          className="flex items-center gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            const name = topicName.trim()
            if (name === '') return
            move.mutate({ id: naming, topic: name, index: Number.MAX_SAFE_INTEGER })
            setNaming(null)
            setTopicName('')
          }}
        >
          <Input
            autoFocus
            value={topicName}
            aria-label={t('board.newTopic')}
            placeholder={t('board.newTopicPlaceholder')}
            onChange={(event) => setTopicName(event.target.value)}
          />
          <Button type="submit" size="sm" variant="primary" disabled={topicName.trim() === ''}>
            {t('board.putUnder')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setNaming(null)}>
            {t('board.cancel')}
          </Button>
        </form>
      )}

      {task.writing && filter === 'all' && (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(232px,1fr))] gap-2.5">
          {Array.from({ length: task.coming }, (_, index) => (
            <CardSkeleton key={index} />
          ))}
        </div>
      )}

      {groups.map((topic) => {
        const shown = topic.cards.filter((card) => shows(card, filter))
        if (shown.length === 0 && (carry.dragging === null || byKind)) return null
        return (
          <div
            key={topic.name}
            data-topic-drop={byKind ? undefined : topic.name}
            className={cn(
              'flex flex-col gap-1.5 rounded-md',
              carry.dragging !== null && !byKind && 'outline-1 outline-line-2 outline-dashed',
            )}
          >
            {byKind ? (
              <h4 className="caption flex items-center gap-2 px-0.5">
                {topic.name}
                <span className="font-mono text-2xs text-faint">{topic.cards.length}</span>
              </h4>
            ) : (
              (groups.length > 1 || topic.name !== '') && (
                <TopicHeading
                  name={topic.name}
                  count={topic.cards.length}
                  onRename={(to) => retopic.mutate({ from: topic.name, to })}
                />
              )
            )}
            <div className="grid grid-cols-[repeat(auto-fill,minmax(232px,1fr))] items-stretch gap-2.5">
              {shown.map((card) => (
                <BoardCardItem
                  key={card.id}
                  board={board}
                  card={card}
                  library={library}
                  elsewhere={elsewhereOf(view, card)}
                  kept={kept}
                  slot={topic.cards.indexOf(card)}
                  fresh={task.fresh.has(card.id)}
                  carried={carry.dragging?.id === card.id}
                  movable={!byKind}
                  busy={busy || chip.isPending}
                  onVerdict={(verdict) => judge.mutate({ id: card.id, verdict })}
                  onField={(key, text) => write.mutate({ id: card.id, key, text })}
                  onDelete={() => remove.mutate(card.id)}
                  onChip={(words) => chip.mutate({ id: card.id, words })}
                  onGrab={(event) => carry.begin(event, card.id)}
                  topicMenu={
                    <RowMenu
                      label={t('board.topicMenu')}
                      actions={[
                        ...named
                          .filter((name) => name !== card.topic)
                          .map((name) => ({
                            key: `topic:${name}`,
                            label: t('board.moveUnder', { topic: name }),
                            onSelect: () =>
                              move.mutate({
                                id: card.id,
                                topic: name,
                                index: Number.MAX_SAFE_INTEGER,
                              }),
                          })),
                        ...(card.topic === ''
                          ? []
                          : [
                              {
                                key: 'no-topic',
                                label: t('board.noTopic'),
                                onSelect: () =>
                                  move.mutate({
                                    id: card.id,
                                    topic: '',
                                    index: Number.MAX_SAFE_INTEGER,
                                  }),
                              },
                            ]),
                        {
                          key: 'new-topic',
                          label: t('board.newTopic'),
                          onSelect: () => {
                            setTopicName('')
                            setNaming(card.id)
                          },
                        },
                      ]}
                    />
                  }
                />
              ))}
            </div>
          </div>
        )
      })}

      {cards.length === 0 && !task.writing && (
        <p className="rounded-lg border border-dashed border-line-2 bg-softer px-4 py-3.5 text-sm text-dim">
          {t('board.empty')}
        </p>
      )}
      {cards.length > 0 && cards.every((card) => !shows(card, filter)) && (
        <p className="text-sm text-dim">{t('board.emptyFilter')}</p>
      )}

      {carry.dragging !== null && carried != null && (
        <div
          aria-hidden
          className="pointer-events-none fixed z-50 rounded-lg border border-accent bg-raise px-2.5 py-2 text-sm font-semibold shadow-lg"
          style={{
            left: carry.dragging.ghost.left,
            top: carry.dragging.ghost.top,
            width: carry.dragging.size.width,
          }}
        >
          {headlineOf(board, carried)}
        </div>
      )}
    </section>
  )
}

/** A topic's heading: its name, written in place - renamed onto another
 *  topic's name it merges into that one - and how many cards stand under it. */
function TopicHeading({
  name,
  count,
  onRename,
}: {
  name: string
  count: number
  onRename: (to: string) => void
}) {
  const { t } = useTranslation()
  const draft = useFieldDraft(name, (text) => {
    if (text.trim() !== name) onRename(text.trim())
  })
  if (name === '') {
    return <h4 className="caption px-0.5">{t('board.noTopicHeading', { count })}</h4>
  }
  return (
    <div className="flex items-center gap-2">
      <Input
        aria-label={t('board.topicName')}
        title={t('board.topicHint')}
        {...draft}
        className="h-7 w-auto max-w-80 border-transparent bg-transparent px-1 text-xs font-semibold tracking-caption uppercase hover:border-line-2 focus:bg-soft"
      />
      <span className="font-mono text-2xs text-faint">{count}</span>
    </div>
  )
}

/**
 * Cards an agent proposed for this board from outside the window: they wait
 * here, above the cards, until the person puts them on the board or turns
 * them down - an agent only ever proposes (ADR 0018).
 */
function AgentCards({ workId, board }: { workId: string; board: Board }) {
  const { t } = useTranslation()
  // Asked afresh whenever the board is opened, and every half minute while
  // it is, as the cover's board asks: an agent proposes from another process.
  const pending = useQuery({
    ...queries.pendingProposals(),
    staleTime: 0,
    refetchInterval: 30_000,
  })
  const waiting = (pending.data ?? []).filter(
    (one) => one.kind === 'cards' && one.work_id === workId && one.board === board.key,
  )
  const refresh = [keys.boards(workId), keys.pendingProposals, keys.transcripts]
  const put = useAppMutation({
    mutationFn: (messageId: string) => applyProposal(messageId),
    failure: 'board.putFailed',
    refresh,
  })
  const dismiss = useAppMutation({
    mutationFn: (messageId: string) => dismissProposal(messageId),
    failure: 'assistant.dismissFailed',
    refresh,
  })
  if (waiting.length === 0) return null
  return (
    <ul className="flex flex-col gap-1.5">
      {waiting.map((one) => (
        <li
          key={one.message_id}
          className="flex flex-wrap items-center gap-2 rounded-lg bg-info-soft px-3 py-1.5 text-xs text-info"
        >
          <b className="font-semibold">
            {t('board.agent', {
              client: one.chat_title ?? t('ideas.anAgent'),
              board: sayLabel(board.label),
            })}
          </b>
          <span className="ml-auto flex gap-1.5">
            <Button
              size="xs"
              variant="primary"
              disabled={put.isPending}
              onClick={() => put.mutate(one.message_id)}
            >
              {t('board.putProposed')}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              disabled={dismiss.isPending}
              onClick={() => dismiss.mutate(one.message_id)}
            >
              {t('assistant.dismiss')}
            </Button>
          </span>
        </li>
      ))}
    </ul>
  )
}
