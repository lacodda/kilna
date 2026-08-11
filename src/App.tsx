import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getWorkspace, type Workspace } from '@/lib/api'
import { LoopBar } from '@/components/LoopBar'
import { StatusPanel } from '@/components/StatusPanel'

export default function App() {
  const { t } = useTranslation()
  const [workspace, setWorkspace] = useState<Workspace | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getWorkspace()
      .then(setWorkspace)
      .catch((cause: unknown) => setError(String(cause)))
  }, [])

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col gap-8 px-8 py-10">
      <header className="flex items-baseline gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">{t('app.name')}</h1>
        <p className="text-sm text-neutral-500">{t('app.tagline')}</p>
      </header>

      <LoopBar />

      {error !== null ? (
        <p role="alert" className="text-sm text-red-600">
          {t('status.failed', { message: error })}
        </p>
      ) : workspace === null ? (
        <p className="text-sm text-neutral-500">{t('status.loading')}</p>
      ) : (
        <StatusPanel workspace={workspace} />
      )}
    </div>
  )
}
