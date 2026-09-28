import { useTranslation } from 'react-i18next'
import type { ReleaseField, ReleaseKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'
import { Textarea } from '@/components/ui/textarea'

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
export function ReleaseFieldsEditor({
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
    <FieldGroup label={t('editor.releaseFields')} help={t('editor.releaseFieldsHint')}>
      <div className="flex flex-col gap-3">
        {kinds.map((kind, kindIndex) =>
          (kind.fields ?? []).length === 0 ? null : (
            <div key={kind.key} className="flex flex-col gap-2 rounded-xl border border-line p-3">
              <div className="flex items-center gap-2">
                <code className="font-mono text-xs text-dim">{kind.key}</code>
                <span className="text-sm font-medium">{sayLabel(kind.label)}</span>
              </div>

              <ul className="flex flex-col gap-3">
                {(kind.fields ?? []).map((field, fieldIndex) => (
                  <li key={field.key} className="flex flex-col gap-2">
                    <div className="flex items-center gap-2">
                      <code className="shrink-0 font-mono text-xs text-dim">{field.key}</code>
                      <Input
                        className="flex-1"
                        value={sayLabel(field.label)}
                        aria-label={t('editor.fieldLabel')}
                        onChange={(event) =>
                          set(kindIndex, fieldIndex, { label: event.target.value })
                        }
                      />
                      <span className="shrink-0 caption">
                        {t(`editor.fieldType.${field.type}`)}
                      </span>
                      {/* No stepper: a limit is hundreds of characters, and
                          nobody clicks their way there. */}
                      <NumberField
                        className="w-24"
                        hideStepper
                        min={1}
                        step={1}
                        value={field.limit ?? null}
                        placeholder={t('editor.fieldLimit')}
                        aria-label={t('editor.fieldLimit')}
                        onValueChange={(limit) => {
                          set(kindIndex, fieldIndex, {
                            // Nothing typed is nobody counting, not a limit of
                            // zero — which the profile refuses to save anyway.
                            limit: limit === null ? null : Math.max(1, Math.trunc(limit)),
                          })
                        }}
                      />
                    </div>
                    <Field label={t('editor.fieldHintLabel')}>
                      <Input
                        value={sayLabel(field.hint)}
                        placeholder={t('editor.fieldHint')}
                        aria-label={t('editor.fieldHintLabel')}
                        onChange={(event) =>
                          set(kindIndex, fieldIndex, {
                            hint: event.target.value === '' ? null : event.target.value,
                          })
                        }
                      />
                    </Field>
                    {/* Labelled, as the actions editor labels its own template:
                        three unlabelled boxes stacked under a field are three
                        boxes nobody can tell apart once they are empty. */}
                    <Field label={t('editor.fieldTemplateLabel')}>
                      <Textarea
                        value={field.template ?? ''}
                        placeholder={t('editor.fieldTemplate')}
                        aria-label={t('editor.fieldTemplateLabel')}
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
                    </Field>
                  </li>
                ))}
              </ul>
            </div>
          ),
        )}
      </div>
    </FieldGroup>
  )
}
