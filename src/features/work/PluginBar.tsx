import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { listPlugins, runPlugin } from '@/lib/api'
import { keys } from '@/lib/query'
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
  const client = useQueryClient()

  const plugins = useQuery({ queryKey: keys.plugins, queryFn: listPlugins })

  const run = useMutation({
    mutationFn: ({ executable, command }: { executable: string; command: string }) =>
      runPlugin(executable, command, target, id),
    onSuccess: (said) => {
      // A plugin may rewrite the work it was handed, so nothing local is trusted
      // afterwards.
      void client.invalidateQueries({ queryKey: keys.work(id) })
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.versions(id) })

      // Plugins report in their own words; an empty answer means "done".
      if (said !== null && said.trim() !== '') say.info(said)
    },
    onError: (cause) => say.failed(cause),
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

  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">{t('plugins.title')}</h3>

      {actions.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {actions.map(({ plugin, command }) => (
            <Button
              key={`${plugin.executable}:${command.key}`}
              size="sm"
              disabled={run.isPending}
              title={command.description}
              onClick={() => run.mutate({ executable: plugin.executable, command: command.key })}
            >
              {command.label}
            </Button>
          ))}
        </div>
      )}

      {unusable.map((plugin) => (
        <p key={plugin.executable} className="text-xs text-warn">
          {plugin.executable}: {plugin.reason}
        </p>
      ))}
    </section>
  )
}
