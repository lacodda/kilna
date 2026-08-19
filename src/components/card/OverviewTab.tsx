import { useState } from 'react'
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
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { DatePicker } from '@/components/ui/DatePicker'
import { Button } from '@/components/ui/Button'
import { Field, Input } from '@/components/ui/Input'
import { Panel } from '@/components/ui/Panel'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'
import { Select } from '@/components/ui/Select'

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
  const client = useQueryClient()

  // The field is edited locally but owned by the query: whenever the stored
  // title changes underneath it — a fresh load, a plugin rewriting it — the
  // draft is dropped. Adjusting during render rather than in an effect avoids
  // the frame where the input still shows the previous work's title.
  const [title, setTitle] = useState(work.title)
  const [syncedTo, setSyncedTo] = useState(work.title)

  if (syncedTo !== work.title) {
    setSyncedTo(work.title)
    setTitle(work.title)
  }

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
      if (context?.previous !== undefined) {
        client.setQueryData(keys.work(work.id), context.previous)
        setTitle(context.previous?.title ?? '')
      }
      say.failedTo(t('toast.workSaveFailed'), cause)
    },
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
    },
    onSettled: () => {
      // The list shows title, status and kind, so any of these changes it. A
      // rename or a status change is also written down in the history tab.
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.journal })
    },
  })

  const saveStatus = useSaveStatus(patch.isPending, patch.isError)

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
      <Panel className="flex flex-wrap items-end gap-4 p-4">
        <Field label={t('work.title')}>
          <Input
            className="min-w-72"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => {
              if (title.trim() !== '' && title !== work.title) patch.mutate({ title: title.trim() })
            }}
          />
        </Field>

        <Field label={t('work.status')} hint={statusHint}>
          <div className="flex items-center gap-2">
            <Select
              className="w-44"
              value={work.status}
              onChange={(status) => patch.mutate({ status })}
              options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
            />
            {pinned && (
              <Button
                size="sm"
                onClick={() => unpin.mutate()}
                disabled={unpin.isPending}
                title={t('work.unpinStatusHint')}
              >
                {t('work.unpinStatus')}
              </Button>
            )}
          </div>
        </Field>

        <Field label={t('work.kind')}>
          <Select
            className="w-44"
            value={work.kind}
            onChange={(kind) => patch.mutate({ kind })}
            options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
          />
        </Field>

        <SaveState status={saveStatus} className="mb-2.5" />
      </Panel>

      {profile.config.work_meta_fields.length > 0 && (
        <Panel className="flex flex-wrap gap-4 p-4">
          {profile.config.work_meta_fields.map((field) => (
            <MetaInput
              key={field.key}
              field={field}
              value={work.meta[field.key]}
              onChange={(value) => setMeta(field.key, value)}
            />
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
      <Field label={field.label}>
        <input
          type="checkbox"
          className="size-4 accent-[var(--accent)]"
          checked={value === true}
          onChange={(event) => onChange(event.target.checked)}
        />
      </Field>
    )
  }

  if (field.type === 'date') {
    // Not a `<label>` wrapper: clicking the caption would reach the popover
    // trigger as well and toggle it twice.
    return (
      <div className="flex flex-col gap-1">
        <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
          {field.label}
        </span>
        <DatePicker
          className="w-40"
          aria-label={field.label}
          placeholder={t('work.noDate')}
          value={typeof value === 'string' ? value : ''}
          onChange={(next) => onChange(next)}
        />
      </div>
    )
  }

  return (
    <Field label={field.label}>
      <Input
        className={field.type === 'number' ? 'w-28' : 'w-56'}
        type={field.type === 'number' ? 'number' : 'text'}
        defaultValue={String(value ?? '')}
        onBlur={(event) => {
          const raw = event.target.value
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
