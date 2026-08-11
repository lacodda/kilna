import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  backupWorkspace,
  exportMarkdown,
  importLegacy,
  suggestedBackupName,
  workspacePath,
} from '@/lib/api'
import { Button } from '@/components/ui/Button'
import { ProfileEditor } from '@/components/ProfileEditor'

interface Props {
  onChanged: () => void
}

// Getting data out and in. The export is the "you are not locked in" promise
// made checkable; the backup is the whole workspace in one file.
export function DataView({ onChanged }: Props) {
  const { t } = useTranslation()
  const [path, setPath] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    workspacePath()
      .then(setPath)
      .catch((cause: unknown) => setError(String(cause)))
  }, [])

  const run = (task: () => Promise<string>) => {
    setBusy(true)
    setError(null)
    setNotice(null)
    task()
      .then((message) => {
        if (message !== '') setNotice(message)
      })
      .catch((cause: unknown) => setError(String(cause)))
      .finally(() => setBusy(false))
  }

  const doExport = () =>
    run(async () => {
      const directory = await open({ directory: true, title: t('data.exportTitle') })
      if (typeof directory !== 'string') return ''

      const report = await exportMarkdown(directory)
      return t('data.exported', { files: report.files, works: report.works })
    })

  const doBackup = () =>
    run(async () => {
      const suggested = await suggestedBackupName()
      const destination = await save({ defaultPath: suggested, title: t('data.backupTitle') })
      if (destination === null) return ''

      const written = await backupWorkspace(destination)
      return t('data.backedUp', { path: written })
    })

  const doImport = () =>
    run(async () => {
      const source = await open({
        title: t('data.importTitle'),
        filters: [{ name: 'SQLite', extensions: ['db', 'sqlite'] }],
      })
      if (typeof source !== 'string') return ''

      const report = await importLegacy(source)
      onChanged()
      return t('data.imported', {
        works: report.works,
        versions: report.versions,
        scores: report.scores,
        skipped: report.skipped,
      })
    })

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <ProfileEditor onSaved={onChanged} />

      <hr className="border-neutral-200 dark:border-neutral-800" />

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.export')}</h3>
        <p className="text-sm text-neutral-500">{t('data.exportHint')}</p>
        <div>
          <Button variant="primary" disabled={busy} onClick={doExport}>
            {t('data.exportAction')}
          </Button>
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.backup')}</h3>
        <p className="text-sm text-neutral-500">{t('data.backupHint')}</p>
        <div>
          <Button disabled={busy} onClick={doBackup}>
            {t('data.backupAction')}
          </Button>
        </div>
        {path !== null && (
          <p className="text-xs text-neutral-500">
            {t('data.workspaceAt')} <code className="font-mono">{path}</code>
          </p>
        )}
        <p className="text-xs text-neutral-500">{t('data.restoreHint')}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.import')}</h3>
        <p className="text-sm text-neutral-500">{t('data.importHint')}</p>
        <div>
          <Button disabled={busy} onClick={doImport}>
            {t('data.importAction')}
          </Button>
        </div>
      </section>

      {busy && <p className="text-sm text-neutral-500">{t('data.working')}</p>}
      {notice !== null && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-sm text-emerald-900 dark:bg-emerald-950/50 dark:text-emerald-200">
          {notice}
        </p>
      )}
      {error !== null && (
        <p role="alert" className="text-sm text-red-600">
          {error}
        </p>
      )}
    </div>
  )
}
