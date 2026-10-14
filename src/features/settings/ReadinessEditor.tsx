import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { ReleaseKind, WorkKind } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Checkbox } from '@/components/ui/checkbox'
import { FieldGroup } from '@/components/ui/field'

/**
 * What a release through each of a kind's doors cannot go out without
 * (v0.97): the versions it needs, a cover, the fields written. One place to
 * read and tick the whole checklist, though each requirement is kept on
 * what it requires - a role in the door's `requires`, the cover on the
 * door, a field on the field (ADR 0063) - so a field taken out of the
 * profile takes its requirement with it.
 *
 * Unticking writes `false` rather than dropping the word: a requirement the
 * shipped profile has and the owner took off must stay off when the next
 * start carries the shipped profile forward.
 *
 * The score is not a box: it is asked for whenever the kind names axes, and
 * not asked for when it names none.
 */
export function ReadinessEditor({
  kind,
  onChange,
}: {
  kind: WorkKind
  onChange: (changes: Partial<WorkKind>) => void
}) {
  const { t } = useTranslation()
  const doors = kind.release_kinds ?? []
  const roles = kind.version_roles ?? []

  const setDoor = (key: string, change: (door: ReleaseKind) => ReleaseKind) =>
    onChange({
      release_kinds: doors.map((door) => (door.key === key ? change(door) : door)),
    })

  return (
    <FieldGroup
      label={t('editor.readiness')}
      help={(kind.axes ?? []).length > 0 ? t('editor.readinessScored') : t('editor.readinessHint')}
    >
      <div className="flex flex-col gap-3">
        {doors.map((door) => {
          const requires = door.requires ?? []
          const fields = door.fields ?? []
          return (
            <div
              key={door.key}
              role="group"
              aria-label={t('editor.readinessOf', { door: sayLabel(door.label) })}
              className="flex flex-col gap-2 rounded-xl border border-line p-3"
            >
              <div className="flex items-center gap-2">
                <code className="font-mono text-xs text-dim">{door.key}</code>
                <span className="text-sm font-medium">{sayLabel(door.label)}</span>
              </div>

              {roles.length > 0 && (
                <Line caption={t('editor.readinessRoles')}>
                  {roles.map((role) => (
                    <Checkbox
                      key={role.key}
                      checked={requires.includes(role.key)}
                      onCheckedChange={(on) =>
                        setDoor(door.key, (entry) => ({
                          ...entry,
                          requires: on
                            ? [...requires, role.key]
                            : requires.filter((key) => key !== role.key),
                        }))
                      }
                    >
                      {sayLabel(role.label)}
                    </Checkbox>
                  ))}
                </Line>
              )}

              <Line caption={t('editor.readinessPicture')}>
                <Checkbox
                  checked={door.requires_cover === true}
                  onCheckedChange={(on) =>
                    setDoor(door.key, (entry) => ({ ...entry, requires_cover: on }))
                  }
                >
                  {t('readiness.cover')}
                </Checkbox>
              </Line>

              {fields.length > 0 && (
                <Line caption={t('editor.readinessFields')}>
                  {fields.map((field) => (
                    <Checkbox
                      key={field.key}
                      checked={field.required === true}
                      onCheckedChange={(on) =>
                        setDoor(door.key, (entry) => ({
                          ...entry,
                          fields: (entry.fields ?? []).map((one) =>
                            one.key === field.key ? { ...one, required: on } : one,
                          ),
                        }))
                      }
                    >
                      {sayLabel(field.label)}
                    </Checkbox>
                  ))}
                </Line>
              )}
            </div>
          )
        })}
      </div>
    </FieldGroup>
  )
}

/** One row of the checklist: what it is about, and its boxes. */
function Line({ caption, children }: { caption: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
      <span className="w-20 shrink-0 caption">{caption}</span>
      {children}
    </div>
  )
}
