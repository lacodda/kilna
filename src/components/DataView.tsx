import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  backupWorkspace,
  exportMarkdown,
  importLegacy,
  suggestedBackupName,
  workspacePath,
} from '@/lib/api'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/Button'
import { ProfileEditor } from '@/components/ProfileEditor'
import { StatusDrift } from '@/components/StatusDrift'

// Getting data out and in. The export is the "you are not locked in" promise
// made checkable; the backup is the whole workspace in one file.
export function DataView() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)

  // The path never changes while the app runs, so it is asked for once.
  const path = useQuery({
    queryKey: ['workspacePath'],
    queryFn: workspacePath,
    staleTime: Infinity,
  })

  // Each of these opens an OS file dialog first, so they are not mutations in
  // the query sense — there is nothing to retry and no variables to carry.
  const run = (task: () => Promise<string>) => {
    setBusy(true)
    task()
      .then((message) => {
        // An empty message means the file dialog was dismissed: nothing
        // happened, so nothing is said.
        if (message !== '') say.ok(message)
      })
      .catch((cause: unknown) => say.failed(cause))
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
      // An import rewrites everything the app has read so far.
      void client.invalidateQueries()

      return t('data.imported', {
        works: report.works,
        versions: report.versions,
        scores: report.scores,
        skipped: report.skipped,
      })
    })

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <ProfileEditor />

      <hr className="border-line" />

      <StatusDrift />

      <hr className="border-line" />

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.export')}</h3>
        <p className="text-sm text-dim">{t('data.exportHint')}</p>
        <div>
          <Button variant="primary" disabled={busy} onClick={doExport}>
            {t('data.exportAction')}
          </Button>
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.backup')}</h3>
        <p className="text-sm text-dim">{t('data.backupHint')}</p>
        <div>
          <Button disabled={busy} onClick={doBackup}>
            {t('data.backupAction')}
          </Button>
        </div>
        {path.data != null && (
          <p className="text-xs text-dim">
            {t('data.workspaceAt')} <code className="font-mono">{path.data}</code>
          </p>
        )}
        <p className="text-xs text-dim">{t('data.restoreHint')}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.import')}</h3>
        <p className="text-sm text-dim">{t('data.importHint')}</p>
        <div>
          <Button disabled={busy} onClick={doImport}>
            {t('data.importAction')}
          </Button>
        </div>
      </section>

      {busy && <p className="text-sm text-dim">{t('data.working')}</p>}
    </div>
  )
}
