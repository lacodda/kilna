import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import type { PromptTemplate, WorkKind } from '@/lib/api/types'
import { scopeOf } from '@/lib/actions'
import { say as sayLabel } from '@/lib/useProfile'
import { Select, type Option } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'

/**
 * One action of the profile, whole: its button text and hint, what the answer
 * becomes, the kinds and the thing it is about, the message and the method.
 * The key is shown and stays - a running task and a chat are recognised by
 * it.
 */
export function ActionEditor({
  action,
  kinds,
  producesOptions,
  scopeOptions,
  onChange,
  onRemove,
}: {
  action: PromptTemplate
  kinds: WorkKind[]
  /** What an answer can become, with prose as the absence of a value. */
  producesOptions: Option[]
  /** What an action can be about, with the work as the absence of one. */
  scopeOptions: Option[]
  onChange: (changes: Partial<PromptTemplate>) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()

  return (
    <li className="flex flex-col gap-2 rounded-xl border border-line p-3">
      <div className="flex items-center gap-2">
        <code className="shrink-0 font-mono text-xs text-dim">{action.key}</code>
        <Input
          className="flex-1"
          value={sayLabel(action.label)}
          onChange={(event) => onChange({ label: event.target.value })}
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
          onChange={(value) => onChange({ produces: value === '' ? undefined : value })}
          options={producesOptions}
        />
        <Button
          variant="danger"
          size="icon-sm"
          title={t('editor.removeAction')}
          aria-label={t('editor.removeAction')}
          onClick={onRemove}
        >
          <X aria-hidden className="size-3.5" />
        </Button>
      </div>
      <Input
        value={sayLabel(action.description)}
        placeholder={t('editor.actionDescription')}
        aria-label={t('editor.actionDescription')}
        onChange={(event) =>
          onChange({ description: event.target.value === '' ? undefined : event.target.value })
        }
      />
      <div className="flex flex-wrap items-center gap-2">
        <span className="caption">{t('editor.actionKinds')}</span>
        {/* The kinds an action is for, as chips: none on means every kind. */}
        <ChipGroup
          multiple
          aria-label={t('editor.actionKinds')}
          value={action.kinds ?? []}
          onValueChange={(next) => onChange({ kinds: next.length === 0 ? undefined : next })}
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
          className="w-44"
          aria-label={t('editor.actionScope')}
          placeholder={t('editor.scopeWork')}
          value={scopeOf(action) === 'work' ? '' : scopeOf(action)}
          onChange={(value) => onChange({ scope: value === '' ? undefined : value })}
          options={scopeOptions}
        />
      </div>
      <Field label={t('editor.actionTemplate')} help={t('editor.actionTemplateHint')}>
        <Textarea
          autoResize
          maxRows={10}
          rows={3}
          className="font-mono text-xs"
          value={action.template}
          onChange={(event) => onChange({ template: event.target.value })}
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
            onChange({ method: event.target.value === '' ? undefined : event.target.value })
          }
        />
      </Field>
    </li>
  )
}
