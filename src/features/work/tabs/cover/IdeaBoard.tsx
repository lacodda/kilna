import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Eye, Sparkles } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import { applyProposal, dismissProposal } from '@/lib/api/assistant'
import {
  addOwnIdea,
  deleteIdea,
  judgeIdea,
  judgeSiblingCover,
  takeIdea,
  takeSiblingCover,
} from '@/lib/api/ideas'
import type { CoverBoard, IdeaVerdict, Work } from '@/lib/api/types'
import {
  BOARD_FILTERS,
  MOST_IDEAS,
  cardsOf,
  counts,
  IDEAS_ON_MAKE,
  ideasOnMake,
  requestOf,
  shows,
  type BoardCard,
  type BoardFilter,
} from '@/lib/ideas'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { Scroll } from '@/components/frame'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'
import { IdeaCard, IdeaSkeleton } from '@/features/work/tabs/cover/IdeaCard'
import { clockOf, useCoverIdeas } from '@/features/work/tabs/cover/useCoverIdeas'

interface Props {
  work: Work
  board: CoverBoard
  /** An idea was taken into the constructor: show it there. */
  onTaken: () => void
}

/**
 * The board of ideas on the Cover tab (v0.89, ADR 0050): the mockup's
 * composer - the person's own idea, whether to have it worked out, how many
 * ideas of the assistant's own - the filters with their counts, the line of
 * the run going, and the cards, skeletons first while ideas are written.
 *
 * There is no master: a cover starts from here or from the constructor
 * directly, and an idea taken in is an ordinary edit of the cover.
 */
