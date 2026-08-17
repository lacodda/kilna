import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { getWorkspace } from '@/lib/api'
import { humanError } from '@/lib/errors'
import { keys } from '@/lib/query'
import { ProfileContext } from '@/lib/useProfile'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { Sidebar } from '@/components/shell/Sidebar'
import { Topbar } from '@/components/shell/Topbar'
import { WorkList } from '@/components/WorkList'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { CalendarView } from '@/components/CalendarView'
import { DataView } from '@/components/DataView'
import { JournalView } from '@/components/JournalView'
import { TrashView } from '@/components/TrashView'
import { Styleguide } from '@/components/Styleguide'
import { EmptyState } from '@/components/ui/EmptyState'
import { Panel } from '@/components/ui/Panel'
import { Skeleton } from '@/components/ui/Skeleton'

// The works screen carries the selection in its URL: /works is the list with
// nothing open, /works/:workId opens that work. The back button just works.
function WorksScreen() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { workId, tab } = useParams()

  return (
    <div className="grid h-full gap-6 overflow-hidden p-6 lg:grid-cols-[20rem_1fr]">
      <WorkList selectedId={workId ?? null} onSelect={(id) => navigate(`/works/${id}`)} />

      <main className="overflow-y-auto">
        {workId === undefined ? (
          <EmptyState title={t('empty.title')} body={t('empty.body')} />
        ) : (
          <WorkCard
            key={workId}
            workId={workId}
            tab={tab}
            onDeleted={() => navigate('/works')}
            onUndone={(restored) => navigate(`/works/${restored}`)}
          />
        )}
      </main>
    </div>
  )
}

// The shell's own loading state: the frame is already drawn, so this only has
// to stand in for the sidebar and topbar until the workspace answers.
function ShellSkeleton() {
  return (
    <div className="grid h-full grid-cols-[216px_1fr] grid-rows-[52px_1fr] [grid-template-areas:'side_top'_'side_main']">
      <div className="border-r border-line [grid-area:side]" />
      <div className="border-b border-line [grid-area:top]" />
      <div className="flex flex-col gap-3 p-6 [grid-area:main]">
        <Skeleton className="h-9 w-64" />
        <Skeleton className="h-40 w-full" />
      </div>
    </div>
  )
}

export default function App() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const location = useLocation()

  const {
    data: workspace,
    error,
    isPending,
  } = useQuery({ queryKey: keys.workspace, queryFn: getWorkspace })

  if (isPending) return <ShellSkeleton />

  if (error !== null) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <Panel className="max-w-lg p-6">
          <p role="alert" className="text-sm text-bad">
            {t('status.failed', { message: humanError(error) })}
          </p>
        </Panel>
      </div>
    )
  }

  if (workspace.profile === null) {
    return (
      <p role="alert" className="p-8 text-sm text-bad">
        {t('status.noProfile')}
      </p>
    )
  }

  const openWork = (workId: string) => navigate(`/works/${workId}`)
  const screen = location.pathname.split('/')[1] ?? ''

  return (
    <ProfileContext value={workspace.profile}>
      <div className="grid h-full grid-cols-[216px_1fr] grid-rows-[52px_1fr] [grid-template-areas:'side_top'_'side_main']">
        <div className="[grid-area:side]">
          <Sidebar
            profileId={workspace.profile.id}
            // The selection belongs to the profile being left.
            onProfileSwitched={() => navigate('/works')}
          />
        </div>
        <div className="[grid-area:top]">
          <Topbar works={workspace.works} />
        </div>

        {/* Keyed by the screen so the entry animation replays on navigation,
            but not when moving between works inside the same screen. */}
        <div key={screen} className="screen-in overflow-y-auto [grid-area:main]">
          {/* Resetting on the screen name means a crash does not outlive the
              route that caused it. */}
          <ErrorBoundary resetKey={screen}>
            <Routes>
              <Route path="/" element={<Navigate to="/works" replace />} />
              {/* The open tab is part of the address, so the back button walks
                  between tabs and a tab can be linked to directly. */}
              <Route path="/works/:workId?/:tab?" element={<WorksScreen />} />
              <Route
                path="/catalogue"
                element={
                  <div className="p-6">
                    <Catalogue onSelect={openWork} />
                  </div>
                }
              />
              <Route
                path="/calendar"
                element={
                  <div className="p-6">
                    <CalendarView onSelect={openWork} />
                  </div>
                }
              />
              <Route
                path="/journal"
                element={
                  <div className="p-6">
                    <JournalView />
                  </div>
                }
              />
              <Route
                path="/trash"
                element={
                  <div className="p-6">
                    <TrashView />
                  </div>
                }
              />
              <Route
                path="/settings"
                element={
                  <div className="p-6">
                    <DataView />
                  </div>
                }
              />
              {import.meta.env.DEV && (
                <Route
                  path="/styleguide"
                  element={
                    <div className="p-6">
                      <Styleguide />
                    </div>
                  }
                />
              )}
              <Route path="*" element={<Navigate to="/works" replace />} />
            </Routes>
          </ErrorBoundary>
        </div>
      </div>
    </ProfileContext>
  )
}
