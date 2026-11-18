import { useTranslation } from 'react-i18next'
import type { Kind, MetaField, WorkKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Select } from '@/components/AppSelect'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Switch } from '@/components/ui/switch'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

type FieldType = MetaField['type']

const TYPES: FieldType[] = [
  'text',
  'multiline',
  'number',
  'date',
  'boolean',
  'choice',
  'list',
  'arc',
]

/**
 * The fields a work carries beside its text (v0.112): each a key, a word and
 * a type, the kinds of work that have it, and for a choice its answers.
 *
 * Until now a field was added only by editing the profile document. A key
 * stands once made - works store their values under it - and a field taken
 * out leaves what works hold untouched, read again the day it comes back.
 */
export function MetaFieldsEditor({
  fields,
  kinds,
  onChange,
  problems,
}: {
  fields: MetaField[]
  kinds: WorkKind[]
  onChange: (fields: MetaField[]) => void
  problems?: (key: string) => readonly string[]
}) {
  const { t } = useTranslation()
  const types = TYPES.map((type) => ({ value: type, label: t(`editor.metaType.${type}`) }))

  return (
    <KeyedList
      boxed
      label={t('editor.metaFields')}
      help={t('editor.metaFieldsHint')}
      entries={fields}
      onChange={onChange}
      make={(key): MetaField => ({ key, label: key, type: 'text' })}
      addLabel={t('editor.addField')}
      problems={problems}
      below={(field, set) => (
        <div className="flex flex-col gap-2 pl-6">
          <div className="flex flex-wrap items-center gap-2">
            <span className="caption">{t('editor.actionKinds')}</span>
            {/* The kinds that have the field, as chips: none on means every
                kind, as with the kinds an action is for. */}
            <ChipGroup
              multiple
              aria-label={t('editor.fieldKinds', { key: field.key })}
              value={field.kinds ?? []}
              onValueChange={(next) => set({ kinds: next.length === 0 ? undefined : next })}
            >
              {kinds.map((kind) => (
                <Chip key={kind.key} value={kind.key}>
                  {sayLabel(kind.label)}
                </Chip>
              ))}
            </ChipGroup>
            <span className="text-xs text-faint">
              {(field.kinds ?? []).length === 0 ? t('editor.actionKindsAll') : ''}
            </span>
            <span className="flex-1" />
            <Switch checked={field.own ?? false} onCheckedChange={(own) => set({ own })}>
              {t('editor.fieldOwn')}
            </Switch>
            {/* The author's key (v0.146): every action keeps it as it is. */}
            {(field.type === 'list' || field.type === 'text' || field.type === 'multiline') && (
              <Switch
                checked={field.protected ?? false}
                onCheckedChange={(on) => set({ protected: on || undefined })}
              >
                {t('editor.fieldProtected')}
              </Switch>
            )}
          </div>
          {field.type === 'choice' && (
            <KeyedList<Kind>
              label={t('editor.fieldOptions')}
              help={t('editor.fieldOptionsHint')}
              entries={field.options ?? []}
              onChange={(options) => {
                // A start that names an answer taken out would refuse the
                // whole profile: it goes with its answer.
                const kept =
                  typeof field.default === 'string' &&
                  !options.some((option) => option.key === field.default)
                    ? { default: undefined }
                    : {}
                set({ options, ...kept })
              }}
              make={(key) => ({ key, label: key })}
            >
              {(option, setOption) => (
                <LabelInput
                  className="flex-1"
                  label={option.label}
                  onChange={(label) => setOption({ label: label ?? '' })}
                  aria-label={t('editor.wordOf', { key: option.key })}
                />
              )}
            </KeyedList>
          )}
        </div>
      )}
    >
      {(field, set) => (
        <>
          <LabelInput
            className="flex-1"
            label={field.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={t('editor.wordOf', { key: field.key })}
          />
          <Select
            className="w-44"
            aria-label={t('editor.metaTypeOf', { key: field.key })}
            value={field.type}
            onChange={(type) =>
              set({
                type: type as FieldType,
                // What only one type means goes with the type: a choice's
                // answers, a text's numbering, and a start of the old shape.
                options: type === 'choice' ? (field.options ?? []) : undefined,
                numbered_from: type === 'text' ? field.numbered_from : undefined,
                default: type === field.type ? field.default : undefined,
              })
            }
            options={types}
          />
        </>
      )}
    </KeyedList>
  )
}