export function IdeaBoard({ work, board, onTaken }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const ideas = useCoverIdeas(work)
  const [words, setWords] = useState('')
  const [refine, setRefine] = useState(true)
  const [count, setCount] = useState(() => ideasOnMake(config) || IDEAS_ON_MAKE)
  const [filter, setFilter] = useState<BoardFilter>('all')
  const [previewing, setPreviewing] = useState(false)
  const refresh = [keys.coverBoard(work.id), keys.work(work.id)]

  const cards = cardsOf(board)
  const tally = counts(cards)
  const shown = cards.filter((card) => shows(card, filter))
  const hasWords = words.trim() !== ''
  const withAi = ideas.action !== undefined
  const request = requestOf(withAi ? count : 0, words, withAi && refine)
  const asksAi = withAi && (request.count > 0 || request.refine !== null)

  // The same words twice are one idea: a second Generate with the composer
  // still holding them asks again without putting them on the board twice.
  const alreadyOn = board.ideas.some(
    (card) => card.idea.source === 'own' && card.idea.concept.idea.trim() === words.trim(),
  )

  const putOwn = useAppMutation({
    mutationFn: () => addOwnIdea(work.id, words),
    failure: 'ideas.addFailed',
    refresh,
  })
  const judge = useAppMutation({
    mutationFn: ({ id, verdict }: { id: string; verdict: IdeaVerdict | null }) =>
      judgeIdea(id, verdict),
    failure: 'ideas.judgeFailed',
    refresh,
  })
  const judgeSibling = useAppMutation({
    mutationFn: ({ sibling, verdict }: { sibling: string; verdict: IdeaVerdict }) =>
      judgeSiblingCover(work.id, sibling, verdict),
    failure: 'ideas.judgeFailed',
    refresh,
  })
  const take = useAppMutation({
    mutationFn: (card: BoardCard) =>
      card.kind === 'idea'
        ? takeIdea(card.card.idea.id)
        : takeSiblingCover(work.id, card.sibling.work_id),
    failure: 'ideas.takeFailed',
    refresh: [...refresh, keys.pictures, keys.covers],
    onSuccess: () => onTaken(),
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteIdea(id),
    failure: 'ideas.deleteFailed',
    refresh: [...refresh, keys.deletions],
  })
  const busy = judge.isPending || judgeSibling.isPending || take.isPending || remove.isPending

  const generate = () => {
    if (hasWords && !alreadyOn) putOwn.mutate(undefined)
    if (asksAi) ideas.ask(request)
  }
  const stars = tally.star
  const more = () =>
    ideas.ask({
      count: count > 0 ? count : ideasOnMake(config) || IDEAS_ON_MAKE,
      refine: null,
      more: true,
    })

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
      {ideas.action !== undefined && previewing && (
        <TaskPreviewDialog
          open={previewing}
          onOpenChange={setPreviewing}
          target={{ on: 'cover', workId: work.id, request }}
          action={ideas.action}
          before={() =>
            hasWords && !alreadyOn ? addOwnIdea(work.id, words) : Promise.resolve(null)
          }
          onStarted={() => {
            ideas.started(request)
            setPreviewing(false)
          }}
        />
      )}
      <AgentIdeas workId={work.id} />

      <section
        aria-label={t('ideas.composer')}
        className="grid shrink-0 grid-cols-[minmax(0,1fr)_auto] items-end gap-2.5 rounded-lg border border-line bg-raise px-3 py-2.5 max-[900px]:grid-cols-1"
      >
        <label className="flex flex-col gap-1">
          <span className="caption">{t('ideas.own')}</span>
          <Textarea
            rows={2}
            autoResize
            maxRows={6}
            value={words}
            placeholder={t('ideas.ownPlaceholder')}
            onChange={(event) => setWords(event.target.value)}
          />
        </label>
        <div className="flex flex-wrap items-center gap-3">
          {withAi && (
            <>
              <Checkbox
                checked={refine && hasWords}
                disabled={!hasWords}
                onCheckedChange={(checked) => setRefine(checked === true)}
              >
                {t('ideas.refine')}
              </Checkbox>
              <span className="caption">{t('ideas.fromAi')}</span>
              <SegmentedControl
                aria-label={t('ideas.fromAi')}
                value={String(count)}
                onValueChange={(next) => setCount(Number(next))}
              >
                {Array.from({ length: MOST_IDEAS + 1 }, (_, n) => (
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
            disabled={(!asksAi && !(hasWords && !alreadyOn)) || ideas.writing || ideas.asking}
            onClick={generate}
          >
            <Sparkles aria-hidden />
            {asksAi ? t('ideas.generate') : t('ideas.putOwn')}
          </Button>
          {asksAi && (
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t('ideas.preview')}
              title={t('ideas.preview')}
              disabled={ideas.writing || ideas.asking}
              onClick={() => setPreviewing(true)}
            >
              <Eye aria-hidden />
            </Button>
          )}
          {withAi && (
            <Button
              size="sm"
              variant="ghost"
              disabled={stars === 0 || ideas.writing || ideas.asking}
              disabledReason={stars === 0 ? t('ideas.moreNeedsStars') : undefined}
              title={t('ideas.moreHint')}
              onClick={more}
            >
              {tally.rejected > 0
                ? t('ideas.moreBoth', { stars, rejected: tally.rejected })
                : t('ideas.more', { stars })}
            </Button>
          )}
        </div>
      </section>

      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <ChipGroup
          aria-label={t('ideas.filter')}
          value={[filter]}
          onValueChange={(next) => {
            const [picked] = next
            if (picked !== undefined) setFilter(picked as BoardFilter)
          }}
        >
          {BOARD_FILTERS.map((one) => (
            <Chip key={one} value={one}>
              {t(`ideas.filters.${one}`)}
              <span className="font-mono text-2xs">{tally[one]}</span>
            </Chip>
          ))}
        </ChipGroup>
        {ideas.writing && (
          <span className="ml-auto flex items-center gap-2 text-xs text-dim" role="status">
            <span aria-hidden className="size-2 animate-pulse rounded-full bg-accent" />
            {t('ideas.writing', { count: ideas.coming })}
            {ideas.since !== null && (
              <span className="font-mono tabular-nums">{clockOf(ideas.since)}</span>
            )}
            <Button size="xs" variant="ghost" onClick={ideas.stop}>
              {t('ideas.stop')}
            </Button>
          </span>
        )}
      </div>

      <Scroll label={t('ideas.board')} className="min-h-0">
        <div className="grid grid-cols-[repeat(auto-fill,minmax(212px,1fr))] items-stretch gap-2.5 pr-0.75 pb-2.5">
          {ideas.writing &&
            filter === 'all' &&
            Array.from({ length: ideas.coming }, (_, index) => <IdeaSkeleton key={index} />)}
          {shown.map((card) =>
            card.kind === 'idea' ? (
              <IdeaCard
                key={card.card.idea.id}
                source={card.card.idea.source}
                from={card.card.from_title}
                angle={card.card.idea.angle}
                headline={card.card.idea.headline}
                idea={card.card.idea.concept.idea}
                look={card.card.look}
                verdict={card.card.idea.verdict}
                fresh={ideas.fresh.has(card.card.idea.id)}
                busy={busy}
                onVerdict={(verdict) => judge.mutate({ id: card.card.idea.id, verdict })}
                onTake={() => take.mutate(card)}
                onDelete={() => remove.mutate(card.card.idea.id)}
              />
            ) : (
              <IdeaCard
                key={`sibling-${card.sibling.work_id}`}
                source="offered"
                from={card.sibling.title}
                angle=""
                headline={card.sibling.title}
                idea={card.sibling.concept.idea}
                look={card.sibling.look}
                picture={card.sibling.picture === null ? null : fileSrc(card.sibling.picture.path)}
                verdict={null}
                busy={busy}
                onVerdict={(verdict) => {
                  if (verdict !== null) {
                    judgeSibling.mutate({ sibling: card.sibling.work_id, verdict })
                  }
                }}
                onTake={() => take.mutate(card)}
              />
            ),
          )}
          {shown.length === 0 && !ideas.writing && (
            <p className="col-span-full rounded-lg border border-dashed border-line-2 bg-softer px-4 py-3.5 text-sm text-dim">
              {filter === 'all' ? t('ideas.empty') : t('ideas.emptyFilter')}
            </p>
          )}
        </div>
      </Scroll>
    </div>
  )
}

/**
 * Ideas an agent proposed for this board from outside the window: they wait
 * here, above the cards, until the person puts them on the board or turns
 * them down - an agent only ever proposes (ADR 0018).
 */
function AgentIdeas({ workId }: { workId: string }) {
  const { t } = useTranslation()
  // Asked afresh whenever the board is opened, and every half minute while it
  // is, as the bell asks: an agent proposes from another process, and the
  // window hears of it only by asking - a cached "nothing" hid ideas an agent
  // had just proposed (found by the live run of v0.89).
  const pending = useQuery({
    ...queries.pendingProposals(),
    staleTime: 0,
    refetchInterval: 30_000,
  })
  const waiting = (pending.data ?? []).filter(
    (one) => one.kind === 'coverIdeas' && one.work_id === workId,
  )
  const refresh = [keys.coverBoard(workId), keys.pendingProposals, keys.transcripts]
  const put = useAppMutation({
    mutationFn: (messageId: string) => applyProposal(messageId),
    failure: 'ideas.putFailed',
    refresh,
  })
  const dismiss = useAppMutation({
    mutationFn: (messageId: string) => dismissProposal(messageId),
    failure: 'assistant.dismissFailed',
    refresh,
  })
  if (waiting.length === 0) return null
  return (
    <ul className="flex shrink-0 flex-col gap-1.5">
      {waiting.map((one) => (
        <li
          key={one.message_id}
          className="flex flex-wrap items-center gap-2 rounded-lg bg-info-soft px-3 py-1.5 text-xs text-info"
        >
          <b className="font-semibold">
            {t('ideas.agent', { client: one.chat_title ?? t('ideas.anAgent') })}
          </b>
          <span className="ml-auto flex gap-1.5">
            <Button
              size="xs"
              variant="primary"
              disabled={put.isPending}
              onClick={() => put.mutate(one.message_id)}
            >
              {t('ideas.putProposed')}
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
