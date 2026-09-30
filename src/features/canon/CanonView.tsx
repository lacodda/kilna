import { useState } from 'react'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { BookPlus, CalendarRange, Plus } from 'lucide-react'
import { startTask } from '@/lib/api/assistant'
import type { FactStatus } from '@/lib/api/types'
import { gatherAction } from '@/lib/actions'
import { hasCanon, LENS_CHOICES, type LensChoice } from '@/lib/canon'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useAssistant } from '@/lib/useAssistant'
import { useDebounced } from '@/lib/useDebounced'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame } from '@/components/frame'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { CanonTimeline } from '@/features/canon/CanonTimeline'
import { CardDetail } from '@/features/canon/CardDetail'
import { CardList } from '@/features/canon/CardList'
import { CardSide } from '@/features/canon/CardSide'
import { NewCardDialog } from '@/features/canon/NewCardDialog'

/**
 * The canon: the world a channel's works are made from, kept as facts on
 * cards (ADR 0043).
 *
 * Three columns, as the mockup draws them: the cards by kind on the left, the
 * open card in the middle - its sections and their facts, each fact with its
 * layer, state, source and time - and on the right its pictures, the cards it
 * stands beside and what the assistant proposes for it. Across the top the
 * lens: the card read through the eyes of a task, the facts that task may
 * not see dimmed. The lens is not a filter of the screen's own: which facts a
 * task sees is the backend's answer, the same one every prompt reads through.
 *
 * The open card is part of the address, so back walks between cards and a
 * search hit lands on one. With no card open the middle can hold the
 * timeline instead - every dated fact in the order it happened - and that is
 * in the address too (`?timeline`).
 */
export function CanonView() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const assistant = useAssistant()
  const { cardId } = useParams()
  const [params] = useSearchParams()
  const timeline = cardId === undefined && params.has('timeline')
  const { config } = useProfile()

  const [lens, setLens] = useState<LensChoice>('all')
  const [text, setText] = useState('')
  const [status, setStatus] = useState<FactStatus | undefined>(undefined)
  const [creating, setCreating] = useState(false)
  const [gathering, setGathering] = useState(false)
  const query = useDebounced(text.trim(), 200)

  const cards = useQuery(
    queries.cardsMatching({ search: query === '' ? undefined : query, status }),
  )
  const everything = useQuery(queries.cardsMatching({}))
  const card = useQuery({ ...queries.card(cardId ?? ''), enabled: cardId !== undefined })

  const open = (id: string | null) => void navigate(id === null ? '/canon' : `/canon/${id}`)

  // "Gather facts from a song": the profile's work action that proposes for
  // the canon, started on a work picked here. Its answer lands in a chat of
  // its own; the drawer opens on it.
  const gather = gatherAction(config.prompts ?? [])
  const start = useAppMutation({
    mutationFn: (workId: string) => startTask(workId, gather?.key ?? '', {}),
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
      assistant.open(started.chatId)
    },
  })

  if (!hasCanon(config)) {
    return (
      <Frame>
        <EmptyState title={t('canon.noCanon')} body={t('canon.noCanonBody')} className="flex-1" />
      </Frame>
    )
  }

  const lensHint = t(`canon.lensHint.${lens}`)

  return (
    <Frame
      head={
        <>
          <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2.5 rounded-lg border border-line bg-soft/40 px-2.5 py-1.5">
            <span className="text-2xs font-semibold tracking-caption text-faint uppercase">
              {t('canon.lens')}
            </span>
            <SegmentedControl
              aria-label={t('canon.lens')}
              value={lens}
              onValueChange={(next) => setLens(next as LensChoice)}
            >
              {LENS_CHOICES.map((one) => (
                <Segment key={one} value={one}>
                  {t(`canon.lensName.${one}`)}
                </Segment>
              ))}
            </SegmentedControl>
            <span className="min-w-48 flex-1 text-xs text-faint">{lensHint}</span>
          </div>
          <Button
            variant="ghost"
            aria-pressed={timeline}
            onClick={() => void navigate(timeline ? '/canon' : '/canon?timeline')}
          >
            <CalendarRange aria-hidden />
            {t('canon.timeline')}
          </Button>
          {gather !== undefined && (
            <Button variant="ghost" onClick={() => setGathering(true)} disabled={start.isPending}>
              <BookPlus aria-hidden />
              {sayLabel(gather.label)}
            </Button>
          )}
          <Button variant="primary" onClick={() => setCreating(true)}>
            <Plus aria-hidden />
            {t('canon.newCard')}
          </Button>
        </>
      }
    >
      <div className="grid min-h-0 flex-1 grid-cols-[232px_minmax(0,1fr)_296px] gap-2.5 max-[1100px]:grid-cols-[208px_minmax(0,1fr)]">
        <CardList
          query={cards}
          total={everything.data?.length ?? 0}
          text={text}
          onText={setText}
          status={status}
          onStatus={setStatus}
          openId={cardId ?? null}
          onOpen={open}
        />
        {timeline ? (
          <CanonTimeline lens={lens} onOpen={open} />
        ) : cardId === undefined ? (
          <EmptyState
            className="flex-1"
            title={(everything.data?.length ?? 0) === 0 ? t('canon.empty') : t('canon.pick')}
            body={(everything.data?.length ?? 0) === 0 ? t('canon.emptyBody') : undefined}
            action={
              (everything.data?.length ?? 0) === 0 ? (
                <Button variant="primary" onClick={() => setCreating(true)}>
                  <Plus aria-hidden />
                  {t('canon.newCard')}
                </Button>
              ) : undefined
            }
          />
        ) : card.data !== undefined ? (
          <CardDetail
            key={card.data.card.id}
            view={card.data}
            lens={lens}
            onOpen={open}
            onGone={() => open(null)}
          />
        ) : card.isError ? (
          <EmptyState className="flex-1" title={t('canon.gone')} body={t('canon.goneBody')} />
        ) : (
          <Skeleton className="flex-1 rounded-lg" />
        )}
        {card.data !== undefined && cardId !== undefined ? (
          <div className="flex min-h-0 flex-col max-[1100px]:hidden">
            <CardSide view={card.data} lens={lens} onOpen={open} />
          </div>
        ) : (
          <div className="max-[1100px]:hidden" />
        )}
      </div>

      {creating && (
        <NewCardDialog
          open
          onOpenChange={setCreating}
          cards={everything.data ?? []}
          onCreated={(id) => {
            setCreating(false)
            open(id)
          }}
        />
      )}
      {gathering && gather !== undefined && (
        <PickWorkDialog
          open
          onOpenChange={setGathering}
          title={sayLabel(gather.label)}
          onPick={(work) => {
            setGathering(false)
            start.mutate(work.work_id)
          }}
        />
      )}
    </Frame>
  )
}
