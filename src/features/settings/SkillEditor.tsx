import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Board, PromptTemplate, SkillPreview, VersionRole, WorkKind } from '@/lib/api/types'
import { previewSkill } from '@/lib/api/workspace'
import { ACTION_ICON_NAMES } from '@/lib/actionIcon'
import { scopeOf } from '@/lib/actions'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { CATEGORIES } from '@/lib/skills'
import { say as sayLabel } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field } from '@/components/ui/field'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { LabelInput } from '@/features/settings/LabelInput'

/**
 * One skill, whole (FR-17): its name, when to reach for it and its group,
 * the kinds and the thing it is about, what it answers with, whether the
 * window offers it as a button and the message that button sends, and the
 * method - the craft's way of doing it, which reaches the model as its
 * instruction (ADR 0021). Every scope and every answer the backend reads is
 * offered; picking one never quietly makes it a different skill.
 *
 * What the assistant would be handed is one press away: the method and the
 * shape of the answer said with the profile's live vocabulary - a score
 * names every axis with its rubric.
 */
export function SkillEditor({
  action,
  kinds,
  roles,
  boards,
  set,
  problems,
}: {
  action: PromptTemplate
  kinds: WorkKind[]
  roles: VersionRole[]
  boards: Board[]
  set: (changes: Partial<PromptTemplate>) => void
  problems: readonly string[]
}) {
  const { t } = useTranslation()
  const shown = action.show_in_ui !== false

  // Prose is the absence of a value, and Base UI items may not carry an
  // empty one: it is the select's placeholder, and picking it clears.
  const producesOptions = [
    { value: 'score', label: t('editor.producesScore') },
    ...roles.map((role) => ({
      value: `version:${role.key}`,
      label: t('editor.producesVersion', { role: sayLabel(role.label) }),
    })),
    { value: 'scenes', label: t('editor.producesScenes') },
    { value: 'scenes:add', label: t('editor.producesScenesAdd') },
    { value: 'scenes:revise', label: t('editor.producesScenesRevise') },
    { value: 'comment', label: t('editor.producesComment') },
    { value: 'reply', label: t('editor.producesReply') },
    { value: 'description', label: t('editor.producesDescription') },
    { value: 'canon', label: t('editor.producesCanon') },
    { value: 'card-prompt', label: t('editor.producesCardPrompt') },
    { value: 'card-note', label: t('editor.producesCardNote') },
    { value: 'release', label: t('editor.producesRelease') },
    { value: 'cover-ideas', label: t('editor.producesCoverIdeas') },
    { value: 'words', label: t('editor.producesWords') },
    { value: 'bricks', label: t('editor.producesBricks') },
    { value: 'trials', label: t('editor.producesTrials') },
    ...boards.map((board) => ({
      value: `cards:${board.key}`,
      label: t('editor.producesCards', { board: sayLabel(board.label) }),
    })),
    { value: 'conception', label: t('editor.producesConception') },
  ]
  // Every scope the backend reads; a work is the absence of one.
  const scopeOptions = [
    { value: 'scene', label: t('editor.scopeScene') },
    { value: 'selection', label: t('editor.scopeSelection') },
    { value: 'style', label: t('editor.scopeStyle') },
    { value: 'comment', label: t('editor.scopeComment') },
    { value: 'canon', label: t('editor.scopeCanon') },
    { value: 'release', label: t('editor.scopeRelease') },
    { value: 'cover', label: t('editor.scopeCover') },
    { value: 'phrases', label: t('editor.scopePhrases') },
    { value: 'lab', label: t('editor.scopeLab') },
    { value: 'board', label: t('editor.scopeBoard') },
  ]
  const categoryOptions = CATEGORIES.map((category) => ({
    value: category,
    label: t(`skills.category.${category}`),
  }))
  const iconOptions = ACTION_ICON_NAMES.map((name) => ({ value: name, label: name }))

  // The kind the preview reads its axes and roles from: the first the skill
  // is for, or the first the profile has.
  const kindKeys = (action.kinds ?? []).length > 0 ? action.kinds! : kinds.map((kind) => kind.key)
  const [kind, setKind] = useState(kindKeys[0] ?? '')
  const previewKind = kindKeys.includes(kind) ? kind : (kindKeys[0] ?? '')
  const [preview, setPreview] = useState<SkillPreview | null>(null)
  const read = useAppMutation({
    mutationFn: () => previewSkill(previewKind, action),
    onSuccess: setPreview,
    failure: 'toast.skillPreviewFailed',
  })

  return (
    <div className="flex flex-col gap-3">
      {problems.length > 0 && (
        <ul role="alert" className="flex flex-col gap-1 text-xs text-bad">
          {problems.map((problem, index) => (
            <li key={index}>{problem}</li>
          ))}
        </ul>
      )}
      <div className="flex flex-col gap-2">
        <LabelInput
          label={action.label}
          onChange={(label) => set({ label: label ?? '' })}
          aria-label={t('editor.actionLabel')}
        />
        <LabelInput
          optional
          label={action.description}
          placeholder={t('editor.actionDescription')}
          aria-label={t('editor.actionDescription')}
          onChange={(description) => set({ description })}
        />
        <LabelInput
          optional
          label={action.when}
          placeholder={t('skills.when')}
          aria-label={t('skills.when')}
          onChange={(when) => set({ when })}
        />
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <span className="caption">{t('editor.actionKinds')}</span>
        {/* The kinds a skill is for, as chips: none on means every kind. */}
        <ChipGroup
          multiple
          aria-label={t('editor.actionKinds')}
          value={action.kinds ?? []}
          onValueChange={(next) => set({ kinds: next.length === 0 ? undefined : next })}
        >
          {kinds.map((one) => (
            <Chip key={one.key} value={one.key}>
              {sayLabel(one.label)}
            </Chip>
          ))}
        </ChipGroup>
        <span className="text-xs text-faint">
          {(action.kinds ?? []).length === 0 ? t('editor.actionKindsAll') : ''}
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Select
          className="w-48"
          aria-label={t('skills.category.label')}
          placeholder={t('skills.category.none')}
          value={action.category ?? ''}
          onChange={(category) => set({ category: category === '' ? undefined : category })}
          options={categoryOptions}
        />
        <Select
          className="w-52"
          aria-label={t('editor.actionScope')}
          placeholder={t('editor.scopeWork')}
          value={scopeOf(action) === 'work' ? '' : scopeOf(action)}
          onChange={(value) => set({ scope: value === '' ? undefined : value })}
          options={scopeOptions}
        />
        <Select
          className="w-56"
          aria-label={t('editor.actionProduces')}
          placeholder={t('editor.producesProse')}
          value={
            action.produces != null &&
            producesOptions.some((option) => option.value === action.produces)
              ? action.produces
              : ''
          }
          onChange={(value) => set({ produces: value === '' ? undefined : value })}
          options={producesOptions}
        />
      </div>

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
        <Switch
          checked={shown}
          onCheckedChange={(next) => set({ show_in_ui: next ? undefined : false })}
        >
          {t('skills.showInUi')}
        </Switch>
        <Select
          className="w-44"
          aria-label={t('editor.actionIcon')}
          placeholder={t('editor.actionIconNone')}
          value={action.icon != null && ACTION_ICON_NAMES.includes(action.icon) ? action.icon : ''}
          onChange={(icon) => set({ icon: icon === '' ? undefined : icon })}
          options={iconOptions}
        />
        <Switch checked={action.web ?? false} onCheckedChange={(web) => set({ web })}>
          {t('editor.actionWeb')}
        </Switch>
      </div>

      <Field
        label={t('editor.actionTemplate')}
        help={shown ? t('editor.actionTemplateHint') : t('skills.templateHidden')}
      >
        <Textarea
          autoResize
          maxRows={10}
          rows={3}
          className="font-mono text-xs"
          value={action.template}
          onChange={(event) => set({ template: event.target.value })}
        />
      </Field>
      <Field label={t('editor.actionMethod')} help={t('skills.methodHint')}>
        <Textarea
          autoResize
          maxRows={30}
          rows={6}
          className="font-mono text-xs"
          value={action.method ?? ''}
          placeholder={t('editor.actionMethodPlaceholder')}
          onChange={(event) =>
            set({ method: event.target.value === '' ? undefined : event.target.value })
          }
        />
      </Field>

      <div className="flex flex-wrap items-center gap-2">
        {kindKeys.length > 1 && (
          <Select
            className="w-44"
            aria-label={t('skills.previewKind')}
            value={previewKind}
            onChange={setKind}
            options={kindKeys.map((key) => ({
              value: key,
              label: sayLabel(kinds.find((one) => one.key === key)?.label ?? key),
            }))}
          />
        )}
        <Button size="sm" onClick={() => read.mutate()} disabled={read.isPending}>
          {t('skills.preview')}
        </Button>
      </div>
      {preview !== null && (
        <section
          aria-label={t('skills.previewTitle')}
          className="flex flex-col gap-2 rounded-md border border-line bg-bg p-3 text-xs"
        >
          <PreviewPart title={t('skills.previewMethod')} text={preview.method} />
          <PreviewPart title={t('skills.previewAnswer')} text={preview.answer} />
          <p className="text-dim">
            {preview.placeholders.length === 0
              ? t('skills.previewReadsNothing')
              : t('skills.previewReads', {
                  placeholders: preview.placeholders.map((name) => `{${name}}`).join(', '),
                })}
          </p>
        </section>
      )}
    </div>
  )
}

function PreviewPart({ title, text }: { title: string; text: string | null }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1">
      <h4 className="caption">{title}</h4>
      {text === null ? (
        <p className="text-faint">{t('skills.previewNone')}</p>
      ) : (
        <pre className="selectable max-h-72 overflow-y-auto font-mono whitespace-pre-wrap text-dim">
          {text}
        </pre>
      )}
    </div>
  )
}
