import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { backupWorkspace, exportMarkdown, importLegacy, suggestedBackupName } from '@/lib/api/data'
import { nameOf } from '@/lib/media'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Skeleton } from '@/components/ui/skeleton'
import { MediaFolderField } from '@/features/settings/MediaFolderField'

/** What a finished gesture says, and where it can be seen when it left something on disk. */
interface Said {
  message: string
  shows?: string
}

/** Show a place in the system's file manager, saying so if it cannot. */
const reveal = (path: string) => {
  revealItemInDir(path).catch((cause: unknown) => say.failed(cause))
}

// Getting data out and in. The export is the "you are not locked in" promise
// made checkable; the backup is the whole workspace in one file and a folder
// beside it; a restore is steps a person takes with kilna closed, written down
// exactly.
export function DataSection() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)

  // The path never changes while the app runs, so it is asked for once.
  const path = useQuery({ ...queries.workspacePath(), staleTime: Infinity })
  const log = useQuery({ ...queries.logPath(), staleTime: Infinity })

  // Each of these opens an OS file dialog first, so they are not mutations in
  // the query sense — there is nothing to retry and no variables to carry.
  // `null` means the dialog was dismissed: nothing happened, so nothing is said.
  const run = (task: () => Promise<Said | null>) => {
    setBusy(true)
    task()
      .then((said) => {
        if (said === null) return
        const { message, shows } = said
        if (shows === undefined) say.ok(message)
        else say.withAction(message, t('data.show'), () => reveal(shows))
      })
      .catch((cause: unknown) => say.failed(cause))
      .finally(() => setBusy(false))
  }

  const doExport = () =>
    run(async () => {
      const directory = await open({ directory: true, title: t('data.exportTitle') })
      if (typeof directory !== 'string') return null

      const report = await exportMarkdown(directory)
      return {
        message: t('data.exported', { files: report.files, works: report.works }),
        shows: directory,
      }
    })

  const doBackup = () =>
    run(async () => {
      const suggested = await suggestedBackupName()
      const destination = await save({ defaultPath: suggested, title: t('data.backupTitle') })
      if (destination === null) return null

      const written = await backupWorkspace(destination)
      return { message: t('data.backedUp', { path: written }), shows: written }
    })

  const doImport = () =>
    run(async () => {
      const source = await open({
        title: t('data.importTitle'),
        filters: [{ name: 'SQLite', extensions: ['db', 'sqlite'] }],
      })
      if (typeof source !== 'string') return null

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
      return {
        message:
          report.deleted > 0
            ? `${summary}${material} ${t('data.importedLeftDeleted', { count: report.deleted })}`
            : `${summary}${material}`,
      }
    })

  return (
    // A field per way in or out, each explained under its button, as the
    // mockup's fields are - not a heading and a paragraph the button waits
    // below. `self-start` keeps a button its own width in the column. Three
    // groups, by what someone comes here to do.
    <div className="flex max-w-105 flex-col gap-6">
      <section className="flex flex-col gap-4">
        <h3 className="caption">{t('data.groupOut')}</h3>
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
      </section>

      <section className="flex flex-col gap-4">
        <h3 className="caption">{t('data.groupIn')}</h3>
        <RestoreSteps path={path.data ?? null} />

        <FieldGroup label={t('data.import')} help={t('data.importHint')}>
          <Button className="self-start" disabled={busy} onClick={doImport}>
            {t('data.importAction')}
          </Button>
        </FieldGroup>
      </section>

      <section className="flex flex-col gap-4">
        <h3 className="caption">{t('data.groupWhere')}</h3>
        <FieldGroup label={t('data.workspaceAt')} help={t('data.workspaceAtHint')}>
          {/* The line keeps its place while the path is on its way. Everything
              else on this screen is static, so a skeleton of the whole thing
              would be a lie about what is loading — but this one line arriving
              late pushed the paragraph under it down, which is the jump. */}
          {path.data == null ? (
            <Skeleton className="h-3 w-72" />
          ) : (
            <code className="selectable font-mono text-xs break-all text-text">{path.data}</code>
          )}
          <Button
            className="self-start"
            disabled={path.data == null}
            onClick={() => {
              if (path.data != null) reveal(path.data)
            }}
          >
            {t('data.showFile')}
          </Button>
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
      </section>

      {busy && <p className="text-sm text-dim">{t('data.working')}</p>}
    </div>
  )
}

/**
 * How a backup goes back in: the steps, in order, by the names they have on this
 * machine.
 *
 * Steps rather than a button, because a workspace cannot be replaced under the
 * process that holds it open. Each step is here because skipping it breaks
 * something: closing the window does not close kilna while work goes on (v0.107),
 * a write-ahead log left beside a restored file belongs to another database, and
 * the files of the backup lie in a folder of their own. `backup.rs` follows these
 * same steps in a test.
 */
function RestoreSteps({ path }: { path: string | null }) {
  const { t } = useTranslation()
  const file = path === null ? 'kilna.db' : nameOf(path)

  return (
    <FieldGroup label={t('data.restore')} help={t('data.restoreLeft')}>
      <p className="text-sm text-dim">{t('data.restoreHint')}</p>
      <ol className="flex list-decimal flex-col gap-1.5 pl-5 text-sm text-text">
        <li>{t('data.restoreSteps.close')}</li>
        <li>{t('data.restoreSteps.aside', { file })}</li>
        <li>{t('data.restoreSteps.copy', { file })}</li>
        <li>{t('data.restoreSteps.open')}</li>
      </ol>
    </FieldGroup>
  )
}
