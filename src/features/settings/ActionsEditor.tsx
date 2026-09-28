import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import type { PromptTemplate, VersionRole, WorkKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { ActionEditor } from '@/features/settings/ActionEditor'

/**
 * The profile's actions, whole: the wording, the method and what each
 * produces — not only the label. A method is the craft's own way of doing
 * the action (ADR 0021), shipped with the profile and edited here; the
 * key stays fixed once made, because a running task and a chat are
 * recognised by it.
 */
export function ActionsEditor({
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
      // Said, not passed whole: a shipped role's label is `{ en, ru }`, and
      // handed to the sentence as it is it read "A version: [object Object]".
      label: t('editor.producesVersion', { role: sayLabel(role.label) }),
    })),
    { value: 'scenes', label: t('editor.producesScenes') },
    { value: 'scenes:add', label: t('editor.producesScenesAdd') },
    { value: 'scenes:revise', label: t('editor.producesScenesRevise') },
    { value: 'comment', label: t('editor.producesComment') },
    { value: 'reply', label: t('editor.producesReply') },
  ]
  // Every scope the backend reads; a work is the absence of one.
  const scopeOptions = [
    { value: 'scene', label: t('editor.scopeScene') },
    { value: 'style', label: t('editor.scopeStyle') },
    { value: 'comment', label: t('editor.scopeComment') },
  ]

  // A key typed as a slug: lower case, letters, digits and dashes, unique.
  const key = newKey.trim().toLowerCase()
  const keyTaken = actions.some((action) => action.key === key)
  const keyValid = /^[a-z][a-z0-9_-]*$/.test(key) && !keyTaken

  return (
    // The hint is under the whole group, where the key of a new action is
    // typed: "keys are fixed once made" is said beside the box that makes one.
    <FieldGroup label={t('editor.prompts')} help={t('editor.promptsHint')}>
      <ul className="flex flex-col gap-4">
        {actions.map((action, index) => (
          <ActionEditor
            key={action.key}
            action={action}
            kinds={kinds}
            producesOptions={producesOptions}
            scopeOptions={scopeOptions}
            onChange={(changes) => set(index, changes)}
            onRemove={() => onChange(actions.filter((_, i) => i !== index))}
          />
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
          <Plus aria-hidden />
          {t('editor.addAction')}
        </Button>
        {key !== '' && keyTaken && (
          <span className="text-xs text-bad">{t('editor.actionKeyTaken')}</span>
        )}
      </div>
    </FieldGroup>
  )
}
