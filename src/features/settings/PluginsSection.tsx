import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { setPluginEnabled, setPluginSetting } from '@/lib/api/plugins'
import type { JsonValue, Plugin, PluginSetting } from '@/lib/api/types'
import { openExternal } from '@/lib/link'
import { inLanguage } from '@/lib/pluginWords'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'
import { Skeleton } from '@/components/ui/skeleton'
import { StatusDot } from '@/components/ui/status-dot'
import { Switch } from '@/components/ui/switch'
import { Select } from '@/components/AppSelect'

/** The page that says how a plugin is written. */
const GUIDE = 'https://lacodda.github.io/kilna/guides/writing-a-plugin/'

/**
 * The plugins on this machine (v0.108): what each is, whether it can run, where
 * its commands stand, and what can be set for it.
 *
 * Until now a plugin that could not run said why in the head of every work, and
 * one that could was known only by its buttons. Here each has a place: turned
 * off, it offers nothing and stays listed; its settings are drawn from what its
 * manifest declares, and what is shown is what a run is told.
 */
export function PluginsSection() {
  const { t } = useTranslation()
  const plugins = useQuery(queries.plugins())
  const folder = useQuery({ ...queries.pluginsFolder(), staleTime: Infinity })

  const reveal = () => {
    if (folder.data != null) {
      revealItemInDir(folder.data).catch((cause: unknown) => say.failed(cause))
    }
  }

  const installed = plugins.data ?? []

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <FieldGroup label={t('plugins.folder')} help={t('plugins.folderHint')}>
        {folder.data == null ? (
          <Skeleton className="h-3 w-72" />
        ) : (
          <code className="selectable font-mono text-xs break-all text-text">{folder.data}</code>
        )}
        <div className="flex flex-wrap gap-2">
          <Button disabled={folder.data == null} onClick={reveal}>
            {t('plugins.showFolder')}
          </Button>
          {/* A plugin dropped in the folder a moment ago is what this is for:
              the list is only read when asked. */}
          <Button
            variant="ghost"
            disabled={plugins.isFetching}
            onClick={() => {
              void plugins.refetch()
            }}
          >
            {t('plugins.lookAgain')}
          </Button>
        </div>
      </FieldGroup>

      {plugins.isPending ? (
        <Skeleton className="h-24 w-full" />
      ) : installed.length === 0 ? (
        <EmptyState
          title={t('plugins.noneTitle')}
          body={t('plugins.noneBody')}
          action={
            <Button
              onClick={() => {
                void openExternal(GUIDE)
              }}
            >
              {t('plugins.guide')}
            </Button>
          }
        />
      ) : (
        <ul className="flex flex-col gap-2.5">
          {installed.map((plugin) => (
            <PluginBlock key={plugin.executable} plugin={plugin} />
          ))}
        </ul>
      )}
    </div>
  )
}

/** One plugin: its name and state in the bar, then what it offers and takes. */
function PluginBlock({ plugin }: { plugin: Plugin }) {
  const { t, i18n } = useTranslation()
  const manifest = plugin.manifest

  const toggle = useAppMutation({
    mutationFn: (enabled: boolean) => setPluginEnabled(plugin.executable, enabled),
    failure: 'plugins.saveFailed',
    refresh: [keys.plugins],
  })

  const state = !plugin.usable
    ? { status: 'bad' as const, label: t('plugins.state.unusable') }
    : plugin.enabled
      ? { status: 'good' as const, label: t('plugins.state.on') }
      : { status: 'neutral' as const, label: t('plugins.state.off') }

  const description =
    manifest === null
      ? null
      : inLanguage(i18n.language, manifest.description ?? '', manifest.descriptions)

  return (
    <li
      data-plugin={plugin.executable}
      className="flex flex-col gap-3 rounded-md border border-line bg-softer px-3.5 py-3"
    >
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <span className="font-semibold text-text">{plugin.executable}</span>
        {manifest !== null && (
          <span className="font-mono text-xs text-dim tabular-nums">{manifest.version}</span>
        )}
        <StatusDot status={state.status} label={state.label} showLabel />
        <Switch
          className="ml-auto"
          checked={plugin.enabled}
          disabled={toggle.isPending}
          onCheckedChange={(enabled) => toggle.mutate(enabled)}
        >
          {t('plugins.enabled')}
        </Switch>
      </div>

      {description !== null && description !== '' && (
        <p className="text-sm text-dim">{description}</p>
      )}
      {plugin.reason !== null && <p className="text-sm break-words text-bad">{plugin.reason}</p>}

      {manifest !== null && manifest.commands.length > 0 && (
        <dl className="grid grid-cols-[minmax(0,14rem)_minmax(0,1fr)] gap-x-3 gap-y-0.5 text-sm">
          {manifest.commands.map((command) => (
            <div key={command.key} className="contents">
              <dt className="truncate text-text">
                {inLanguage(i18n.language, command.label, command.labels)}
              </dt>
              <dd className="text-dim">{t(`plugins.where.${command.target}`)}</dd>
            </div>
          ))}
        </dl>
      )}

      {manifest !== null && manifest.settings.length > 0 && (
        <div className="flex flex-col gap-3 border-t border-line pt-3">
          <h4 className="caption">{t('plugins.settings')}</h4>
          <div className="flex max-w-105 flex-col gap-3">
            {manifest.settings.map((setting) => (
              <SettingField key={setting.key} plugin={plugin} setting={setting} />
            ))}
          </div>
        </div>
      )}

      <code className="selectable font-mono text-xs break-all text-faint">{plugin.path}</code>
    </li>
  )
}

