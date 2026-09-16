import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Plus, X } from 'lucide-react'
import {
  updateProfileConfig,
  type Axis,
  type Kind,
  type ProfileConfig,
  type PromptTemplate,
  type Tier,
  type VersionRole,
  type WorkKind,
  type ReleaseKind,
  type ReleaseField,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { allOf, useProfile } from '@/lib/useProfile'
import { Select } from '@/components/ui/AppSelect'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'
import { Textarea } from '@/components/ui/textarea'

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

  // Writes go back into the same `work_kinds[i]` entry: copy the config,
  // replace the one kind, keep `format` and everything else untouched.
  const setKind = (index: number, changes: Partial<WorkKind>) => {
    patch({
      work_kinds: config.work_kinds.map((kind, i) =>
        i === index ? { ...kind, ...changes } : kind,
      ),
    })
  }

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <header>
        <h3 className="text-sm font-semibold">{profile.name}</h3>
        <p className="text-sm text-dim">{profile.description}</p>
      </header>

      {/* Since v0.57 the vocabulary belongs to the kind, not the profile: a
          song and a video are judged on different axes and go out through
          different doors. One section per `work_kinds[]` entry, heading
          being the kind's own label. */}
      {config.work_kinds.map((kind, kindIndex) => (
        <KindVocabulary
          key={kind.key}
          kind={kind}
          onChange={(changes) => setKind(kindIndex, changes)}
        />
      ))}

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
        label={t('editor.workKinds')}
        entries={config.work_kinds}
        onChange={(work_kinds) => patch({ work_kinds })}
      />

      <ActionsEditor
        actions={config.prompts}
        kinds={config.work_kinds}
        roles={allOf(config, 'version_roles')}
        onChange={(prompts) => patch({ prompts })}
      />

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

/**
 * What a release of each kind goes out as.
 *
 * Labels, hints, limits and templates — not keys: a key is what the value is
 * stored under on every release already planned, and renaming one would
 * orphan what was written under it, exactly as renaming an axis key would
 * orphan its scores. The same rule the rest of this screen follows.
 *
 * Adding and removing fields is deliberately not here either. A field is a
 * box on a screen and a key in a stored map, and the place to decide there
 * should be one more of those is the profile document, where the whole shape
 * is visible at once.
 */
function ReleaseFieldsEditor({
  kinds,
  onChange,
}: {
  kinds: ReleaseKind[]
  onChange: (kinds: ReleaseKind[]) => void
}) {
  const { t } = useTranslation()

  const set = (kindIndex: number, fieldIndex: number, changes: Partial<ReleaseField>) => {
    onChange(
      kinds.map((kind, i) =>
        i === kindIndex
          ? {
              ...kind,
              fields: (kind.fields ?? []).map((field, j) =>
                j === fieldIndex ? { ...field, ...changes } : field,
              ),
            }
          : kind,
      ),
    )
  }

  return (
    <section className="flex flex-col gap-3">
      <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
        {t('editor.releaseFields')}
      </h4>
      <p className="text-xs text-dim">{t('editor.releaseFieldsHint')}</p>

      {kinds.map((kind, kindIndex) =>
        (kind.fields ?? []).length === 0 ? null : (
          <div key={kind.key} className="flex flex-col gap-2 rounded-xl border border-line p-3">
            <div className="flex items-center gap-2">
              <code className="font-mono text-xs text-dim">{kind.key}</code>
              <span className="text-sm font-medium">{kind.label}</span>
            </div>

            <ul className="flex flex-col gap-3">
              {(kind.fields ?? []).map((field, fieldIndex) => (
                <li key={field.key} className="flex flex-col gap-2">
                  <div className="flex items-center gap-2">
                    <code className="shrink-0 font-mono text-xs text-dim">{field.key}</code>
                    <Input
                      className="flex-1"
                      value={field.label}
                      aria-label={t('editor.fieldLabel')}
                      onChange={(event) => set(kindIndex, fieldIndex, { label: event.target.value })}
                    />
                    <span className="shrink-0 text-2xs uppercase tracking-caption text-faint">
                      {t(`editor.fieldType.${field.type}`)}
                    </span>
                    <Input
                      className="w-24"
                      type="number"
                      min={1}
                      value={field.limit ?? ''}
                      placeholder={t('editor.fieldLimit')}
                      aria-label={t('editor.fieldLimit')}
                      onChange={(event) => {
                        const raw = event.target.value
                        set(kindIndex, fieldIndex, {
                          // Nothing typed is nobody counting, not a limit of
                          // zero — which the profile refuses to save anyway.
                          limit: raw === '' ? null : Math.max(1, Math.trunc(Number(raw))),
                        })
                      }}
                    />
                  </div>
                  <Input
                    value={field.hint ?? ''}
                    placeholder={t('editor.fieldHint')}
                    aria-label={t('editor.fieldHint')}
                    onChange={(event) =>
                      set(kindIndex, fieldIndex, {
                        hint: event.target.value === '' ? null : event.target.value,
                      })
                    }
                  />
                  <Textarea
                    value={field.template ?? ''}
                    placeholder={t('editor.fieldTemplate')}
                    aria-label={t('editor.fieldTemplate')}
                    autoResize
                    maxRows={6}
                    rows={2}
                    className="font-mono text-xs"
                    onChange={(event) =>
                      set(kindIndex, fieldIndex, {
                        template: event.target.value === '' ? null : event.target.value,
                      })
                    }
                  />
                </li>
              ))}
            </ul>
          </div>
        ),
      )}
    </section>
  )
}

/**
 * One kind's own vocabulary: its axes, tiers, statuses and release kinds.
 *
 * Axis keys are deliberately not editable here either — the same past-score
 * reasoning applies per kind now, not just per profile.
 */
function KindVocabulary({
  kind,
  onChange,
}: {
  kind: WorkKind
  onChange: (changes: Partial<WorkKind>) => void
}) {
  const { t } = useTranslation()

  const axes = kind.axes ?? []
  const tiers = kind.tiers ?? []

  const setAxis = (index: number, changes: Partial<Axis>) => {
    onChange({ axes: axes.map((axis, i) => (i === index ? { ...axis, ...changes } : axis)) })
  }

  const setTier = (index: number, changes: Partial<Tier>) => {
    onChange({ tiers: tiers.map((tier, i) => (i === index ? { ...tier, ...changes } : tier)) })
  }

  return (
    <section className="flex flex-col gap-4 border-t border-line pt-4">
      <h3 className="text-sm font-semibold">{kind.label}</h3>

      <section className="flex flex-col gap-2">
        <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
          {t('editor.axes')}
        </h4>
        <p className="text-xs text-dim">{t('editor.axesHint')}</p>
        <ul className="flex flex-col gap-1.5">
          {axes.map((axis, index) => (
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
                onClick={() => onChange({ axes: axes.filter((_, i) => i !== index) })}
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
          {tiers.map((tier, index) => (
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

      <Vocabulary
        label={t('editor.statuses')}
        entries={kind.statuses ?? []}
        onChange={(statuses) => onChange({ statuses })}
      />
      <Vocabulary
        label={t('editor.releaseKinds')}
        entries={kind.release_kinds ?? []}
        onChange={(release_kinds) => onChange({ release_kinds })}
      />
      {/* What a release of each kind says about itself. Under the release
          kinds because that is what it belongs to, and only for the kinds
          that have any: a profile that says nothing about its releases
          should show an empty screen, not an invitation. */}
      {(kind.release_kinds ?? []).some((entry) => (entry.fields ?? []).length > 0) && (
        <ReleaseFieldsEditor
          kinds={kind.release_kinds ?? []}
          onChange={(release_kinds) => onChange({ release_kinds })}
        />
      )}
      {/* The storyboard's words, for a kind that has any: a song lists none
          and shows nothing here. Keys, as everywhere on this screen, come
          from the document; the labels are what is renamed. */}
      {(kind.shot_types ?? []).length > 0 && (
        <Vocabulary
          label={t('editor.shotTypes')}
          entries={kind.shot_types ?? []}
          onChange={(shot_types) => onChange({ shot_types })}
        />
      )}
      {(kind.scene_blocks ?? []).length > 0 && (
        <Vocabulary
          label={t('editor.sceneBlocks')}
          entries={kind.scene_blocks ?? []}
          onChange={(scene_blocks) => onChange({ scene_blocks })}
        />
      )}
    </section>
  )
}

/**
 * The profile's actions, whole: the wording, the method and what each
 * produces — not only the label. A method is the craft's own way of doing
 * the action (ADR 0021), shipped with the profile and edited here; the
 * key stays fixed once made, because a running task and a chat are
 * recognised by it.
 */
function ActionsEditor({
  actions,
  kinds,
  roles,
  onChange,
}: {
  actions: PromptTemplate[]
  kinds: WorkKind[]
  roles: VersionRole[]
  onChange: (actions: PromptTemplate[]) => void
}) {
  const { t } = useTranslation()
  const [newKey, setNewKey] = useState('')

  const set = (index: number, changes: Partial<PromptTemplate>) => {
    onChange(actions.map((action, i) => (i === index ? { ...action, ...changes } : action)))
  }

  // Prose is the absence of a value, and Base UI items may not carry an
  // empty one: it is the select's placeholder, and picking it clears.
  const producesOptions = [
    { value: 'score', label: t('editor.producesScore') },
    ...roles.map((role) => ({
      value: `version:${role.key}`,
      label: t('editor.producesVersion', { role: role.label }),
    })),
    { value: 'scenes', label: t('editor.producesScenes') },
    { value: 'scenes:add', label: t('editor.producesScenesAdd') },
    { value: 'scenes:revise', label: t('editor.producesScenesRevise') },
  ]
  const scopeOptions = [{ value: 'scene', label: t('editor.scopeScene') }]

  // The kinds an action is for, as chips: none on means every kind.
  const toggleKind = (index: number, key: string) => {
    const current = actions[index]?.kinds ?? []
    const next = current.includes(key) ? current.filter((k) => k !== key) : [...current, key]
    set(index, { kinds: next.length === 0 ? undefined : next })
  }

  // A key typed as a slug: lower case, letters, digits and dashes, unique.
  const key = newKey.trim().toLowerCase()
  const keyTaken = actions.some((action) => action.key === key)
  const keyValid = /^[a-z][a-z0-9_-]*$/.test(key) && !keyTaken

  return (
    <section className="flex flex-col gap-3">
      <h4 className="text-xs font-medium uppercase tracking-wide text-dim">
        {t('editor.prompts')}
      </h4>
      <p className="text-xs text-dim">{t('editor.promptsHint')}</p>
      <ul className="flex flex-col gap-4">
        {actions.map((action, index) => (
          <li key={action.key} className="flex flex-col gap-2 rounded-xl border border-line p-3">
            <div className="flex items-center gap-2">
              <code className="shrink-0 font-mono text-xs text-dim">{action.key}</code>
              <Input
                className="flex-1"
                value={action.label}
                onChange={(event) => set(index, { label: event.target.value })}
                aria-label={t('editor.actionLabel')}
              />
              <Select
                className="w-56"
                aria-label={t('editor.actionProduces')}
                placeholder={t('editor.producesProse')}
                value={
                  action.produces !== undefined &&
                  producesOptions.some((option) => option.value === action.produces)
                    ? action.produces
                    : ''
                }
                onChange={(value) => set(index, { produces: value === '' ? undefined : value })}
                options={producesOptions}
              />
              <Button
                variant="danger"
                size="icon-sm"
                title={t('editor.removeAction')}
                aria-label={t('editor.removeAction')}
                onClick={() => onChange(actions.filter((_, i) => i !== index))}
              >
                <X aria-hidden className="size-3.5" />
              </Button>
            </div>
            <Input
              value={action.description ?? ''}
              placeholder={t('editor.actionDescription')}
              aria-label={t('editor.actionDescription')}
              onChange={(event) =>
                set(index, { description: event.target.value === '' ? undefined : event.target.value })
              }
            />
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-2xs font-semibold uppercase tracking-caption text-faint">
                {t('editor.actionKinds')}
              </span>
              <div role="group" aria-label={t('editor.actionKinds')} className="flex flex-wrap gap-1.5">
                {kinds.map((kind) => {
                  const on = (action.kinds ?? []).includes(kind.key)
                  return (
                    <button
                      key={kind.key}
                      type="button"
                      aria-pressed={on}
                      onClick={() => toggleKind(index, kind.key)}
                      className={
                        on
                          ? 'cursor-pointer rounded-full border border-transparent bg-accent-soft px-2.5 py-0.5 text-[11.5px] font-semibold text-accent-2'
                          : 'cursor-pointer rounded-full border border-line px-2.5 py-0.5 text-[11.5px] text-dim hover:border-line-2 hover:text-text'
                      }
                    >
                      {kind.label}
                    </button>
                  )
                })}
              </div>
              <span className="text-xs text-faint">
                {(action.kinds ?? []).length === 0 ? t('editor.actionKindsAll') : ''}
              </span>
              <span className="flex-1" />
              <Select
                className="w-44"
                aria-label={t('editor.actionScope')}
                placeholder={t('editor.scopeWork')}
                value={action.scope === 'scene' ? 'scene' : ''}
                onChange={(value) => set(index, { scope: value === '' ? undefined : value })}
                options={scopeOptions}
              />
            </div>
            <Field label={t('editor.actionTemplate')} hint={t('editor.actionTemplateHint')}>
              <Textarea
                autoResize
                maxRows={10}
                rows={3}
                className="font-mono text-xs"
                value={action.template}
                onChange={(event) => set(index, { template: event.target.value })}
              />
            </Field>
            <Field label={t('editor.actionMethod')} hint={t('editor.actionMethodHint')}>
              <Textarea
                autoResize
                maxRows={24}
                rows={4}
                className="font-mono text-xs"
                value={action.method ?? ''}
                placeholder={t('editor.actionMethodPlaceholder')}
                onChange={(event) =>
                  set(index, { method: event.target.value === '' ? undefined : event.target.value })
                }
              />
            </Field>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-2">
        <Input
          className="w-48 font-mono text-xs"
          value={newKey}
          placeholder={t('editor.actionKeyPlaceholder')}
          aria-label={t('editor.actionKey')}
          onChange={(event) => setNewKey(event.target.value)}
        />
        <Button
          size="sm"
          disabled={!keyValid}
          onClick={() => {
            onChange([...actions, { key, label: key, template: '' }])
            setNewKey('')
          }}
        >
          <Plus aria-hidden className="size-3.5" />
          {t('editor.addAction')}
        </Button>
        {key !== '' && keyTaken && (
          <span className="text-xs text-bad">{t('editor.actionKeyTaken')}</span>
        )}
      </div>
    </section>
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
