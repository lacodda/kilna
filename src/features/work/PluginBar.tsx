import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { Square } from 'lucide-react'
import { cancelPlugin, onPluginProgress, runPlugin, type PluginTarget } from '@/lib/api/plugins'
import type { PluginReply } from '@/lib/api/types'
import { nameOf } from '@/lib/media'
import { inLanguage } from '@/lib/pluginWords'
import { keys } from '@/lib/query/keys'
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

// What a command on each target may have moved: nothing local is trusted once a
// plugin has answered.
function movedBy(target: PluginTarget, id: string) {
  switch (target) {
    case 'release':
      return [...refresh.release, keys.releaseFields(id)]
    case 'score':
      return [
        ...refresh.version(id),
        ...refresh.finishedFile(id),
        keys.pendingProposals,
        keys.transcripts,
      ]
    case 'work':
    case 'cuts':
      return [...refresh.version(id), ...refresh.finishedFile(id)]
  }
}

// Actions contributed by installed plugins. Nothing shows when none are
// installed — an empty toolbar advertising a feature the user does not have is
// worse than no toolbar.
export function PluginBar({ target, id }: Props) {
  const { t, i18n } = useTranslation()

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
    // A plugin may rewrite what it was handed, make a work's finished file or
    // propose its score.
    refresh: movedBy(target, id),
    onError: (cause) => {
      if (stopped.current) say.info(t('plugins.stopped'))
      else say.failed(cause)
    },
    onSuccess: (reply: PluginReply) => {
      // Plugins report in their own words; an empty answer means "done".
      if (reply.message !== null && reply.message.trim() !== '') say.info(reply.message)
      if (reply.finished !== null) {
        say.ok(t('plugins.madeFinished', { name: nameOf(reply.finished) }))
      } else if (reply.files.length > 0) {
        say.ok(t('plugins.madeFiles', { count: reply.files.length }))
      }
      if (reply.proposed_score !== null) say.ok(t('plugins.proposedScore'))
    },
  })

  const installed = plugins.data ?? []

  const actions = installed.flatMap((plugin) =>
    plugin.usable && plugin.enabled && plugin.manifest !== null
      ? plugin.manifest.commands
          .filter((command) => command.target === target)
          .map((command) => ({
            plugin,
            command,
            label: inLanguage(i18n.language, command.label, command.labels),
            description:
              command.description === null
                ? undefined
                : inLanguage(i18n.language, command.description, command.descriptions),
          }))
      : [],
  )

  // Said once on a card, in the head of the work: a card draws a bar per target,
  // and a plugin that cannot run may not even say which it means. One turned off
  // was turned off so as not to hear of it.
  const unusable =
    target === 'work' ? installed.filter((plugin) => !plugin.usable && plugin.enabled).length : 0

  if (actions.length === 0 && unusable === 0) return null

  // One line, in the head of the overview beside the profile's actions: the
  // name of the group as a caption, the commands, and a link to where the
  // plugins are said in full when one of them cannot run - a line of a head has
  // no room for the reason, and Settings does. While a command runs, the line
  // is its progress and the way to stop it.
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
        actions.map(({ plugin, command, label, description }) => (
          <Button
            key={`${plugin.executable}:${command.key}`}
            size="sm"
            disabled={run.isPending}
            title={description}
            onClick={() =>
              run.mutate({
                executable: plugin.executable,
                command: command.key,
                label,
                run: crypto.randomUUID(),
              })
            }
          >
            {label}
          </Button>
        ))
      )}

      {unusable > 0 && (
        <NavLink to="/settings/plugins" className="text-xs text-warn underline">
          {t('plugins.cannotRun', { count: unusable })}
        </NavLink>
      )}
    </div>
  )
}
