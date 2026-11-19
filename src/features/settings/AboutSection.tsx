import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { checkUpdate, diagnostics, diagnosticsText } from '@/lib/api/diagnostics'
import { openExternal } from '@/lib/link'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'

/** Where a problem is told: the project's issues. */
const ISSUES = 'https://github.com/lacodda/kilna/issues/new'

/**
 * About kilna (v0.162): which version this is and whether a newer one is out,
 * and what to send when something goes wrong - the version, the system, the
 * assistant's CLI and the log's last errors, copied as one text with the
 * person's home written `~`. Nothing leaves the machine unless a button says
 * so: no telemetry, a check for updates only on a press.
 */
export function AboutSection() {
  const { t } = useTranslation()
  const found = useQuery({ queryKey: ['diagnostics'], queryFn: diagnostics })

  const check = useAppMutation({
    mutationFn: checkUpdate,
    failure: 'about.checkFailed',
    onSuccess: (update) => {
      if (update.latest === null) say.info(t('about.cannotTell'))
      else if (update.newer) say.ok(t('about.newer', { version: update.latest }))
      else say.ok(t('about.latest'))
    },
  })

  const copy = useAppMutation({
    mutationFn: async () => {
      await navigator.clipboard.writeText(await diagnosticsText())
    },
    failure: 'about.copyFailed',
    onSuccess: () => say.ok(t('about.copied')),
  })

  const update = check.data
  return (
    <section className="flex max-w-105 flex-col gap-4">
      <h3 className="caption">{t('about.title')}</h3>
      <FieldGroup label={t('about.version')}>
        <span className="font-mono text-sm">
          {found.data === undefined ? '…' : `kilna ${found.data.version}`}
        </span>
        {found.data !== undefined && (
          <span className="text-xs text-dim">
            {[
              `${found.data.os} (${found.data.arch})`,
              found.data.webview === null ? null : `WebView ${found.data.webview}`,
              found.data.cli === null ? t('about.noCli') : `Claude Code ${found.data.cli}`,
            ]
              .filter((part) => part !== null)
              .join(' · ')}
          </span>
        )}
      </FieldGroup>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" disabled={check.isPending} onClick={() => check.mutate()}>
          {t('about.check')}
        </Button>
        {update?.newer === true && update.url !== null && (
          <Button size="sm" variant="primary" onClick={() => void openExternal(update.url!)}>
            {t('about.download', { version: update.latest })}
          </Button>
        )}
      </div>
      <FieldGroup label={t('about.problem')} help={t('about.problemHint')}>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" disabled={copy.isPending} onClick={() => copy.mutate()}>
            {t('about.copy')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void openExternal(ISSUES)}>
            {t('about.report')}
          </Button>
        </div>
      </FieldGroup>
    </section>
  )
}
