import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Trash2 } from 'lucide-react'
import { deleteWork, getWork, listLinks, listScenes, releasesForWork } from '@/lib/api'
import { keys } from '@/lib/query'
import { noteDeleted, noteOpened } from '@/lib/recent'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { hasScenes, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { SkeletonCard } from '@/components/ui/Skeleton'
import { CardHeader } from '@/components/card/CardHeader'
import { FilesTab } from '@/components/card/FilesTab'
import { LinksTab } from '@/components/card/LinksTab'
import { OverviewTab } from '@/components/card/OverviewTab'
import { ScenesTab } from '@/components/card/ScenesTab'
import { DEFAULT_TAB, isTab, type Tab } from '@/components/card/tabs'
import { VersionPanel } from '@/components/VersionPanel'
import { ScorePanel } from '@/components/ScorePanel'
import { ReleasePanel } from '@/components/ReleasePanel'
import { ActionBar } from '@/components/assistant/ActionBar'
import { AssistantPanel } from '@/components/assistant/AssistantPanel'
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
  const profile = useProfile()
  const work = useQuery({ queryKey: keys.work(workId), queryFn: () => getWork(workId) })

  // Noted once the title is known, since the list shows names rather than ids.
  // Keyed on both, so a rename while the card is open updates the entry rather
  // than leaving the old name to be offered tomorrow.
  const title = work.data?.title
  useEffect(() => {
    if (title !== undefined) noteOpened({ id: workId, title })
  }, [workId, title])

  // Only for the count on the tab; the tab itself fetches what it draws.
  const releases = useQuery({
    queryKey: keys.releasesForWork(workId),
    queryFn: () => releasesForWork(workId),
  })
  // For the count on the tab: "Links (2)" is how a song shows it has clips
  // without anyone opening the tab.
  const links = useQuery({
    queryKey: keys.linksFor(workId),
    queryFn: () => listLinks(workId),
  })
  // The storyboard is a fact of the kind: a song has none, and asking for
  // its scenes would be a query for a tab that is not drawn.
  const storyboard = hasScenes(profile.config, work.data?.kind)
  const scenes = useQuery({
    queryKey: keys.scenesFor(workId),
    queryFn: () => listScenes(workId),
    enabled: storyboard,
  })

  const remove = useMutation({
    mutationFn: () => deleteWork(workId),
    onSuccess: (deletionId) => {
      // Out of the recent list too, so it cannot be offered after it is gone.
      // An undo re-opens the card, which puts it back.
      noteDeleted(workId)
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
      <CardHeader
        work={current}
        releases={releases.data?.length ?? 0}
        links={(links.data?.sources.length ?? 0) + (links.data?.derived.length ?? 0)}
        scenes={storyboard ? (scenes.data?.length ?? 0) : undefined}
      />

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
          {/* Profile actions and plugin commands are the same gesture from the
              user's side — do this to this work — so they sit together, below
              the fields both of them read. */}
          <ActionBar workId={workId} />
          <PluginBar target="work" id={workId} />
        </div>
      )
    case 'versions':
      return <VersionPanel workId={workId} />
    case 'scenes':
      return <ScenesTab work={work} />
    case 'score':
      return <ScorePanel workId={workId} />
    case 'releases':
      return <ReleasePanel workId={workId} workTitle={work.title} />
    case 'files':
      return <FilesTab work={work} />
    case 'links':
      return <LinksTab work={work} />
    case 'notes':
      return <NotePanel workId={workId} />
    // The mockup has no assistant tab — it puts the panel in a drawer with a
    // floating button, which is v0.28. Until then it lives here rather than
    // being unreachable.
    case 'assistant':
      return <AssistantPanel workId={workId} />
    case 'history':
      return <WorkHistory workId={workId} />
  }
}
