import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { Meta, MetaField, Work, WorkPatch } from '@/lib/api/types'
import { unpinStatus, updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { DatePicker } from '@/components/DatePicker'
import { Field, FieldGroup } from '@/components/ui/field'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { useFieldDraft } from '@/components/ui/field-draft'
import { InlineField, numberCodec } from '@/components/ui/inline-field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Panel } from '@/components/ui/panel'
import { SaveState, useSaveStatus } from '@/components/ui/save-state'
import { Select } from '@/components/AppSelect'

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

  // The same rules as every other field here (`useFieldDraft`): written when
  // it changed, Escape puts it back. A title cannot be emptied, so an empty box
  // goes back to the stored title rather than stay showing one never saved.
  // A box rather than an InlineField, because it stands in a row of selects.
  const title = useFieldDraft(work.title, (text) => {
    const next = text.trim()
    if (next === '' || next === work.title) return false
    patch.mutate({ title: next })
  })

  // Whose status this is. Picking one from the list pins it — the automation
  // then steps over this work entirely — and the only way back is to say so.
  const pinned = work.status_pinned_at != null
  const statusHint = pinned ? t('work.statusPinned') : t('work.statusDerived')

  const unpin = useAppMutation({
    mutationFn: () => unpinStatus(work.id),
    failure: 'toast.workSaveFailed',
    refresh: refresh.work,
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
    },
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
          <FieldGroup label={t('work.status')} help={pinned ? undefined : statusHint}>
            <Select
              aria-label={t('work.status')}
              className="w-full"
              value={work.status}
              onChange={(status) => patch.mutate({ status })}
              options={vocabulary.statuses.map((s) => ({ value: s.key, label: sayLabel(s.label) }))}
            />
          </FieldGroup>

          {pinned && (
            <p className="mt-1 text-xs text-faint">
              {t('work.statusPinned')}{' '}
              <Button
                variant="link"
                onClick={() => unpin.mutate()}
                disabled={unpin.isPending}
                title={t('work.unpinStatusHint')}
              >
                {t('work.unpinStatus')}
              </Button>
            </p>
          )}
        </div>

        <FieldGroup label={t('work.kind')}>
          <Select
            aria-label={t('work.kind')}
            className="w-full"
            value={work.kind}
            onChange={(kind) => patch.mutate({ kind })}
            options={profile.config.work_kinds.map((k) => ({
              value: k.key,
              label: sayLabel(k.label),
            }))}
          />
        </FieldGroup>

        {/* Spanning the row rather than taking a column of its own: it is a
            word that appears for a second while saving, and a whole grid track
            reserved for it would narrow the fields permanently. */}
        <SaveState
          savingLabel={t('save.saving')}
          savedLabel={t('save.saved')}
          status={saveStatus}
          className="col-span-full"
        />
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
 *
 * A word or a number is an InlineField: read far more often than it is
 * edited, so it sits as text until it is touched, under the same rules as
 * every box here - it follows the stored value while nobody is typing,
 * writes only a change, and Escape puts it back. They were once uncontrolled
 * boxes that wrote on every blur, which is how a value a plugin had just
 * written was put back to the old one by the next pass of the Tab key, and
 * how tabbing through twelve fields left twelve operations and twelve toasts
 * behind.
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
    // The words beside the box are its name, and the whole row is what a
    // pointer aims at - the checkbox carries its label itself.
    return (
      <Checkbox checked={value === true} onCheckedChange={(checked) => onChange(checked)}>
        {sayLabel(field.label)}
      </Checkbox>
    )
  }

  if (field.type === 'multiline') {
    return (
      <FieldGroup label={sayLabel(field.label)}>
        <MetaParagraph
          value={value}
          onCommit={(text) => onChange(text)}
          label={sayLabel(field.label)}
        />
      </FieldGroup>
    )
  }

  if (field.type === 'date') {
    // A group rather than a Field: the picker is a button and a popup, not
    // an input a Field could hand its id to - so it carries its own name.
    return (
      <FieldGroup label={sayLabel(field.label)}>
        <DatePicker
          aria-label={sayLabel(field.label)}
          className="w-full"
          placeholder={t('work.noDate')}
          value={typeof value === 'string' ? value : ''}
          onChange={(next) => onChange(next)}
        />
      </FieldGroup>
    )
  }

  // An empty value shows a dash: an undressed box with nothing in it reads
  // as a rendering fault, not as "nothing yet".
  if (field.type === 'number') {
    // Numbers are stored as numbers so scoring and sorting can use them; what
    // cannot be read as one is refused by the field rather than stored as
    // text. A number a plugin wrote as text is still shown as the number.
    const stored =
      typeof value === 'number'
        ? value
        : typeof value === 'string'
          ? numberCodec.parse(value)
          : null
    return (
      <InlineField
        label={sayLabel(field.label)}
        placeholder="—"
        codec={numberCodec}
        value={stored ?? null}
        onCommit={(next) => onChange(next ?? '')}
      />
    )
  }

  return (
    <InlineField
      label={sayLabel(field.label)}
      placeholder="—"
      value={textOf(value)}
      onCommit={(next) => onChange(next ?? '')}
    />
  )
}

/** A stored value as the text a box shows; nothing for none. */
function textOf(value: Meta[string]): string | null {
  return value === undefined || value === null || value === false ? null : String(value)
}

/**
 * A paragraph field's box, bound to the stored value by the same rules as an
 * InlineField (`useFieldDraft`), with Enter kept for new lines.
 */
function MetaParagraph({
  value,
  onCommit,
  label,
}: {
  value: Meta[string]
  onCommit: (text: string) => void
  /** The box's name. The group around it names the group; a Field could not
   *  hand the box its id through a component of kilna's own. */
  label: string
}) {
  const draft = useFieldDraft(textOf(value) ?? '', onCommit, { multiline: true })
  return <Textarea aria-label={label} rows={5} {...draft} />
}
