import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  backupWorkspace,
  exportMarkdown,
  importLegacy,
  mcpRegistration,
  suggestedBackupName,
  workspacePath,
} from '@/lib/api'
import { say } from '@/lib/toast'
import { useCardView } from '@/lib/cardView'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Skeleton } from '@/components/ui/Skeleton'
import { ProfileEditor } from '@/components/ProfileEditor'
import { StatusDrift } from '@/components/StatusDrift'

// Getting data out and in. The export is the "you are not locked in" promise
// made checkable; the backup is the whole workspace in one file.
export function DataView() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [busy, setBusy] = useState(false)
  const { view, setCardView } = useCardView()

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

      const summary = t('data.imported', {
        works: report.works,
        versions: report.versions,
        scores: report.scores,
        skipped: report.skipped,
      })
      // Said only when it happened: most imports have nothing to leave buried.
      return report.deleted > 0
        ? `${summary} ${t('data.importedLeftDeleted', { count: report.deleted })}`
        : summary
    })

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <ProfileEditor />

      <hr className="border-line" />

      <StatusDrift />

      <hr className="border-line" />

      {/* What the card draws, as this machine likes it. A switch and not a
          checkbox: there is no Save button on this screen, and the card two
          routes away changes the moment it moves. */}
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.cardView')}</h3>
        <p className="text-sm text-dim">{t('data.cardViewHint')}</p>
        <Switch
          checked={view.metaStrip}
          onCheckedChange={(on) => setCardView({ metaStrip: on })}
        >
          {t('data.showMetaStrip')}
        </Switch>
      </section>

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
        {/* The line keeps its place while the path is on its way. Everything
            else on this screen is static, so a skeleton of the whole thing
            would be a lie about what is loading — but this one line arriving
            late pushed the paragraph under it down, which is the jump. */}
        <p className="text-xs text-dim">
          {path.data == null ? (
            <Skeleton className="h-3 w-72" />
          ) : (
            <>
              {t('data.workspaceAt')}{' '}
              <code className="selectable font-mono">{path.data}</code>
            </>
          )}
        </p>
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

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold">{t('data.mcp')}</h3>
        <p className="text-sm text-dim">{t('data.mcpHint')}</p>
        <McpRegistration />
        <p className="text-xs text-dim">{t('data.mcpAfter')}</p>
      </section>

      {busy && <p className="text-sm text-dim">{t('data.working')}</p>}
    </div>
  )
}

/**
 * The command that registers this build with Claude Code, ready to copy.
 *
 * Shown rather than run: kilna does not know which shell, which agent or
 * whether the person wants it at all. The path is this executable's own,
 * so it is right for the build in front of them and wrong for none.
 */
function McpRegistration() {
  const { t } = useTranslation()
  const command = useQuery({ queryKey: ['mcpRegistration'], queryFn: mcpRegistration })
  const [copied, setCopied] = useState(false)

  if (command.data === undefined) return null

  return (
    <div className="flex flex-wrap items-center gap-2">
      <code className="selectable min-w-0 flex-1 overflow-x-auto rounded-md border border-line bg-soft px-2.5 py-1.5 font-mono text-xs whitespace-nowrap">
        {command.data}
      </code>
      <Button
        size="sm"
        onClick={() => {
          void navigator.clipboard.writeText(command.data).then(() => {
            setCopied(true)
            setTimeout(() => setCopied(false), 2000)
          })
        }}
      >
        {copied ? t('data.mcpCopied') : t('data.mcpCopy')}
      </Button>
    </div>
  )
}
