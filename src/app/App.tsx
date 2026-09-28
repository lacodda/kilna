import { useEffect } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { warnUnreadyReleases } from '@/lib/api/releases'
import { humanError } from '@/lib/errors'
import { say } from '@/lib/toast'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useKeys } from '@/app/useKeys'
import { RAIL_WIDTH, useRail } from '@/lib/rail'
import { ProfileContext } from '@/lib/useProfile'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { AssistantLauncher } from '@/features/assistant/AssistantDrawer'
import { QueueBanner } from '@/features/assistant/QueueBanner'
import { WaitingBanner } from '@/features/assistant/WaitingBanner'
import { KeyboardSheet } from '@/shell/KeyboardSheet'
import { ResizeEdges } from '@/components/ui/window-frame'
import { Sidebar } from '@/shell/Sidebar'
import { Splash } from '@/shell/Splash'
import { Titlebar } from '@/shell/Titlebar'
import { Panel } from '@/components/ui/panel'
import { AppShell, Screen } from '@/components/ui/app-shell'
import { drawn } from '@/app/screens'

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
  const { data: workspace, error, isPending } = useQuery(queries.workspace())

  // The startup sweep: warn about every release due inside the week that is
  // not ready. There is no scheduler in this app, so "at startup, per profile"
  // is when standing gaps get noticed; each is written once, so a sweep that
  // finds nothing new changes nothing. The local date goes with the call — the
  // backend only knows UTC, which after sunset here is already tomorrow.
  const profileId = workspace?.profile?.id
  useEffect(() => {
    if (profileId === undefined) return
    // A sweep that fails says so: the week's gaps would otherwise go
    // unnoticed until the day, which is the one thing the sweep is for.
    warnUnreadyReleases(today())
      .then((standing) => {
        if (standing > 0) void client.invalidateQueries({ queryKey: keys.journal })
      })
      .catch((cause: unknown) => say.failedTo(t('toast.sweepFailed'), cause))
  }, [profileId, client, t])

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
  // Where a crash is reset: the screen, and for a work the work itself - a
  // card that fell over stayed fallen when another work was opened, because
  // every work is the same screen.
  const place = location.pathname.split('/').slice(0, 3).join('/')

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
            <div className="flex flex-col gap-2 px-4 pt-3 empty:hidden">
              <WaitingBanner />
              <QueueBanner />
            </div>

            {/* Keyed by the screen so the entry animation replays on
                navigation, but not when moving between works inside the same
                screen.

                This box does not scroll, and neither does any screen: each
                is held against the height it is handed and lays itself out
                on `components/frame` - a head that stands and a part that
                scrolls. The window having no scrollbar of its own is the
                point. `data-screen-area` is how the tests find it to hold
                every screen to that (`app/smoke.test.tsx`). */}
            <div
              key={screen}
              data-screen-area
              className="screen-in flex min-h-0 flex-1 flex-col overflow-hidden"
            >
              {/* Resetting on the place means a crash does not outlive the
                  route, or the work, that caused it. */}
              <ErrorBoundary resetKey={place}>
                <Routes>
                  <Route path="/" element={<Navigate to="/dashboard" replace />} />
                  {/* Every screen from the one list the rail and the title
                      bar read too (`app/screens.tsx`). The margins are the
                      mockup's: sixteen at the sides, the head twelve under
                      the title bar, fourteen above the window's edge. */}
                  {drawn(import.meta.env.DEV).map((spec) => (
                    <Route
                      key={spec.key}
                      path={spec.path}
                      element={
                        <Screen scroll="held" pad="none" className="px-4 pt-3 pb-3.5">
                          {spec.render(openWork)}
                        </Screen>
                      }
                    />
                  ))}
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
