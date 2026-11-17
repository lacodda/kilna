import { useTranslation } from 'react-i18next'
import type {
  Axis,
  Derive,
  Kind,
  MarkColour,
  ReleaseKind,
  Status,
  VersionRole,
} from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { Select, type Option } from '@/components/AppSelect'
import { NumberField } from '@/components/ui/number-field'
import { Switch } from '@/components/ui/switch'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

/*
 * The lists of a kind's words that are a key, a word and a little more: its
 * statuses, its version roles, its kinds of release, and the plain lists of
 * the storyboard. Each is the same list - dragged into order, added to under
 * a key, taken from - with its own few fields beside the word (v0.112).
 */

type Problems = (key: string) => readonly string[]

const COLOURS: MarkColour[] = ['plain', 'accent', 'good', 'warn', 'bad', 'info']
const MEANINGS: Derive[] = ['manual', 'draft', 'scored', 'scheduled', 'released']

/** The palette's roles, each by its name in the window's language. */
export function useColourOptions(): Option[] {
  const { t } = useTranslation()
  return COLOURS.map((colour) => ({ value: colour, label: t(`editor.colour.${colour}`) }))
}

/**
 * A kind's statuses, in the order a work moves through them. Beside the word:
 * what the status means to the automation - the fact that puts a work in it -
 * and the palette role its badge takes.
 */
