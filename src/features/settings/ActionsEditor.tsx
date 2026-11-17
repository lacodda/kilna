import { useTranslation } from 'react-i18next'
import type { Board, PromptTemplate, VersionRole, WorkKind } from '@/lib/api/types'
import { ACTION_ICON_NAMES } from '@/lib/actionIcon'
import { scopeOf } from '@/lib/actions'
import { say as sayLabel } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field } from '@/components/ui/field'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

/**
 * The profile's actions, whole: the wording, the method and what each
 * produces — not only the label. A method is the craft's own way of doing
 * the action (ADR 0021), shipped with the profile and edited here; the key
 * stays fixed once made, because a running task and a chat are recognised by
 * it.
 *
 * Since v0.112 every answer an action can become and every thing it can be
 * about is offered - an action about a release or a board could be read here
 * and not set, and picking anything in its box quietly made it a different
 * action - and what is wrong with a message is said under it as it is typed:
 * a placeholder nothing fills, a role its kinds do not have. That is the
 * guard the save always ran, moved to where the slip is made.
 */
export function ActionsEditor({
  actions,
  kinds,
  roles,
  boards,
  onChange,
  problems,
}: {
  actions: PromptTemplate[]
  kinds: WorkKind[]
  roles: VersionRole[]
  boards: Board[]
  onChange: (actions: PromptTemplate[]) => void
  /** What is wrong with an action, by its key. */
  problems?: (key: string) => readonly string[]
}) {
  const { t } = useTranslation()

  // Prose is the absence of a value, and Base UI items may not carry an
  // empty one: it is the select's placeholder, and picking it clears.
  const producesOptions = [
    { value: 'score', label: t('editor.producesScore') },
    ...roles.map((role) => ({
      value: `version:${role.key}`,
      // Said, not passed whole: a shipped role's label is `{ en, ru }`, and
      // handed to the sentence as it is it read "A version: [object Object]".
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
  const iconOptions = ACTION_ICON_NAMES.map((name) => ({ value: name, label: name }))

  return (
    // The hint is under the whole group, where the key of a new action is
    // typed: "keys are fixed once made" is said beside the box that makes one.
    <KeyedList<PromptTemplate>
      boxed
      label={t('editor.prompts')}
      help={t('editor.promptsHint')}
      entries={actions}
      onChange={onChange}
      make={(key) => ({ key, label: key, template: '' })}
      addLabel={t('editor.addAction')}
      removeLabel={() => t('editor.removeAction')}
      problems={problems}
      below={(action, set) => (
        <div className="flex flex-col gap-2">
          <LabelInput
            optional
            label={action.description}
            placeholder={t('editor.actionDescription')}
            aria-label={t('editor.actionDescription')}
            onChange={(description) => set({ description })}
          />
          <div className="flex flex-wrap items-center gap-2">
            <span className="caption">{t('editor.actionKinds')}</span>
            {/* The kinds an action is for, as chips: none on means every kind. */}
            <ChipGroup
              multiple
              aria-label={t('editor.actionKinds')}
              value={action.kinds ?? []}
              onValueChange={(next) => set({ kinds: next.length === 0 ? undefined : next })}
            >
              {kinds.map((kind) => (
                <Chip key={kind.key} value={kind.key}>
                  {sayLabel(kind.label)}
                </Chip>
              ))}
            </ChipGroup>
            <span className="text-xs text-faint">
              {(action.kinds ?? []).length === 0 ? t('editor.actionKindsAll') : ''}
            </span>
            <span className="flex-1" />
            <Select
              className="w-52"
              aria-label={t('editor.actionScope')}
              placeholder={t('editor.scopeWork')}
              value={scopeOf(action) === 'work' ? '' : scopeOf(action)}
              onChange={(value) => set({ scope: value === '' ? undefined : value })}
              options={scopeOptions}
            />
          </div>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
            <Select
              className="w-44"
              aria-label={t('editor.actionIcon')}
              placeholder={t('editor.actionIconNone')}
              value={
                action.icon != null && ACTION_ICON_NAMES.includes(action.icon) ? action.icon : ''
              }
              onChange={(icon) => set({ icon: icon === '' ? undefined : icon })}
              options={iconOptions}
            />
            <Switch checked={action.web ?? false} onCheckedChange={(web) => set({ web })}>
              {t('editor.actionWeb')}
            </Switch>
          </div>
          <Field label={t('editor.actionTemplate')} help={t('editor.actionTemplateHint')}>
            <Textarea
              autoResize
              maxRows={10}
              rows={3}
              className="font-mono text-xs"
              value={action.template}
              onChange={(event) => set({ template: event.target.value })}
            />
          </Field>
          <Field label={t('editor.actionMethod')} help={t('editor.actionMethodHint')}>
            <Textarea
              autoResize
              maxRows={24}
              rows={4}
              className="font-mono text-xs"
              value={action.method ?? ''}
              placeholder={t('editor.actionMethodPlaceholder')}
              onChange={(event) =>
                set({ method: event.target.value === '' ? undefined : event.target.value })
              }
            />
          </Field>
        </div>
      )}
    >
      {(action, set) => (
        <>
          <LabelInput
            className="flex-1"
            label={action.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={t('editor.actionLabel')}
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
        </>
      )}
    </KeyedList>
  )
}
