import { useEffect } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getWorkspace, warnUnreadyReleases } from '@/lib/api'
import { humanError } from '@/lib/errors'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { useKeys } from '@/lib/useKeys'
import { RAIL_WIDTH, useRail } from '@/lib/rail'
import { ProfileContext } from '@/lib/useProfile'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { AssistantLauncher } from '@/components/assistant/AssistantDrawer'
import { QueueBanner } from '@/components/assistant/QueueBanner'
import { WaitingBanner } from '@/components/assistant/WaitingBanner'
import { KeyboardSheet } from '@/components/shell/KeyboardSheet'
import { ResizeEdges } from '@/components/ui/window-frame'
import { Sidebar } from '@/components/shell/Sidebar'
import { Splash } from '@/components/shell/Splash'
import { Titlebar } from '@/components/shell/Titlebar'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { DashboardView } from '@/components/DashboardView'
import { CalendarView } from '@/components/CalendarView'
import { SettingsView } from '@/components/settings/SettingsView'
import { JournalView } from '@/components/JournalView'
import { StylesView } from '@/components/StylesView'
import { NotesView } from '@/components/notes/NotesView'
import { TrashView } from '@/components/TrashView'
import { Styleguide } from '@/components/Styleguide'
import { Panel } from '@/components/ui/panel'
import { AppShell, Screen } from '@/components/ui/app-shell'

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
    // Held, not flowing: the card lays itself out against the window's height
    // - its header stands still and the open tab takes what is left, scrolling
    // inside itself - so the screen around it must not scroll at all.
    <Screen scroll="held">
      <WorkCard
        key={workId}
        workId={workId}
        tab={tab}
        onDeleted={() => navigate('/catalogue')}
        onUndone={(restored) => navigate(`/works/${restored}`)}
      />
    </Screen>
  )
}

  export default function App() {
    const { t } = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    // Before the early returns below, because a hook cannot be conditional -
    // and because the shortcuts should answer while the workspace is still
    // loading rather than arriving a moment after the shell does.
    const { helpOpen, setHelpOpen } = useKeys()
    const { rail, toggle: toggleRail } = useRail()

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

    if (isPending) return <Splash />

    // The static splash from index.html is taken down by <Splash /> on the
    // way through; when the workspace answers before that ever rendered, it
    // is taken down here instead.
    document.getElementById('splash')?.remove()

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
          {/* The frame is dowel's: the title bar runs the width of the
              window, as a system one would, the rail starts under it, and the
              screen takes the corner they leave. Nothing in it scrolls but
              what is inside a `<Screen>`.

              The rail folds to icons from the handle in the title bar. Its
              width is the grid's column rather than the nav's own, so the
              screen beside it grows as it folds instead of leaving a gap; the
              160ms is the only motion, and none under reduced motion. */}
          <AppShell
            sideWidth={RAIL_WIDTH[rail]}
            className="transition-[grid-template-columns] duration-160 ease-out motion-reduce:transition-none"
            top={
              <Titlebar
                works={workspace.works}
                compact={rail === 'compact'}
                onToggleRail={toggleRail}
              />
            }
            side={
              <Sidebar
                profileId={workspace.profile.id}
                // The open work belongs to the profile being left.
                onProfileSwitched={() => navigate('/catalogue')}
                compact={rail === 'compact'}
              />
            }
          >
            {/* The id is for the text on stage: a version given the whole
                content area renders into this box through a portal, rail and
                title bar left in place. */}
            <div id="main-area" className="relative flex min-h-0 min-w-0 flex-1 flex-col">
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
                  screen.

                  This box does not scroll: it hands its height down, and each
                  `<Screen>` decides where the scrolling happens inside it. The
                  window having no scrollbar of its own is the point — see
                  `Screen`. */}
              <div key={screen} className="screen-in flex min-h-0 flex-1 flex-col overflow-hidden">
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
                        <Screen>
                          <DashboardView onSelect={openWork} />
                        </Screen>
                      }
                    />
                    {/* The open tab is part of the address, so the back button walks
                        between tabs and a tab can be linked to directly. */}
                    <Route path="/works/:workId?/:tab?" element={<WorksScreen />} />
                    {/* The catalogue holds its own height rather than growing
                        with its rows: its table scrolls both ways inside, so the
                        sideways bar stays at the bottom of the window. */}
                    <Route
                      path="/catalogue"
                      element={
                        <Screen scroll="held">
                          <Catalogue onSelect={openWork} />
                        </Screen>
                      }
                    />
                    {/* Like the catalogue: the screen holds the window's
                        height rather than growing with its list, so the queue
                        scrolls inside its own column and the month it is being
                        read against stays on screen. */}
                    <Route
                      path="/calendar"
                      element={
                        <Screen scroll="held">
                          <CalendarView onSelect={openWork} />
                        </Screen>
                      }
                    />
                    {/* Held like the catalogue: the list and the open note
                        each scroll inside their own column, and the note's
                        tags stay on the window's bottom edge. The open note
                        is in the address, so back walks between notes. */}
                    <Route
                      path="/notes/:noteId?"
                      element={
                        <Screen scroll="held">
                          <NotesView />
                        </Screen>
                      }
                    />
                    <Route
                      path="/styles"
                      element={
                        <Screen>
                          <StylesView />
                        </Screen>
                      }
                    />
                    <Route
                      path="/journal"
                      element={
                        <Screen>
                          <JournalView />
                        </Screen>
                      }
                    />
                    <Route
                      path="/trash"
                      element={
                        <Screen>
                          <TrashView />
                        </Screen>
                      }
                    />
                    {/* The section is part of the address, like a card's tab:
                        the rail's Settings link lands on the first one. */}
                    <Route
                      path="/settings/:section?"
                      element={
                        <Screen scroll="held">
                          <SettingsView />
                        </Screen>
                      }
                    />
                    {import.meta.env.DEV && (
                      <Route
                        path="/styleguide"
                        element={
                          <Screen>
                            <Styleguide />
                          </Screen>
                        }
                      />
                    )}
                    <Route path="*" element={<Navigate to="/dashboard" replace />} />
                  </Routes>
                </ErrorBoundary>
              </div>
            </div>
          </AppShell>
      </AssistantLauncher>

      <KeyboardSheet open={helpOpen} onOpenChange={setHelpOpen} />
      <ResizeEdges />
    </ProfileContext>
  )
}
