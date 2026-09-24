import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  unpinStatus,
  updateWork,
  type Meta,
  type MetaField,
  type Work,
  type WorkPatch,
} from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { useFieldDraft } from '@/lib/fieldDraft'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { DatePicker } from '@/components/ui/DatePicker'
import { Field } from '@/components/ui/Field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Panel } from '@/components/ui/panel'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  work: Work
}

/**
 * What the work is: its title, where it stands, and the profile's own fields.
 *
 * These used to sit above the panels, in a header that grew a row every time the
 * profile gained a field. They are a tab now, and the header above shows the
 * same values read-only — you look at the header, you edit here.
 */
export function OverviewTab({ work }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const vocabulary = vocabularyOf(profile.config, work.kind)
  const client = useQueryClient()

  const patch = useMutation({
    mutationFn: (changes: WorkPatch) => updateWork(work.id, changes),
    // Optimistic: the header above should not lag behind the field just left.
    onMutate: async (changes) => {
      await client.cancelQueries({ queryKey: keys.work(work.id) })
      const previous = client.getQueryData<Work | null>(keys.work(work.id))

      if (previous != null) {
        client.setQueryData<Work>(keys.work(work.id), { ...previous, ...changes })
      }
      return { previous }
    },
    onError: (cause, _changes, context) => {
      // Put back what was there; the toast explains why it moved.
      // The title box follows the stored title back on its own.
      if (context?.previous !== undefined) {
        client.setQueryData(keys.work(work.id), context.previous)
      }
      say.failedTo(t('toast.workSaveFailed'), cause)
    },
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.workEdited'),
        refresh: [keys.works, keys.catalogue, keys.journal],
      })
    },
  })

  const saveStatus = useSaveStatus(patch.isPending, patch.isError)

  // The same rules as every other field here (`fieldDraft`): written when it
  // changed, Escape puts it back. A title cannot be emptied, so an empty box
  // goes back to the stored title rather than stay showing one never saved.
  const title = useFieldDraft(work.title, (text) => {
    const next = text.trim()
    if (next === '' || next === work.title) return false
    patch.mutate({ title: next })
  })

  // Whose status this is. Picking one from the list pins it — the automation
  // then steps over this work entirely — and the only way back is to say so.
  const pinned = work.status_pinned_at != null
  const statusHint = pinned ? t('work.statusPinned') : t('work.statusDerived')

  const unpin = useMutation({
    mutationFn: () => unpinStatus(work.id),
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.journal })
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  const setMeta = (key: string, value: Meta[string]) => {
    const meta: Meta = { ...work.meta }
    if (value === '' || value === undefined) delete meta[key]
    else meta[key] = value
    patch.mutate({ meta })
  }

  return (
    <div className="flex flex-col gap-4">
      {/* A grid rather than a wrapping row, for the reason measured below: at
          582px the three fields need 672 and Kind drops to a line of its own,
          leaving a hole beside the status. Aligned to the top because the
          status carries a line of hint under it — bottom alignment pushed that
          one field up while the others stayed, which read as broken rather than
          annotated. */}
      <Panel className="grid grid-cols-[repeat(auto-fit,minmax(180px,1fr))] items-start gap-4 p-4">
        <Field label={t('work.title')}>
          <Input className="w-full" {...title} />
        </Field>

        {/* The notice sits under the select rather than beside it: sharing the
            row left "Released" showing as "Relea…", and the status is what is
            being read here. It stays outside the `Field`, so the field holds
            the one control its caption names. */}
        <div className="flex flex-col">
          <Field label={t('work.status')} hint={pinned ? undefined : statusHint}>
            <Select
              className="w-full"
              value={work.status}
              onChange={(status) => patch.mutate({ status })}
              options={vocabulary.statuses.map((s) => ({ value: s.key, label: sayLabel(s.label) }))}
            />
          </Field>

          {pinned && (
            <p className="mt-1 text-xs text-faint">
              {t('work.statusPinned')}{' '}
              <button
                type="button"
                onClick={() => unpin.mutate()}
                disabled={unpin.isPending}
                title={t('work.unpinStatusHint')}
                className="cursor-pointer text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text disabled:opacity-50"
              >
                {t('work.unpinStatus')}
              </button>
            </p>
          )}
        </div>

        <Field label={t('work.kind')}>
          <Select
            className="w-full"
            value={work.kind}
            onChange={(kind) => patch.mutate({ kind })}
            options={profile.config.work_kinds.map((k) => ({ value: k.key, label: sayLabel(k.label) }))}
          />
        </Field>

        {/* Spanning the row rather than taking a column of its own: it is a
            word that appears for a second while saving, and a whole grid track
            reserved for it would narrow the fields permanently. */}
        <SaveState status={saveStatus} className="col-span-full" />
      </Panel>

      {/* A grid, not a wrapping row. Four fields of different widths wrap into a
          ragged shape the moment the column narrows — measured at 577px wide:
          BPM and Key on one line, Duration and Language on the next with a hole
          beside them. Columns of a shared width fill predictably instead. */}
      {profile.config.work_meta_fields.length > 0 && (
        <Panel className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-4 p-4">
          {profile.config.work_meta_fields.map((field) => (
            // A paragraph takes the whole row: in a 180px column it would be a
            // column of two-word lines, which is worse than the single-line box
            // it replaced. The span sits on a wrapper rather than being threaded
            // through every branch of MetaInput.
            <div
              key={field.key}
              className={field.type === 'multiline' ? 'col-span-full' : undefined}
            >
              <MetaInput
                field={field}
                value={work.meta[field.key]}
                onChange={(value) => setMeta(field.key, value)}
              />
            </div>
          ))}
        </Panel>
      )}
    </div>
  )
}

