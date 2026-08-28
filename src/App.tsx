import { useEffect } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getWorkspace, warnUnreadyReleases } from '@/lib/api'
import { humanError } from '@/lib/errors'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { ProfileContext } from '@/lib/useProfile'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { AssistantLauncher } from '@/components/assistant/AssistantDrawer'
import { QueueBanner } from '@/components/assistant/QueueBanner'
import { WaitingBanner } from '@/components/assistant/WaitingBanner'
import { Sidebar } from '@/components/shell/Sidebar'
import { Topbar } from '@/components/shell/Topbar'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { DashboardView } from '@/components/DashboardView'
import { CalendarView } from '@/components/CalendarView'
import { DataView } from '@/components/DataView'
import { JournalView } from '@/components/JournalView'
import { TrashView } from '@/components/TrashView'
import { Styleguide } from '@/components/Styleguide'
import { Panel } from '@/components/ui/Panel'
import { Skeleton } from '@/components/ui/Skeleton'

// An open work, filling the screen. The address carries which one and which
// tab, so the back button walks between them.
//
// Until v0.21 a list of every work sat beside it here, duplicating the
// catalogue; `/works` with nothing open now sends you to the list that remains.
function WorksScreen() {
  const navigate = useNavigate()
  const { workId, tab } = useParams()

  if (workId === undefined) return <Navigate to="/catalogue" replace />

  return (
    <main className="h-full overflow-y-auto p-6">
      <WorkCard
        key={workId}
        workId={workId}
        tab={tab}
        onDeleted={() => navigate('/catalogue')}
        onUndone={(restored) => navigate(`/works/${restored}`)}
      />
    </main>
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

    const client = useQueryClient()
    const {
      data: workspace,
      error,
      isPending,
    } = useQuery({ queryKey: keys.workspace, queryFn: getWorkspace })

    // The startup sweep: warn about every release due inside the week that is
    // not ready. There is no scheduler in this app, so "at startup, per profile"
    // is when standing gaps get noticed; each is written once, so a sweep that
    // finds nothing new changes nothing. The local date goes with the call — the
    // backend only knows UTC, which after sunset here is already tomorrow.
    const profileId = workspace?.profile?.id
    useEffect(() => {
      if (profileId === undefined) return
      void warnUnreadyReleases(today()).then((standing) => {
        if (standing > 0) void client.invalidateQueries({ queryKey: keys.journal })
      })
    }, [profileId, client])

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

    const openWork = (workId: string, tab?: string) =>
      navigate(tab === undefined ? `/works/${workId}` : `/works/${workId}/${tab}`)
    const screen = location.pathname.split('/')[1] ?? ''

    return (
      <ProfileContext value={workspace.profile}>
        {/* The assistant from anywhere — a run belongs to its chat, and the chat
            should not require walking back to the card that started it. It wraps
            the shell rather than sitting beside it because the waiting banner
            inside asks it to open a chat. */}
        <AssistantLauncher>
          <div className="grid h-full grid-cols-[216px_1fr] grid-rows-[52px_1fr] [grid-template-areas:'side_top'_'side_main']">
          <div className="[grid-area:side]">
            <Sidebar
              profileId={workspace.profile.id}
              // The open work belongs to the profile being left.
              onProfileSwitched={() => navigate('/catalogue')}
            />
          </div>
          <div className="[grid-area:top]">
            <Topbar works={workspace.works} />
          </div>

          <div className="flex min-h-0 flex-col [grid-area:main]">
            {/* Above the scroll and outside the screen key: a pending question
                belongs to the workspace rather than to whichever screen is
                open, and it must not replay its entry animation on every
                navigation. */}
            <div className="flex flex-col gap-2 px-6 pt-4 empty:hidden">
              <WaitingBanner />
              <QueueBanner />
            </div>

            {/* Keyed by the screen so the entry animation replays on
                navigation, but not when moving between works inside the same
                screen. */}
            <div key={screen} className="screen-in min-h-0 flex-1 overflow-y-auto">
              {/* Resetting on the screen name means a crash does not outlive the
                  route that caused it. */}
              <ErrorBoundary resetKey={screen}>
                <Routes>
                  {/* The dashboard is where the app opens: the first question
                      is what needs deciding, not what exists. */}
                  <Route path="/" element={<Navigate to="/dashboard" replace />} />
                  <Route
                    path="/dashboard"
                    element={
                      <div className="p-6">
                        <DashboardView onSelect={openWork} />
                      </div>
                    }
                  />
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
                  <Route path="*" element={<Navigate to="/dashboard" replace />} />
                </Routes>
              </ErrorBoundary>
            </div>
          </div>
        </div>
      </AssistantLauncher>
    </ProfileContext>
  )
}