export function StatusesEditor({
  statuses,
  onChange,
  problems,
}: {
  statuses: Status[]
  onChange: (statuses: Status[]) => void
  problems?: Problems
}) {
  const { t } = useTranslation()
  const colours = useColourOptions()
  const meanings = MEANINGS.map((meaning) => ({
    value: meaning,
    label: t(`editor.derive.${meaning}`),
  }))

  return (
    <KeyedList
      label={t('editor.statuses')}
      help={t('editor.statusesHint')}
      entries={statuses}
      onChange={onChange}
      make={(key): Status => ({ key, label: key, derive: 'manual' })}
      addLabel={t('editor.addStatus')}
      problems={problems}
    >
      {(status, set) => (
        <>
          <LabelInput
            className="flex-1"
            label={status.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={status.key}
          />
          <Select
            className="w-48"
            aria-label={t('editor.deriveOf', { key: status.key })}
            value={status.derive}
            onChange={(derive) => set({ derive: derive as Derive })}
            options={meanings}
          />
          <Select
            className="w-32"
            aria-label={t('editor.colourOf', { key: status.key })}
            placeholder={t('editor.colour.none')}
            value={status.colour ?? ''}
            onChange={(colour) => set({ colour: colour === '' ? null : (colour as MarkColour) })}
            options={colours}
          />
        </>
      )}
    </KeyedList>
  )
}

/**
 * A kind's version roles: the independent bodies a work of the kind carries.
 * Beside the word: how a body reads, and the role it comments on, if it is
 * written about another; under it, whether it counts as a time the work was
 * written and whether it is sung.
 */
export function RolesEditor({
  roles,
  onChange,
  problems,
}: {
  roles: VersionRole[]
  onChange: (roles: VersionRole[]) => void
  problems?: Problems
}) {
  const { t } = useTranslation()
  const bodies = [
    { value: 'plain', label: t('editor.body.plain') },
    { value: 'markdown', label: t('editor.body.markdown') },
  ]
  const counts = [
    { value: 'yes', label: t('editor.counts.yes') },
    { value: 'no', label: t('editor.counts.no') },
  ]

  return (
    <KeyedList
      label={t('editor.roles')}
      help={t('editor.rolesHint')}
      entries={roles}
      onChange={onChange}
      make={(key): VersionRole => ({ key, label: key })}
      addLabel={t('editor.addRole')}
      problems={problems}
      below={(role, set) => (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 pl-6">
          <Select
            className="w-56"
            aria-label={t('editor.countsOf', { key: role.key })}
            placeholder={t('editor.counts.auto')}
            value={
              role.counts_as_version === true ? 'yes' : role.counts_as_version === false ? 'no' : ''
            }
            onChange={(value) => set({ counts_as_version: value === '' ? null : value === 'yes' })}
            options={counts}
          />
          <Switch checked={role.sung ?? false} onCheckedChange={(sung) => set({ sung })}>
            {t('editor.sung')}
          </Switch>
        </div>
      )}
    >
      {(role, set) => (
        <>
          <LabelInput
            className="flex-1"
            label={role.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={role.key}
          />
          <Select
            className="w-36"
            aria-label={t('editor.bodyOf', { key: role.key })}
            value={role.body === 'markdown' ? 'markdown' : 'plain'}
            onChange={(body) => set({ body: body === 'markdown' ? 'markdown' : null })}
            options={bodies}
          />
          <Select
            className="w-44"
            aria-label={t('editor.commentsOnOf', { key: role.key })}
            placeholder={t('editor.commentsOnNone')}
            value={role.comments_on ?? ''}
            onChange={(target) => set({ comments_on: target === '' ? null : target })}
            options={roles
              .filter((other) => other.key !== role.key)
              .map((other) => ({
                value: other.key,
                label: t('editor.commentsOn', { role: sayLabel(other.label) }),
              }))}
          />
        </>
      )}
    </KeyedList>
  )
}

/**
 * A kind's kinds of release: the doors its works go out through. Under each,
 * how it weighs the kind's axes - a second opinion of the same score: a short
 * lives on its hook where an album track lives on its craft. An axis left
 * empty is weighed as the kind weighs it.
 */
export function ReleaseKindsEditor({
  kinds,
  axes,
  onChange,
  problems,
}: {
  kinds: ReleaseKind[]
  axes: Axis[]
  onChange: (kinds: ReleaseKind[]) => void
  problems?: Problems
}) {
  const { t } = useTranslation()

  return (
    <KeyedList
      label={t('editor.releaseKinds')}
      help={axes.length > 0 ? t('editor.releaseKindsHint') : undefined}
      entries={kinds}
      onChange={onChange}
      make={(key): ReleaseKind => ({ key, label: key, requires: [] })}
      addLabel={t('editor.addReleaseKind')}
      problems={problems}
      below={(kind, set) =>
        axes.length === 0 ? null : (
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 pl-6">
            <span className="caption">{t('editor.weights')}</span>
            {axes.map((axis) => (
              <label key={axis.key} className="flex items-center gap-1.5 text-xs text-dim">
                {sayLabel(axis.label)}
                <NumberField
                  className="w-20"
                  hideStepper
                  min={0}
                  step={0.5}
                  value={kind.axis_weights?.[axis.key] ?? null}
                  placeholder={String(axis.weight)}
                  onValueChange={(weight) => {
                    const rest = Object.fromEntries(
                      Object.entries(kind.axis_weights ?? {}).filter(([key]) => key !== axis.key),
                    )
                    const next = weight === null ? rest : { ...rest, [axis.key]: weight }
                    set({ axis_weights: Object.keys(next).length === 0 ? undefined : next })
                  }}
                  aria-label={t('editor.weightOf', {
                    axis: sayLabel(axis.label),
                    door: sayLabel(kind.label),
                  })}
                />
              </label>
            ))}
          </div>
        )
      }
    >
      {(kind, set) => (
        <LabelInput
          className="flex-1"
          label={kind.label}
          onChange={(label) => set({ label: label ?? '' })}
          aria-label={kind.key}
        />
      )}
    </KeyedList>
  )
}

/**
 * A plain list of words: a key and what it is called. The kinds of shot and
 * the layers of a storyboard, the kinds of collection.
 */
export function WordsEditor<T extends Kind>({
  label,
  help,
  entries,
  onChange,
  make,
  addLabel,
  problems,
}: {
  label: string
  help?: string
  entries: T[]
  onChange: (entries: T[]) => void
  /** Absent, the list is renamed and reordered, and neither grown nor cut. */
  make?: (key: string) => T
  addLabel?: string
  problems?: Problems
}) {
  return (
    <KeyedList
      label={label}
      help={help}
      entries={entries}
      onChange={onChange}
      make={make}
      addLabel={addLabel}
      removable={make !== undefined}
      problems={problems}
    >
      {(entry, set) => (
        <LabelInput
          className="flex-1"
          label={entry.label}
          // `Partial<T>` of a generic entry: the word is every entry's.
          onChange={(word) => set({ label: word ?? '' } as Partial<T>)}
          aria-label={entry.key}
        />
      )}
    </KeyedList>
  )
}
