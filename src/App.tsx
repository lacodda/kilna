import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Monitor, Moon, Sun } from 'lucide-react'
import { getWorkspace, type Workspace } from '@/lib/api'
import { ProfileContext } from '@/lib/useProfile'
import { nextTheme, useTheme, type Theme } from '@/lib/theme'
import { LoopBar } from '@/components/LoopBar'
import { WorkList } from '@/components/WorkList'
import { WorkCard } from '@/components/WorkCard'
import { Catalogue } from '@/components/Catalogue'
import { CalendarView } from '@/components/CalendarView'
import { DataView } from '@/components/DataView'
import { ProfileSwitcher } from '@/components/ProfileSwitcher'
import { Styleguide } from '@/components/Styleguide'
import { Button } from '@/components/ui/Button'

// The styleguide is the development inventory of the design system; it is not
// part of the released surface.
const VIEWS = import.meta.env.DEV
  ? (['works', 'catalogue', 'calendar', 'data', 'styleguide'] as const)
  : (['works', 'catalogue', 'calendar', 'data'] as const)

type View = (typeof VIEWS)[number]

const THEME_ICONS: Record<Theme, typeof Monitor> = {
  system: Monitor,
  light: Sun,
  dark: Moon,
}

export default function App() {
  const { t } = useTranslation()
  const { theme, setTheme } = useTheme()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [view, setView] = useState<View>('works')
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

  const ThemeIcon = THEME_ICONS[theme]

  return (
    <ProfileContext value={workspace.profile}>
      <div className="flex h-full flex-col">
        <header className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-line px-6 py-3">
          <h1 className="text-lg font-semibold tracking-tight">{t('app.name')}</h1>
          <LoopBar />
          <div className="ml-auto flex items-center gap-3">
            <p className="text-xs text-faint">
              {t('status.works')} {workspace.works}
            </p>
            <Button
              variant="icon"
              size="iconMd"
              title={t(`theme.${theme}`)}
              onClick={() => setTheme(nextTheme(theme))}
            >
              <ThemeIcon aria-hidden />
            </Button>
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

        <nav className="flex gap-1 border-b border-line px-6">
          {VIEWS.map((tab) => (
            <button
              key={tab}
              type="button"
              onClick={() => setView(tab)}
              className={
                view === tab
                  ? 'cursor-pointer border-b-2 border-accent px-3 py-2 text-sm font-medium'
                  : 'cursor-pointer border-b-2 border-transparent px-3 py-2 text-sm text-dim hover:text-text'
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
        ) : view === 'styleguide' ? (
          <main className="flex-1 overflow-y-auto p-6">
            <Styleguide />
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
                <div className="flex h-full items-center justify-center rounded-2xl border border-dashed border-line-2 p-8 text-center">
                  <div>
                    <p className="font-medium">{t('empty.title')}</p>
                    <p className="mt-1 text-sm text-dim">{t('empty.body')}</p>
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
