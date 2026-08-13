import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { listPlugins, runPlugin, type Plugin } from '@/lib/api'
import { Button } from '@/components/ui/Button'

interface Props {
  target: 'work' | 'release'
  id: string
  onChanged: () => void
}

// Actions contributed by installed plugins. Nothing shows when none are
// installed — an empty toolbar advertising a feature the user does not have is
// worse than no toolbar.
export function PluginBar({ target, id, onChanged }: Props) {
  const { t } = useTranslation()
  const [plugins, setPlugins] = useState<Plugin[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listPlugins()
      .then(setPlugins)
      .catch((cause: unknown) => setError(String(cause)))
  }, [])

  const actions = plugins.flatMap((plugin) =>
    plugin.usable && plugin.manifest !== null
      ? plugin.manifest.commands
          .filter((command) => command.target === target)
          .map((command) => ({ plugin, command }))
      : [],
  )

  const unusable = plugins.filter((plugin) => !plugin.usable)

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
              disabled={busy}
              title={command.description}
              onClick={() => {
                setBusy(true)
                setError(null)
                setMessage(null)
                runPlugin(plugin.executable, command.key, target, id)
                  .then((said) => {
                    setMessage(said)
                    onChanged()
                  })
                  .catch((cause: unknown) => setError(String(cause)))
                  .finally(() => setBusy(false))
              }}
            >
              {command.label}
            </Button>
          ))}
        </div>
      )}

      {message !== null && <p className="text-sm text-dim">{message}</p>}
      {error !== null && (
        <p role="alert" className="text-sm text-bad">
          {error}
        </p>
      )}

      {unusable.map((plugin) => (
        <p key={plugin.executable} className="text-xs text-warn">
          {plugin.executable}: {plugin.reason}
        </p>
      ))}
    </section>
  )
}
