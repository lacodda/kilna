import { useCallback, useEffect, useState } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { getWorkspace, type Workspace } from '@/lib/api'
import { ProfileContext } from '@/lib/useProfile'
import { Sidebar } from '@/components/shell/Sidebar'
import { Topbar } from '@/components/shell/Topbar'
import { WorkList } from '@/components/WorkList'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { CalendarView } from '@/components/CalendarView'
import { DataView } from '@/components/DataView'
import { Styleguide } from '@/components/Styleguide'

interface ScreenProps {
  revision: number
  refresh: () => void
}

// The works screen carries the selection in its URL: /works is the list with
// nothing open, /works/:workId opens that work. The back button just works.
function WorksScreen({ revision, refresh }: ScreenProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { workId } = useParams()

  return (
    <div className="grid h-full gap-6 overflow-hidden p-6 lg:grid-cols-[20rem_1fr]">
      <WorkList
        selectedId={workId ?? null}
        onSelect={(id) => navigate(`/works/${id}`)}
        revision={revision}
        onChanged={refresh}
      />

      <main className="overflow-y-auto">
        {workId === undefined ? (
          <div className="flex h-full items-center justify-center rounded-2xl border border-dashed border-line-2 p-8 text-center">
            <div>
              <p className="font-medium">{t('empty.title')}</p>
              <p className="mt-1 text-sm text-dim">{t('empty.body')}</p>
            </div>
          </div>
        ) : (
          <WorkCard
            key={workId}
            workId={workId}
            onChanged={refresh}
            onDeleted={() => {
              navigate('/works')
              refresh()
            }}
          />
        )}
      </main>
    </div>
  )
}

export default function App() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const location = useLocation()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [error, setError] = useState<string | null>(null)
  // Bumped whenever a work changes, so the list and the counters refetch.
  const [revision, setRevision] = useState(0)

  const refresh = useCallback(() => setRevision((current) => current + 1), [])

  useEffect(() => {
    getWorkspace()
      .then((loaded) => {
        setWorkspace(loaded)
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [revision])

  if (error !== null) {
    return (
      <p role="alert" className="p-8 text-sm text-bad">
        {t('status.failed', { message: error })}
      </p>
    )
  }

  if (workspace === null) {
    return <p className="p-8 text-sm text-dim">{t('status.loading')}</p>
  }

  if (workspace.profile === null) {
    return (
      <p role="alert" className="p-8 text-sm text-bad">
        {t('status.noProfile')}
      </p>
    )
  }

  const openWork = (workId: string) => navigate(`/works/${workId}`)

  return (
    <ProfileContext value={workspace.profile}>
      <div className="grid h-full grid-cols-[216px_1fr] grid-rows-[52px_1fr] [grid-template-areas:'side_top'_'side_main']">
        <div className="[grid-area:side]">
          <Sidebar
            profileId={workspace.profile.id}
            onProfileSwitched={() => {
              // The selection belongs to the profile being left.
              navigate('/works')
              refresh()
            }}
          />
        </div>
        <div className="[grid-area:top]">
          <Topbar works={workspace.works} />
        </div>

        {/* Keyed by the screen so the entry animation replays on navigation,
            but not when moving between works inside the same screen. */}
        <div
          key={location.pathname.split('/')[1]}
          className="screen-in overflow-y-auto [grid-area:main]"
        >
          <Routes>
            <Route path="/" element={<Navigate to="/works" replace />} />
            <Route path="/works/:workId?" element={<WorksScreen revision={revision} refresh={refresh} />} />
            <Route
              path="/catalogue"
              element={
                <div className="p-6">
                  <Catalogue revision={revision} onSelect={openWork} />
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
              path="/settings"
              element={
                <div className="p-6">
                  <DataView onChanged={refresh} />
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
        </div>
      </div>
    </ProfileContext>
  )
}
