import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import { backupWorkspace, exportMarkdown, importLegacy, suggestedBackupName } from '@/lib/api/data'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Skeleton } from '@/components/ui/skeleton'
import { MediaFolderField } from '@/features/settings/MediaFolderField'

// Getting data out and in. The export is the "you are not locked in" promise
// made checkable; the backup is the whole workspace in one file.
export function DataSection() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)

  // The path never changes while the app runs, so it is asked for once.
  const path = useQuery({ ...queries.workspacePath(), staleTime: Infinity })
  const log = useQuery({ ...queries.logPath(), staleTime: Infinity })

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

      const summary = t('data.imported', {
        works: report.works,
        versions: report.versions,
        scores: report.scores,
        skipped: report.skipped,
      })
      // The register and the bank are said only when there were any: an
      // older source has neither (ADR 0044, 0045).
      const material =
        report.terms + report.notes > 0
          ? ` ${t('data.importedMaterial', { terms: report.terms, named: report.named, notes: report.notes })}`
          : ''
      // Said only when it happened: most imports have nothing to leave buried.
      return report.deleted > 0
        ? `${summary}${material} ${t('data.importedLeftDeleted', { count: report.deleted })}`
        : `${summary}${material}`
    })

  return (
    // A field per way in or out, each explained under its button, as the
    // mockup's fields are - not a heading and a paragraph the button waits
    // below. `self-start` keeps a button its own width in the column.
    <div className="flex max-w-105 flex-col gap-4">
      <FieldGroup label={t('data.export')} help={t('data.exportHint')}>
        <Button variant="primary" className="self-start" disabled={busy} onClick={doExport}>
          {t('data.exportAction')}
        </Button>
      </FieldGroup>

      <FieldGroup label={t('data.backup')} help={t('data.backupHint')}>
        <Button className="self-start" disabled={busy} onClick={doBackup}>
          {t('data.backupAction')}
        </Button>
      </FieldGroup>

      {/* Where the workspace lives, with how a backup goes back in under
          it: the restore is done to this file, by hand, with kilna closed. */}
      <FieldGroup label={t('data.workspaceAt')} help={t('data.restoreHint')}>
        {/* The line keeps its place while the path is on its way. Everything
            else on this screen is static, so a skeleton of the whole thing
            would be a lie about what is loading — but this one line arriving
            late pushed the paragraph under it down, which is the jump. */}
        {path.data == null ? (
          <Skeleton className="h-3 w-72" />
        ) : (
          <code className="selectable font-mono text-xs break-all text-text">{path.data}</code>
        )}
      </FieldGroup>

      {/* Where the application writes what went wrong while nobody was
          looking: the file a report of "it broke yesterday" can attach. */}
      {log.data != null && (
        <FieldGroup label={t('data.logAt')} help={t('data.logHint')}>
          <code className="selectable font-mono text-xs break-all text-text">{log.data}</code>
        </FieldGroup>
      )}

      {/* Where the work's media lives outside the workspace: a place on this
          machine, beside the place the workspace itself lives. */}
      <MediaFolderField />

      <FieldGroup label={t('data.import')} help={t('data.importHint')}>
        <Button className="self-start" disabled={busy} onClick={doImport}>
          {t('data.importAction')}
        </Button>
      </FieldGroup>

      {busy && <p className="text-sm text-dim">{t('data.working')}</p>}
    </div>
  )
}
