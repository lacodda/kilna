import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Square } from 'lucide-react'
import { cancelPlugin, onPluginProgress, runPlugin, type PluginTarget } from '@/lib/api/plugins'
import type { PluginReply } from '@/lib/api/types'
import { nameOf } from '@/lib/media'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'

interface Props {
  target: PluginTarget
  id: string
}

/** A run going on now: which command, and how far it says it is. */
interface Running {
  run: string
  label: string
  /** From 0 to 1, or `null` while the plugin has not said. */
  share: number | null
  message: string | null
}

// Actions contributed by installed plugins. Nothing shows when none are
// installed — an empty toolbar advertising a feature the user does not have is
// worse than no toolbar.
export function PluginBar({ target, id }: Props) {
  const { t } = useTranslation()

  const plugins = useQuery(queries.plugins())
  const [running, setRunning] = useState<Running | null>(null)

  // What a plugin writes about itself while it runs (v0.101): a share and a
  // line, on the run this bar started. Another bar's run is not this one's.
  useEffect(() => {
    let stop: (() => void) | undefined
    let gone = false
    void onPluginProgress((step) => {
      setRunning((now) =>
        now?.run === step.run
          ? {
              ...now,
              share: step.progress ?? now.share,
              message: step.message ?? now.message,
            }
          : now,
      )
    }).then((unlisten) => {
      if (gone) unlisten()
      else stop = unlisten
    })
    return () => {
      gone = true
      stop?.()
    }
  }, [])

  // Set when the person stops the run, so its ending is said as a stop
  // rather than as the failure the plugin's killing reads as.
  const stopped = useRef(false)

  const run = useAppMutation({
    mutationFn: async ({
      executable,
      command,
      label,
      run: runId,
    }: {
      executable: string
      command: string
      label: string
      run: string
    }) => {
      stopped.current = false
      setRunning({ run: runId, label, share: null, message: null })
      try {
        return await runPlugin(executable, command, target, id, runId)
      } finally {
        setRunning(null)
      }
    },
    // A plugin may rewrite the work it was handed, or make its finished
    // file, so nothing local is trusted afterwards.
    refresh:
      target === 'release'
        ? refresh.version(id)
        : [...refresh.version(id), ...refresh.finishedFile(id)],
    onError: (cause) => {
      if (stopped.current) say.info(t('plugins.stopped'))
      else say.failed(cause)
    },
    onSuccess: (reply: PluginReply) => {
      // Plugins report in their own words; an empty answer means "done".
      if (reply.message !== null && reply.message.trim() !== '') say.info(reply.message)
      const first = reply.files.at(0)
      if (reply.chosen && first !== undefined) {
        say.ok(t('plugins.madeFinished', { name: nameOf(first) }))
      } else if (reply.files.length > 0) {
        say.ok(t('plugins.madeFiles', { count: reply.files.length }))
      }
    },
  })

  const installed = plugins.data ?? []

  const actions = installed.flatMap((plugin) =>
    plugin.usable && plugin.manifest !== null
      ? plugin.manifest.commands
          .filter((command) => command.target === target)
          .map((command) => ({ plugin, command }))
      : [],
  )

  const unusable = installed.filter((plugin) => !plugin.usable)

  if (actions.length === 0 && unusable.length === 0) return null

  // One line, in the head of the overview beside the profile's actions: the
  // name of the group as a caption, the commands, and a plugin that cannot
  // run saying why in a word of warning - the whole reason on hover, since a
  // line of a head has no room for a paragraph. While a command runs, the
  // line is its progress and the way to stop it.
  return (
    <div
      role="group"
      aria-label={t('plugins.title')}
      className="flex min-w-0 flex-wrap items-center gap-1.5"
    >
      <span className="caption">{t('plugins.title')}</span>

      {running !== null ? (
        <div className="flex min-w-60 flex-1 items-center gap-2">
          <span className="shrink-0 text-xs">{running.label}</span>
          <Progress
            size="sm"
            className="min-w-24 flex-1"
            label={t('plugins.progress', { label: running.label })}
            value={running.share === null ? null : running.share * 100}
          />
          {running.message !== null && (
            <span className="max-w-64 truncate text-xs text-dim" title={running.message}>
              {running.message}
            </span>
          )}
          <Button
            size="xs"
            variant="ghost"
            onClick={() => {
              stopped.current = true
              cancelPlugin(running.run).catch((cause: unknown) => say.failed(cause))
            }}
          >
            <Square aria-hidden />
            {t('plugins.stop')}
          </Button>
        </div>
      ) : (
        actions.map(({ plugin, command }) => (
          <Button
            key={`${plugin.executable}:${command.key}`}
            size="sm"
            disabled={run.isPending}
            title={command.description ?? undefined}
            onClick={() =>
              run.mutate({
                executable: plugin.executable,
                command: command.key,
                label: command.label,
                run: crypto.randomUUID(),
              })
            }
          >
            {command.label}
          </Button>
        ))
      )}

      {unusable.map((plugin) => (
        <span
          key={plugin.executable}
          className="max-w-64 truncate text-xs text-warn"
          title={`${plugin.executable}: ${plugin.reason ?? ''}`}
        >
          {plugin.executable}: {plugin.reason}
        </span>
      ))}
    </div>
  )
}