/**
 * One setting, drawn from what the manifest says it is.
 *
 * Shows what a run would be told: what was set here, or the plugin's default.
 * A value set here can be taken back, and from then on follows the plugin's
 * default again, whatever its next release makes it.
 */
function SettingField({ plugin, setting }: { plugin: Plugin; setting: PluginSetting }) {
  const { t, i18n } = useTranslation()
  const label = inLanguage(i18n.language, setting.label, setting.labels)
  const help = inLanguage(i18n.language, setting.description ?? '', setting.descriptions)
  const isSet = plugin.settings[setting.key] !== undefined
  const value = plugin.settings[setting.key] ?? setting.default ?? null

  const save = useAppMutation({
    mutationFn: (next: JsonValue | null) => setPluginSetting(plugin.executable, setting.key, next),
    failure: 'plugins.saveFailed',
    refresh: [keys.plugins],
  })

  const hint = help === '' ? undefined : help

  return (
    <div className="flex flex-col gap-1">
      <SettingControl
        setting={setting}
        label={label}
        hint={hint}
        value={value}
        busy={save.isPending}
        onSettle={(next) => save.mutate(next)}
        language={i18n.language}
      />
      {isSet && (
        <Button
          size="xs"
          variant="ghost"
          className="self-start"
          disabled={save.isPending}
          onClick={() => save.mutate(null)}
        >
          {t('plugins.backToDefault')}
        </Button>
      )}
    </div>
  )
}

/** The control a setting's type asks for. */
function SettingControl({
  setting,
  label,
  hint,
  value,
  busy,
  onSettle,
  language,
}: {
  setting: PluginSetting
  label: string
  hint: string | undefined
  value: JsonValue | null
  busy: boolean
  onSettle: (next: JsonValue) => void
  language: string
}) {
  switch (setting.type) {
    case 'boolean':
      return (
        <Field label={label} help={hint} beside>
          <Switch
            checked={value === true}
            disabled={busy}
            onCheckedChange={(checked) => onSettle(checked)}
          />
        </Field>
      )
    case 'number':
      return (
        <NumberSetting
          // Keyed by what stands: taking a value back puts the default in the box.
          key={String(value)}
          label={label}
          help={hint}
          stored={typeof value === 'number' ? value : null}
          min={setting.min}
          max={setting.max}
          step={setting.step}
          onSettle={onSettle}
        />
      )
    case 'choice':
      return (
        <FieldGroup label={label} help={hint}>
          <Select
            aria-label={label}
            value={typeof value === 'string' ? value : ''}
            onChange={onSettle}
            options={setting.options.map((option) => ({
              value: option.value,
              label: inLanguage(language, option.label, option.labels),
            }))}
          />
        </FieldGroup>
      )
    case 'text':
      return (
        <TextSetting
          key={String(value)}
          label={label}
          help={hint}
          stored={typeof value === 'string' ? value : ''}
          onSettle={onSettle}
        />
      )
  }
}

/** A number, saved on leaving: each arrow press is not a save of its own. An
 *  emptied box is not a value, and goes back to what stands. */
function NumberSetting({
  label,
  help,
  stored,
  min,
  max,
  step,
  onSettle,
}: {
  label: string
  help: ReactNode
  stored: number | null
  min?: number
  max?: number
  step?: number
  onSettle: (next: number) => void
}) {
  const [draft, setDraft] = useState(stored)

  return (
    <Field label={label} help={help}>
      <div
        onBlur={() => {
          if (draft === null) setDraft(stored)
          else if (draft !== stored) onSettle(draft)
        }}
      >
        <NumberField value={draft} min={min} max={max} step={step} onValueChange={setDraft} />
      </div>
    </Field>
  )
}

/** A line of text, saved on leaving or on Enter. */
function TextSetting({
  label,
  help,
  stored,
  onSettle,
}: {
  label: string
  help: ReactNode
  stored: string
  onSettle: (next: string) => void
}) {
  const [draft, setDraft] = useState(stored)
  const settle = () => {
    if (draft !== stored) onSettle(draft)
  }

  return (
    <Field label={label} help={help}>
      <Input
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={settle}
        onKeyDown={(event) => {
          if (event.key === 'Enter') settle()
          if (event.key === 'Escape') setDraft(stored)
        }}
      />
    </Field>
  )
}
