import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { runPlugin } from '@/lib/api/plugins'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'

interface Props {
  target: 'work' | 'release'
  id: string
}

// Actions contributed by installed plugins. Nothing shows when none are
// installed — an empty toolbar advertising a feature the user does not have is
// worse than no toolbar.
export function PluginBar({ target, id }: Props) {
  const { t } = useTranslation()

  const plugins = useQuery(queries.plugins())

  const run = useAppMutation({
    mutationFn: ({ executable, command }: { executable: string; command: string }) =>
      runPlugin(executable, command, target, id),
    // A plugin may rewrite the work it was handed, so nothing local is trusted
    // afterwards.
    refresh: refresh.version(id),
    onSuccess: (said) => {
      // Plugins report in their own words; an empty answer means "done".
      if (said !== null && said.trim() !== '') say.info(said)
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
  // line of a head has no room for a paragraph.
  return (
    <div
      role="group"
      aria-label={t('plugins.title')}
      className="flex min-w-0 flex-wrap items-center gap-1.5"
    >
      <span className="caption">{t('plugins.title')}</span>

      {actions.map(({ plugin, command }) => (
        <Button
          key={`${plugin.executable}:${command.key}`}
          size="sm"
          disabled={run.isPending}
          title={command.description ?? undefined}
          onClick={() => run.mutate({ executable: plugin.executable, command: command.key })}
        >
          {command.label}
        </Button>
      ))}

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
