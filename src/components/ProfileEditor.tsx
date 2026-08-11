import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { updateProfileConfig, type Axis, type Kind, type ProfileConfig, type Tier } from '@/lib/api'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'

interface Props {
  onSaved: () => void
}

// Editing the scenario, not designing a schema: the tables never change, only
// the vocabulary and the criteria. Axis keys are deliberately not editable —
// past score snapshots are keyed by them, and renaming a key would orphan them.
export function ProfileEditor({ onSaved }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [config, setConfig] = useState<ProfileConfig>(profile.config)
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)

  const patch = (changes: Partial<ProfileConfig>) => {
    setConfig((current) => ({ ...current, ...changes }))
    setSaved(false)
  }

  const save = () => {
    updateProfileConfig(profile.id, config)
      .then(() => {
        setSaved(true)
        onSaved()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  const setAxis = (index: number, changes: Partial<Axis>) => {
    patch({ axes: config.axes.map((axis, i) => (i === index ? { ...axis, ...changes } : axis)) })
  }

  const setTier = (index: number, changes: Partial<Tier>) => {
    patch({ tiers: config.tiers.map((tier, i) => (i === index ? { ...tier, ...changes } : tier)) })
  }

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <header>
        <h3 className="text-sm font-semibold">{profile.name}</h3>
        <p className="text-sm text-neutral-500">{profile.description}</p>
      </header>

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-neutral-500">
          {t('editor.axes')}
        </h4>
        <p className="text-xs text-neutral-500">{t('editor.axesHint')}</p>
        <ul className="flex flex-col gap-1.5">
          {config.axes.map((axis, index) => (
            <li key={axis.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-neutral-500">{axis.key}</code>
              <Input
                className="flex-1"
                value={axis.label}
                onChange={(event) => setAxis(index, { label: event.target.value })}
                aria-label={`${axis.key} label`}
              />
              <Input
                className="w-20"
                type="number"
                min={0}
                step={0.5}
                value={axis.weight}
                onChange={(event) => setAxis(index, { weight: Number(event.target.value) })}
                aria-label={`${axis.key} weight`}
              />
              <Button
                size="sm"
                variant="danger"
                title={t('editor.removeAxis')}
                onClick={() =>
                  patch({ axes: config.axes.filter((_, i) => i !== index) })
                }
              >
                ✕
              </Button>
            </li>
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-neutral-500">
          {t('editor.tiers')}
        </h4>
        <p className="text-xs text-neutral-500">{t('editor.tiersHint')}</p>
        <ul className="flex flex-col gap-1.5">
          {config.tiers.map((tier, index) => (
            <li key={tier.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-neutral-500">{tier.key}</code>
              <Input
                className="flex-1"
                value={tier.label}
                onChange={(event) => setTier(index, { label: event.target.value })}
                aria-label={`${tier.key} label`}
              />
              <Input
                className="w-20"
                type="number"
                min={0}
                max={100}
                value={tier.min}
                onChange={(event) => setTier(index, { min: Number(event.target.value) })}
                aria-label={`${tier.key} threshold`}
              />
            </li>
          ))}
        </ul>
      </section>

      <Vocabulary
        label={t('editor.statuses')}
        entries={config.statuses}
        onChange={(statuses) => patch({ statuses })}
      />
      <Vocabulary
        label={t('editor.workKinds')}
        entries={config.work_kinds}
        onChange={(work_kinds) => patch({ work_kinds })}
      />
      <Vocabulary
        label={t('editor.releaseKinds')}
        entries={config.release_kinds}
        onChange={(release_kinds) => patch({ release_kinds })}
      />

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-neutral-500">
          {t('editor.prompts')}
        </h4>
        <ul className="flex flex-col gap-1.5">
          {config.prompts.map((prompt, index) => (
            <li key={prompt.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-neutral-500">{prompt.key}</code>
              <Input
                className="flex-1"
                value={prompt.label}
                onChange={(event) =>
                  patch({
                    prompts: config.prompts.map((p, i) =>
                      i === index ? { ...p, label: event.target.value } : p,
                    ),
                  })
                }
                aria-label={`${prompt.key} label`}
              />
            </li>
          ))}
        </ul>
      </section>

      <div className="flex items-center gap-3">
        <Button variant="primary" onClick={save}>
          {t('editor.save')}
        </Button>
        {saved && <span className="text-sm text-emerald-600">{t('editor.saved')}</span>}
        {error !== null && (
          <span role="alert" className="text-sm text-red-600">
            {error}
          </span>
        )}
      </div>

      <p className="text-xs text-neutral-500">{t('editor.keysHint')}</p>
    </div>
  )
}

interface VocabularyProps {
  label: string
  entries: Kind[]
  onChange: (entries: Kind[]) => void
}

function Vocabulary({ label, entries, onChange }: VocabularyProps) {
  return (
    <section className="flex flex-col gap-2">
      <h4 className="text-xs font-medium uppercase tracking-wide text-neutral-500">{label}</h4>
      <ul className="flex flex-wrap gap-1.5">
        {entries.map((entry, index) => (
          <li key={entry.key} className="flex items-center gap-1">
            <Input
              className="w-40"
              value={entry.label}
              onChange={(event) =>
                onChange(
                  entries.map((e, i) => (i === index ? { ...e, label: event.target.value } : e)),
                )
              }
              aria-label={entry.key}
            />
          </li>
        ))}
      </ul>
    </section>
  )
}
