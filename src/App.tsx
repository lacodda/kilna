import { useEffect } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getWorkspace, warnUnreadyReleases } from '@/lib/api'
import { humanError } from '@/lib/errors'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { useKeys } from '@/lib/useKeys'
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
import { TrashView } from '@/components/TrashView'
import { Styleguide } from '@/components/Styleguide'
import { Panel } from '@/components/ui/panel'

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
    // No scrolling of its own: the screen area outside already scrolls, and a
    // second scroller here drew a second scrollbar over the card's tab strip -
    // the one the pilot saw. It also gave `sticky top-0` in the header the
    // wrong box to stick to.
    //
    // No padding at the top either: the header sticks to the top of the
    // scrolling area, and 24px above it is 24px the scrolled text shows
    // through. The card's own cover provides the space instead.
    <main className="px-6 pb-6">
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

  export default function App() {
    const { t } = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    // Before the early returns below, because a hook cannot be conditional -
    // and because the shortcuts should answer while the workspace is still
    // loading rather than arriving a moment after the shell does.
    const { helpOpen, setHelpOpen } = useKeys()

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
          {/* The title bar runs the width of the window, as a system one
              would: the mark sits where the system prints the name, and the
              window buttons sit at its right end. The rail starts under it. */}
          <div className="grid h-full grid-cols-[216px_1fr] grid-rows-[40px_1fr] [grid-template-areas:'top_top'_'side_main']">
          {/* `min-h-0` for the same reason the main column has it: the rail
              is a grid item, and without it the nav measured its content
              rather than the track, so its border and footer stopped
              halfway down a tall window. */}
          <div className="min-h-0 [grid-area:side]">
            <Sidebar
              profileId={workspace.profile.id}
              // The open work belongs to the profile being left.
              onProfileSwitched={() => navigate('/catalogue')}
            />
          </div>
          <div className="min-w-0 [grid-area:top]">
            <Titlebar works={workspace.works} />
          </div>

          {/* `min-w-0` beside `min-h-0`, and for the same reason on the other
              axis: a grid item defaults to `min-width: auto`, so this column
              grew to fit its widest child instead of staying inside the track.
              A wide table pushed the whole screen out from under the sidebar,
              and the clip below had nothing left to scroll. */}
          {/* `relative` and the id are for the text on stage: a version
              given the whole content area renders into this box through a
              portal, sidebar and topbar left in place. */}
          <div id="main-area" className="relative flex min-h-0 min-w-0 flex-col [grid-area:main]">
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

                `overflow-y-auto` alone leaves the horizontal axis at `auto`
                too, so this box was quietly scrollable sideways and a trackpad
                swipe slid the content under the sidebar — the same trap the tab
                strip hit at v0.35. The axis is clipped explicitly.

                `scrollbar-gutter: stable` keeps the scrollbar’s 10px reserved
                whether or not a screen is long enough to need one. Without it
                every navigation was a small sideways jump: the skeleton is
                short, and the content replacing it is not. */}
            <div
              key={screen}
              className="screen-in min-h-0 flex-1 overflow-y-auto overflow-x-hidden [scrollbar-gutter:stable]"
            >
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
                      <div className="px-6 pt-3 pb-6">
                        <DashboardView onSelect={openWork} />
                      </div>
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
                      <div className="flex h-full min-h-0 flex-col px-6 pt-3 pb-6">
                        <Catalogue onSelect={openWork} />
                      </div>
                    }
                  />
                  {/* Like the catalogue: the screen holds the window's
                      height rather than growing with its list, so the queue
                      scrolls inside its own column and the month it is being
                      read against stays on screen. */}
                  <Route
                    path="/calendar"
                    element={
                      <div className="flex h-full min-h-0 flex-col px-6 pt-3 pb-6">
                        <CalendarView onSelect={openWork} />
                      </div>
                    }
                  />
                  <Route
                    path="/journal"
                    element={
                      <div className="px-6 pt-3 pb-6">
                        <JournalView />
                      </div>
                    }
                  />
                  <Route
                    path="/trash"
                    element={
                      <div className="px-6 pt-3 pb-6">
                        <TrashView />
                      </div>
                    }
                  />
                  {/* The section is part of the address, like a card's tab:
                      the rail's Settings link lands on the first one. */}
                  <Route
                    path="/settings/:section?"
                    element={
                      <div className="px-6 pt-3 pb-6">
                        <SettingsView />
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

      <KeyboardSheet open={helpOpen} onOpenChange={setHelpOpen} />
      <ResizeEdges />
    </ProfileContext>
  )
}