/**
 * One profile-defined field.
 *
 * Width follows the type rather than being one size for everything: a date is
 * exactly as wide as a date, a title-length text field is not 8rem.
 */
function MetaInput({
  field,
  value,
  onChange,
}: {
  field: MetaField
  value: Meta[string]
  onChange: (value: Meta[string]) => void
}) {
  const { t } = useTranslation()

  if (field.type === 'boolean') {
    return (
      <Field label={sayLabel(field.label)}>
        <input
          type="checkbox"
          className="size-4 accent-[var(--accent)]"
          checked={value === true}
          onChange={(event) => onChange(event.target.checked)}
        />
      </Field>
    )
  }

  if (field.type === 'multiline') {
    return (
      <Field label={sayLabel(field.label)}>
        <MetaText multiline rows={5} value={value} onCommit={(text) => onChange(text)} />
      </Field>
    )
  }

  if (field.type === 'date') {
    // In a `Field` like the rest since it stopped wrapping its content in a
    // `<label>`: the caption is tied to the trigger by id, so a click on it
    // opens the month once instead of reaching the trigger twice.
    return (
      <Field label={sayLabel(field.label)}>
        <DatePicker
          className="w-full"
          placeholder={t('work.noDate')}
          value={typeof value === 'string' ? value : ''}
          onChange={(next) => onChange(next)}
        />
      </Field>
    )
  }

  return (
    <Field label={sayLabel(field.label)}>
      <MetaText
        number={field.type === 'number'}
        value={value}
        onCommit={(raw) => {
          if (raw === '') return onChange('')
          if (field.type !== 'number') return onChange(raw)
          // Numbers are stored as numbers so scoring and sorting can use them;
          // a value that is not a number yet is kept as typed rather than
          // dropped.
          const parsed = Number(raw)
          onChange(Number.isNaN(parsed) ? raw : parsed)
        }}
      />
    </Field>
  )
}

/**
 * A profile field's box, bound to the stored value by the rules in
 * `lib/fieldDraft`: it follows the stored value while nobody is typing,
 * writes only when what was typed differs, and Escape puts it back.
 *
 * These were uncontrolled boxes that wrote on every blur, which is how a
 * value a plugin had just written was put back to the old one by the next
 * pass of the Tab key, and how tabbing through twelve fields left twelve
 * operations and twelve toasts behind.
 */
function MetaText({
  value,
  onCommit,
  multiline = false,
  number = false,
  rows,
  id,
  'aria-describedby': describedBy,
}: {
  value: Meta[string]
  onCommit: (text: string) => void
  multiline?: boolean
  number?: boolean
  rows?: number
  /** Set by the `Field` around it, so its caption names this box. */
  id?: string
  'aria-describedby'?: string
}) {
  const stored = value === undefined || value === null || value === false ? '' : String(value)
  const draft = useFieldDraft(stored, onCommit, { multiline })

  if (multiline) {
    return <Textarea id={id} aria-describedby={describedBy} rows={rows} {...draft} />
  }
  return (
    <Input
      id={id}
      aria-describedby={describedBy}
      className="w-full"
      type={number ? 'number' : 'text'}
      {...draft}
    />
  )
}
