import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { updateProfileConfig, type Axis, type Kind, type ProfileConfig, type Tier } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'

// Editing the scenario, not designing a schema: the tables never change, only
// the vocabulary and the criteria. Axis keys are deliberately not editable —
// past score snapshots are keyed by them, and renaming a key would orphan them.
export function ProfileEditor() {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [config, setConfig] = useState<ProfileConfig>(profile.config)

  const patch = (changes: Partial<ProfileConfig>) => {
    setConfig((current) => ({ ...current, ...changes }))
  }

  const save = useMutation({
    mutationFn: () => updateProfileConfig(profile.id, config),
    onSuccess: () => {
      // The vocabulary is on every screen: labels, statuses, kinds, axes.
      void client.invalidateQueries({ queryKey: keys.workspace })
      void client.invalidateQueries({ queryKey: keys.profiles })
      void client.invalidateQueries({ queryKey: keys.catalogue })
    },
    onError: (cause) => say.failedTo(t('toast.profileSaveFailed'), cause),
  })

  const saveStatus = useSaveStatus(save.isPending, save.isError)

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
        <p className="text-sm text-dim">{profile.description}</p>
      </header>

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
          {t('editor.axes')}
        </h4>
        <p className="text-xs text-dim">{t('editor.axesHint')}</p>
        <ul className="flex flex-col gap-1.5">
          {config.axes.map((axis, index) => (
            <li key={axis.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-dim">{axis.key}</code>
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
                variant="danger"
                size="icon-sm"
                title={t('editor.removeAxis')}
                onClick={() =>
                  patch({ axes: config.axes.filter((_, i) => i !== index) })
                }
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </li>
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
          {t('editor.tiers')}
        </h4>
        <p className="text-xs text-dim">{t('editor.tiersHint')}</p>
        <ul className="flex flex-col gap-1.5">
          {config.tiers.map((tier, index) => (
            <li key={tier.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-dim">{tier.key}</code>
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

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
          {t('editor.rhythm')}
        </h4>
        <p className="text-xs text-dim">{t('editor.rhythmHint')}</p>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2 text-sm">
            <Input
              className="w-20"
              type="number"
              min={1}
              value={config.rhythm?.every_days ?? ''}
              aria-label={t('editor.rhythmDays')}
              onChange={(event) => {
                const raw = event.target.value
                // Clearing the field removes the rhythm entirely — "no pace"
                // is a valid answer, and the layout button explains it.
                patch({
                  rhythm:
                    raw === ''
                      ? null
                      : {
                          every_days: Math.max(1, Math.trunc(Number(raw))),
                          default_time: config.rhythm?.default_time ?? null,
                        },
                })
              }}
            />
            {t('editor.rhythmDaysUnit')}
          </label>
          <label className="ml-4 flex items-center gap-2 text-sm">
            {t('editor.rhythmTime')}
            <Input
              className="w-28"
              type="time"
              value={config.rhythm?.default_time ?? ''}
              disabled={config.rhythm == null}
              aria-label={t('editor.rhythmTime')}
              onChange={(event) => {
                if (config.rhythm == null) return
                patch({
                  rhythm: {
                    ...config.rhythm,
                    default_time: event.target.value === '' ? null : event.target.value,
                  },
                })
              }}
            />
          </label>
        </div>
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
        <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
          {t('editor.prompts')}
        </h4>
        <ul className="flex flex-col gap-1.5">
          {config.prompts.map((prompt, index) => (
            <li key={prompt.key} className="flex items-center gap-2">
              <code className="w-28 shrink-0 font-mono text-xs text-dim">{prompt.key}</code>
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
        <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate()}>
          {t('editor.save')}
        </Button>
        <SaveState status={saveStatus} />
      </div>

      <p className="text-xs text-dim">{t('editor.keysHint')}</p>
    </div>
  )
}

interface VocabularyProps<T extends Kind> {
  label: string
  entries: T[]
  onChange: (entries: T[]) => void
}

// Generic over the entry, because a status carries a `derive` role alongside
// its label and a kind does not — and renaming one must not drop the other.
function Vocabulary<T extends Kind>({ label, entries, onChange }: VocabularyProps<T>) {
  return (
    <section className="flex flex-col gap-2">
      <h4 className="text-xs font-medium uppercase tracking-wide text-dim">{label}</h4>
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
