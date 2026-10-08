import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { deleteWork } from '@/lib/api/works'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { noteDeleted, noteOpened } from '@/lib/recent'
import { announceDeleted } from '@/lib/trash'
import { useProfile } from '@/lib/useProfile'
import { BlindJudgingContext } from '@/lib/blindJudging'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { QueryState } from '@/components/ui/query-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame } from '@/components/frame'
import { CardHeader } from '@/features/work/CardHeader'
import {
  DEFAULT_TAB,
  factsOf,
  isTab,
  NOTHING_COUNTED,
  openingTab,
  tabCounts,
  tabsOf,
} from '@/features/work/tabs'
import { TabBody } from '@/features/work/TabBody'
import { WorkPlayerProvider } from '@/features/work/player'

interface Props {
  workId: string
  /** Which tab the URL asked for; anything unknown falls back. */
  tab: string | undefined
  onDeleted: () => void
  /** Reopen a work that was deleted and then brought back. */
  onUndone: (workId: string) => void
}

/**
 * A work, opened.
 *
 * The card is a frame — a header, a tab bar and one body — rather than the seven
 * stacked panels it used to be. Each tab is a component of its own with its own
 * queries, so opening a card no longer loads everything a work has ever had.
 */
export function WorkCard({ workId, tab, onDeleted, onUndone }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const profile = useProfile()
  const work = useQuery(queries.work(workId))
  // Judging blind belongs to the card, not the Score tab: the header above the
  // tab shows the last verdict too (see `lib/blindJudging`). The card is keyed
  // by the work, so a new work starts with it off.
  const [blind, setBlind] = useState(false)
  const [revealed, setRevealed] = useState(false)

  // Noted once the title is known, since the list shows names rather than ids.
  // Keyed on both, so a rename while the card is open updates the entry rather
  // than leaving the old name to be offered tomorrow.
  const title = work.data?.title
  useEffect(() => {
    if (title !== undefined) noteOpened({ id: workId, title })
  }, [workId, title])

  // The numbers beside the tabs, in one answer; each tab fetches what it
  // draws itself. Until v0.80 the card fetched every release, link, scene
  // and stretch of the work to read four lengths. Refreshed with every write
  // the card makes, from any tab (see `keys.cardCounts`). They are also what
  // the work holds, which keeps a tab its kind no longer names (`factsOf`).
  const counts = useQuery(queries.cardCounts(workId))

  const remove = useAppMutation({
    mutationFn: () => deleteWork(workId),
    failure: 'toast.workDeleteFailed',
    onSuccess: (deletionId) => {
      // Out of the recent list too, so it cannot be offered after it is gone.
      // An undo re-opens the card, which puts it back.
      noteDeleted(workId)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.workDeleted', { title: work.data?.title ?? '' }),
        refresh: refresh.works,
        // The card was closed on the way out; an undo brings it back open.
        onUndone: () => onUndone(workId),
      })
      onDeleted()
    },
  })

  if (work.isPending || work.isError) {
    return (
      <Frame>
        <QueryState
          pending={work.isPending}
          error={work.isError ? t('toast.loadFailed') : null}
          skeleton={<SkeletonCard />}
          onRetry={() => void work.refetch()}
          retryLabel={t('crash.retry')}
        >
          {null}
        </QueryState>
      </Frame>
    )
  }

  // The row is gone — deleted in another view while this one held its id.
  // The way out is where it was listed.
  if (work.data === null) {
    return (
      <Frame>
        <EmptyState
          title={t('error.notFound')}
          action={
            <Button size="sm" render={<Link to="/catalogue" />}>
              {t('nav.catalogue')}
            </Button>
          }
          className="flex-1"
        />
      </Frame>
    )
  }

  const current = work.data

  // A URL naming no tab opens on the one the work's kind names; one naming a
  // tab that does not exist is corrected rather than shown empty. `replace`
  // keeps the bad address out of the history either way.
  if (!isTab(tab)) {
    return <Navigate to={`/works/${workId}/${openingTab(profile.config, current.kind)}`} replace />
  }

  // A tab this work does not have - a storyboard on a song, a splice on a work
  // cut from nothing - goes to the default one, by the same rule the tab bar
  // hides it. It used to draw an empty tab with nothing lit in the bar. What a
  // work holds is only known once its counts have loaded: until then the
  // address is trusted rather than bounced.
  const tabs = tabsOf(factsOf(profile.config, current, counts.data))
  if (!tabs.includes(tab)) {
    return <Navigate to={`/works/${workId}/${DEFAULT_TAB}`} replace />
  }

  return (
    <BlindJudgingContext value={{ blind, revealed, setBlind, setRevealed }}>
      {/* The player belongs to the card, not to a tab: a song keeps playing
          while its text is read or its score given, and a file's tile on the
          Files tab starts it here. */}
      <WorkPlayerProvider>
        <div className="flex min-h-0 flex-1 flex-col">
          <CardHeader
            work={current}
            tabs={tabsOf(factsOf(profile.config, current, counts.data ?? NOTHING_COUNTED))}
            counts={counts.data === undefined ? {} : tabCounts(counts.data)}
            onDelete={() => remove.mutate()}
          />

          {/* The header stands and the open tab takes the rest. No tab is a
            page that scrolls within this box: each lays itself out against
            its height on `components/frame` and scrolls inside, so the card
            itself never scrolls - which is what let the header stop being
            sticky, and what keeps a tab's own head and foot on screen.
            `data-tab-body` is how the smoke test holds every tab to it. */}
          <div
            data-tab-body
            className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-b-xl border border-line p-3"
          >
            <TabBody tab={tab} workId={workId} work={current} />
          </div>
        </div>
      </WorkPlayerProvider>
    </BlindJudgingContext>
  )
}

/** The card's own shape while it loads: the header's band and title, then a
 *  panel the height of a tab. A list of rows here would be the jump it is
 *  meant to cover. */
function SkeletonCard() {
  return (
    <div className="flex flex-col gap-3" aria-hidden>
      <Skeleton className="h-16 w-full rounded-b-none" />
      <div className="flex flex-wrap items-end gap-3">
        <Skeleton className="h-control w-64" />
        <Skeleton className="h-control w-44" />
      </div>
      <Skeleton className="h-64 w-full" />
    </div>
  )
}
