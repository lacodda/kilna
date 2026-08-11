import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getWorkspace, type Workspace } from '@/lib/api'
import { ProfileContext } from '@/lib/useProfile'
import { LoopBar } from '@/components/LoopBar'
import { WorkList } from '@/components/WorkList'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { CalendarView } from '@/components/CalendarView'
import { DataView } from '@/components/DataView'
import { ProfileSwitcher } from '@/components/ProfileSwitcher'

export default function App() {
  const { t } = useTranslation()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [view, setView] = useState<'works' | 'catalogue' | 'calendar' | 'data'>('works')
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
      <p role="alert" className="p-8 text-sm text-red-600">
        {t('status.failed', { message: error })}
      </p>
    )
  }

  if (workspace === null) {
    return <p className="p-8 text-sm text-neutral-500">{t('status.loading')}</p>
  }

  if (workspace.profile === null) {
    return (
      <p role="alert" className="p-8 text-sm text-red-600">
        {t('status.noProfile')}
      </p>
    )
  }

  return (
    <ProfileContext value={workspace.profile}>
      <div className="flex h-full flex-col">
        <header className="flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-neutral-200 px-6 py-3 dark:border-neutral-800">
          <h1 className="text-lg font-semibold tracking-tight">{t('app.name')}</h1>
          <LoopBar />
          <div className="ml-auto flex items-center gap-3">
            <p className="text-xs text-neutral-500">
              {t('status.works')} {workspace.works}
            </p>
            <ProfileSwitcher
              activeId={workspace.profile.id}
              onSwitched={() => {
                // The selection belongs to the profile being left.
                setSelectedId(null)
                refresh()
              }}
            />
          </div>
        </header>

        <nav className="flex gap-1 border-b border-neutral-200 px-6 dark:border-neutral-800">
          {(['works', 'catalogue', 'calendar', 'data'] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              onClick={() => setView(tab)}
              className={
                view === tab
                  ? 'border-b-2 border-kiln-500 px-3 py-2 text-sm font-medium'
                  : 'border-b-2 border-transparent px-3 py-2 text-sm text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200'
              }
            >
              {t(`nav.${tab}`)}
            </button>
          ))}
        </nav>

        {view === 'catalogue' ? (
          <main className="flex-1 overflow-y-auto p-6">
            <Catalogue
              revision={revision}
              onSelect={(workId) => {
                setSelectedId(workId)
                setView('works')
              }}
            />
          </main>
        ) : view === 'data' ? (
          <main className="flex-1 overflow-y-auto p-6">
            <DataView onChanged={refresh} />
          </main>
        ) : view === 'calendar' ? (
          <main className="flex-1 overflow-y-auto p-6">
            <CalendarView
              onSelect={(workId) => {
                setSelectedId(workId)
                setView('works')
              }}
            />
          </main>
        ) : (
          <div className="grid flex-1 gap-6 overflow-hidden p-6 lg:grid-cols-[20rem_1fr]">
            <WorkList
              selectedId={selectedId}
              onSelect={setSelectedId}
              revision={revision}
              onChanged={refresh}
            />

            <main className="overflow-y-auto">
              {selectedId === null ? (
                <div className="flex h-full items-center justify-center rounded-lg border border-dashed border-neutral-300 p-8 text-center dark:border-neutral-700">
                  <div>
                    <p className="font-medium">{t('empty.title')}</p>
                    <p className="mt-1 text-sm text-neutral-500">{t('empty.body')}</p>
                  </div>
                </div>
              ) : (
                <WorkCard
                  key={selectedId}
                  workId={selectedId}
                  onChanged={refresh}
                  onDeleted={() => {
                    setSelectedId(null)
                    refresh()
                  }}
                />
              )}
            </main>
          </div>
        )}
      </div>
    </ProfileContext>
  )
}
