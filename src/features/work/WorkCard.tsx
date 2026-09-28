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
import { canBeCut } from '@/lib/cuts'
import { hasScenes, useProfile } from '@/lib/useProfile'
import { BlindJudgingContext } from '@/lib/blindJudging'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { QueryState } from '@/components/ui/query-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Frame } from '@/components/frame'
import { CardHeader } from '@/features/work/CardHeader'
import { DEFAULT_TAB, isTab, tabsOf } from '@/features/work/tabs'
import { storedCardView } from '@/features/work/cardView'
import { TabBody } from '@/features/work/TabBody'

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

  // Only for the count on the tab; the tab itself fetches what it draws.
  const releases = useQuery(queries.releasesForWork(workId))
  // For the count on the tab: "Links (2)" is how a song shows it has clips
  // without anyone opening the tab.
  const links = useQuery(queries.links(workId))
  // The storyboard is a fact of the kind: a song has none, and asking for
  // its scenes would be a query for a tab that is not drawn.
  const storyboard = hasScenes(profile.config, work.data?.kind)
  const scenes = useQuery({ ...queries.scenes(workId), enabled: storyboard })

  // The splice, for the tab and its count. Always asked: unlike a storyboard,
  // whether a work was cut out of another is a fact about the work rather than
  // about its kind, and the only way to know is to look.
  const cuts = useQuery(queries.cuts(workId))
  const spliced = canBeCut(cuts.data ?? [], links.data?.sources.length ?? 0)

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

  // A URL naming no tab opens on the one this machine prefers; one naming a
  // tab that does not exist is corrected rather than shown empty. `replace`
  // keeps the bad address out of the history either way.
  if (!isTab(tab)) {
    return <Navigate to={`/works/${workId}/${storedCardView().defaultTab}`} replace />
  }

  const current = work.data

  // A tab this work does not have - a storyboard on a song, a splice on a work
  // cut from nothing - goes to the default one, by the same rule the tab bar
  // hides it. It used to draw an empty tab with nothing lit in the bar. Whether
  // a work was cut is only known once its splice has loaded: until then the
  // address is trusted rather than bounced.
  const settled = cuts.isSuccess && links.isSuccess
  const tabs = tabsOf({ storyboard, splice: settled ? spliced : true })
  if (!tabs.includes(tab)) {
    return <Navigate to={`/works/${workId}/${DEFAULT_TAB}`} replace />
  }

  return (
    <BlindJudgingContext value={{ blind, revealed, setBlind, setRevealed }}>
      <div className="flex min-h-0 flex-1 flex-col">
        <CardHeader
          work={current}
          tabs={tabsOf({ storyboard, splice: spliced })}
          counts={{
            releases: releases.data?.length ?? 0,
            links: (links.data?.sources.length ?? 0) + (links.data?.derived.length ?? 0),
            scenes: scenes.data?.length ?? 0,
            cuts: cuts.data?.length ?? 0,
          }}
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
