import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown } from 'lucide-react'
import type { Plugin, PluginBatchReport } from '@/lib/api/types'
import { cancelPlugin, onPluginBatch, runPluginBatch } from '@/lib/api/plugins'
import { inLanguage } from '@/lib/pluginWords'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Dialog } from '@/components/AppDialog'
import { ActionBarButton } from '@/components/ui/action-bar'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Progress } from '@/components/ui/progress'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

interface Props {
  /** The works ticked right now - or a collection's works. */
  workIds: readonly string[]
  /** Drawn as a bar's button (the catalogue's) or a plain one. */
  inBar?: boolean
}

interface Chosen {
  plugin: Plugin
  command: string
  label: string
}

/**
 * A plugin's batch command over many works at once (v0.155, FR-21 21.3): a
 * render by a preset for every track ticked, or every work of a collection.
 * The command is chosen, then how many run at a time - one to four - and the
 * batch shows how far it is, stops on a press, and says work by work how it
 * ended.
 */
export function BulkPlugins({ workIds, inBar = true }: Props) {
  const { t, i18n } = useTranslation()
  const plugins = useQuery(queries.plugins())
  const [chosen, setChosen] = useState<Chosen | null>(null)
  const [parallel, setParallel] = useState('2')
  const [progress, setProgress] = useState<{ done: number; total: number; failed: number } | null>(
    null,
  )
  const [report, setReport] = useState<PluginBatchReport | null>(null)
  const batch = useRef('')

  useEffect(() => {
    let stop: (() => void) | undefined
    void onPluginBatch((event) => {
      if (event.batch === batch.current)
        setProgress({ done: event.done, total: event.total, failed: event.failed })
    }).then((unlisten) => {
      stop = unlisten
    })
    return () => stop?.()
  }, [])

  const run = useAppMutation({
    mutationFn: (one: Chosen) => {
      batch.current = `batch-${String(Date.now())}`
      setProgress({ done: 0, total: workIds.length, failed: 0 })
      return runPluginBatch(
        one.plugin.executable,
        one.command,
        workIds,
        Number(parallel),
        batch.current,
      )
    },
    failure: 'plugins.batchFailed',
    refresh: refresh.works,
    onSuccess: (done) => {
      setProgress(null)
      setReport(done)
    },
    onError: () => setProgress(null),
  })

  const commands = (plugins.data ?? []).flatMap((plugin) =>
    plugin.usable && plugin.enabled && plugin.manifest !== null
      ? plugin.manifest.commands
          .filter((command) => command.batch === true && command.target === 'work')
          .map((command) => ({
            plugin,
            command: command.key,
            label: inLanguage(i18n.language, command.label, command.labels),
          }))
      : [],
  )
  if (commands.length === 0 || workIds.length === 0) return null

  const trigger = inBar ? (
    <ActionBarButton render={<MenuTrigger render={<Button size="sm" variant="ghost" />} />}>
      {t('plugins.batchMenu')}
      <ChevronDown aria-hidden />
    </ActionBarButton>
  ) : (
    <MenuTrigger render={<Button size="sm" variant="ghost" />}>
      {t('plugins.batchMenu')}
      <ChevronDown aria-hidden />
    </MenuTrigger>
  )

  return (
    <>
      <Menu>
        {trigger}
        <MenuPopup align="start" side="top">
          {commands.map((one) => (
            <MenuItem
              key={`${one.plugin.executable}:${one.command}`}
              onClick={() => setChosen(one)}
            >
              {one.label}
            </MenuItem>
          ))}
        </MenuPopup>
      </Menu>

      {chosen !== null && (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open && !run.isPending) {
              setChosen(null)
              setReport(null)
            }
          }}
          title={t('plugins.batchTitle', { label: chosen.label, count: workIds.length })}
          footer={
            report === null ? (
              run.isPending ? (
                <Button variant="danger" onClick={() => void cancelPlugin(batch.current)}>
                  {t('plugins.batchStop')}
                </Button>
              ) : (
                <Button variant="primary" onClick={() => run.mutate(chosen)}>
                  {t('plugins.batchRun', { count: workIds.length })}
                </Button>
              )
            ) : undefined
          }
        >
          {report === null ? (
            <div className="flex flex-col gap-3">
              <div className="flex items-center gap-2">
                <span className="text-sm text-dim">{t('plugins.batchParallel')}</span>
                <SegmentedControl
                  aria-label={t('plugins.batchParallel')}
                  value={parallel}
                  onValueChange={setParallel}
                >
                  {['1', '2', '3', '4'].map((one) => (
                    <Segment key={one} value={one} disabled={run.isPending}>
                      {one}
                    </Segment>
                  ))}
                </SegmentedControl>
              </div>
              {progress !== null && (
                <div className="flex flex-col gap-1">
                  <Progress
                    value={progress.done}
                    max={Math.max(progress.total, 1)}
                    label={t('plugins.batchProgress')}
                  />
                  <span className="text-xs text-dim">
                    {t('plugins.batchDone', {
                      done: progress.done,
                      total: progress.total,
                      failed: progress.failed,
                    })}
                  </span>
                </div>
              )}
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              <p className="text-sm">
                {t(report.cancelled ? 'plugins.batchStopped' : 'plugins.batchFinished', {
                  ok: report.items.filter((one) => one.ok).length,
                  failed: report.items.filter((one) => !one.ok).length,
                })}
              </p>
              <ul className="flex flex-col gap-0.5 text-xs">
                {report.items.map((one) => (
                  <li key={one.id} className="flex gap-1.5">
                    <span className={one.ok ? 'text-good' : 'text-bad'}>{one.ok ? '✓' : '✕'}</span>
                    <span className="font-semibold">{one.title}</span>
                    {one.message !== null && <span className="text-dim">{one.message}</span>}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </Dialog>
      )}
    </>
  )
}
