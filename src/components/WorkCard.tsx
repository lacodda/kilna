import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Trash2 } from 'lucide-react'
import { deleteWork, getWork, releasesForWork } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { Button } from '@/components/ui/Button'
import { SkeletonCard } from '@/components/ui/Skeleton'
import { CardHeader } from '@/components/card/CardHeader'
import { OverviewTab } from '@/components/card/OverviewTab'
import { DEFAULT_TAB, isTab, type Tab } from '@/components/card/tabs'
import { VersionPanel } from '@/components/VersionPanel'
import { ScorePanel } from '@/components/ScorePanel'
import { ReleasePanel } from '@/components/ReleasePanel'
import { AssistantPanel } from '@/components/AssistantPanel'
import { NotePanel } from '@/components/NotePanel'
import { WorkHistory } from '@/components/JournalFeed'
import { PluginBar } from '@/components/PluginBar'

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
  const work = useQuery({ queryKey: keys.work(workId), queryFn: () => getWork(workId) })

  // Only for the count on the tab; the tab itself fetches what it draws.
  const releases = useQuery({
    queryKey: keys.releasesForWork(workId),
    queryFn: () => releasesForWork(workId),
  })

  const remove = useMutation({
    mutationFn: () => deleteWork(workId),
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('toast.workDeleted', { title: work.data?.title ?? '' }),
        refresh: [keys.works, keys.workspace, keys.catalogue, keys.calendar],
        // The card was closed on the way out; an undo brings it back open.
        onUndone: () => onUndone(workId),
      })
      onDeleted()
    },
    onError: (cause) => say.failedTo(t('toast.workDeleteFailed'), cause),
  })

  if (work.isPending) return <SkeletonCard />

  if (work.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  // The row is gone — deleted in another view while this one held its id.
  if (work.data === null) {
    return <p className="text-sm text-dim">{t('error.notFound')}</p>
  }

  // A URL naming a tab that does not exist is corrected rather than shown empty,
  // and `replace` keeps the bad address out of the history.
  if (!isTab(tab)) {
    return <Navigate to={`/works/${workId}/${DEFAULT_TAB}`} replace />
  }

  const current = work.data

  // No gap on this column: the header's two halves have to meet, or the card is
  // cut in two by a stripe of background. The body gets its own margin instead.
  return (
    <div className="flex flex-col">
      <CardHeader work={current} releases={releases.data?.length ?? 0} />

      <div className="mt-4">
        <TabBody tab={tab} workId={workId} work={current} />
      </div>

      <footer className="mt-6 flex justify-end border-t border-line pt-4">
        <Button variant="danger" disabled={remove.isPending} onClick={() => remove.mutate()}>
          <Trash2 aria-hidden className="size-4" />
          {t('work.delete')}
        </Button>
      </footer>
    </div>
  )
}

/** The one tab that is open. Everything else is not mounted at all. */
function TabBody({
  tab,
  workId,
  work,
}: {
  tab: Tab
  workId: string
  work: Parameters<typeof CardHeader>[0]['work']
}) {
  switch (tab) {
    // Plugins write into the work's own `meta` and may rewrite its versions, so
    // they belong beside the fields they change rather than beside its releases.
    case 'overview':
      return (
        <div className="flex flex-col gap-4">
          <OverviewTab work={work} />
          <PluginBar target="work" id={workId} />
        </div>
      )
    case 'lyrics':
      return <VersionPanel workId={workId} />
    case 'score':
      return <ScorePanel workId={workId} />
    case 'releases':
      return <ReleasePanel workId={workId} workTitle={work.title} />
    case 'notes':
      return <NotePanel workId={workId} />
    // The mockup has no assistant tab — it puts the panel in a drawer with a
    // floating button, which is v0.26. Until then it lives here rather than
    // being unreachable.
    case 'assistant':
      return <AssistantPanel workId={workId} />
    case 'history':
      return <WorkHistory workId={workId} />
  }
}
