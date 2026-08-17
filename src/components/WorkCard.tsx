import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  deleteWork,
  getWork,
  updateWork,
  type Meta,
  type Work,
  type WorkPatch,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Field, Input } from '@/components/ui/Input'
import { SaveState, useSaveStatus } from '@/components/ui/SaveState'
import { Select } from '@/components/ui/Select'
import { SkeletonCard } from '@/components/ui/Skeleton'
import { VersionPanel } from '@/components/VersionPanel'
import { ScorePanel } from '@/components/ScorePanel'
import { ReleasePanel } from '@/components/ReleasePanel'
import { AssistantPanel } from '@/components/AssistantPanel'
import { NotePanel } from '@/components/NotePanel'
import { WorkHistory } from '@/components/JournalFeed'
import { PluginBar } from '@/components/PluginBar'

interface Props {
  workId: string
  onDeleted: () => void
  /** Reopen a work that was deleted and then brought back. */
  onUndone: (workId: string) => void
}

export function WorkCard({ workId, onDeleted, onUndone }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const work = useQuery({ queryKey: keys.work(workId), queryFn: () => getWork(workId) })

  // The field is edited locally but owned by the query: whenever the stored
  // title changes underneath it — a fresh load, a plugin rewriting it — the
  // draft is dropped. Adjusting during render rather than in an effect avoids
  // the frame where the input still shows the previous work's title.
  const storedTitle = work.data?.title ?? ''
  const [title, setTitle] = useState(storedTitle)
  const [syncedTo, setSyncedTo] = useState(storedTitle)

  if (syncedTo !== storedTitle) {
    setSyncedTo(storedTitle)
    setTitle(storedTitle)
  }

  const patch = useMutation({
    mutationFn: (changes: WorkPatch) => updateWork(workId, changes),
    // Optimistic: the header should not lag behind the field someone just left.
    onMutate: async (changes) => {
      await client.cancelQueries({ queryKey: keys.work(workId) })
      const previous = client.getQueryData<Work | null>(keys.work(workId))

      if (previous != null) {
        client.setQueryData<Work>(keys.work(workId), { ...previous, ...changes })
      }
      return { previous }
    },
    onError: (cause, _changes, context) => {
      // Put back what was there; the toast explains why it moved.
      if (context?.previous !== undefined) {
        client.setQueryData(keys.work(workId), context.previous)
        setTitle(context.previous?.title ?? '')
      }
      say.failedTo(t('toast.workSaveFailed'), cause)
    },
    onSuccess: (updated) => {
      client.setQueryData(keys.work(workId), updated)
    },
    onSettled: () => {
      // The list shows title, status and kind, so any of these changes it. A
      // rename or a status change is also written down, right below this card.
      void client.invalidateQueries({ queryKey: keys.works })
      void client.invalidateQueries({ queryKey: keys.catalogue })
      void client.invalidateQueries({ queryKey: keys.journal })
    },
  })

  const remove = useMutation({
    mutationFn: () => deleteWork(workId),
    onSuccess: (deletionId) => {
      announceDeleted({
        client,
        deletionId,
        message: t('toast.workDeleted', { title: work.data?.title ?? '' }),
        refresh: [keys.works, keys.workspace, keys.catalogue, keys.calendar],
        // The card was closed on the way out; an undo brings it back open.
        onUndone: () => onUndone(workId),
      })
      onDeleted()
    },
    onError: (cause) => say.failedTo(t('toast.workDeleteFailed'), cause),
  })

  const saveStatus = useSaveStatus(patch.isPending, patch.isError)

  const setMetaField = (key: string, value: string, type: string) => {
    if (work.data == null) return

    const meta: Meta = { ...work.data.meta }
    if (value === '') {
      delete meta[key]
    } else {
      // Numbers are stored as numbers so scoring and sorting can use them;
      // a value that is not a number yet is kept as typed rather than dropped.
      const parsed = type === 'number' ? Number(value) : value
      meta[key] = type === 'number' && Number.isNaN(parsed as number) ? value : parsed
    }
    patch.mutate({ meta })
  }

  if (work.isPending) return <SkeletonCard />

  if (work.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  // The row is gone — deleted in another view while this one held its id.
  if (work.data === null) {
    return <p className="text-sm text-dim">{t('error.notFound')}</p>
  }

  const current = work.data

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-end gap-3">
        <Field label={t('work.title')}>
          <Input
            className="min-w-64"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => {
              if (title.trim() !== '' && title !== current.title) patch.mutate({ title: title.trim() })
            }}
          />
        </Field>

        <Field label={t('work.status')}>
          <Select
            className="w-44"
            value={current.status}
            onChange={(status) => patch.mutate({ status })}
            options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
          />
        </Field>

        <Field label={t('work.kind')}>
          <Select
            className="w-44"
            value={current.kind}
            onChange={(kind) => patch.mutate({ kind })}
            options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
          />
        </Field>

        <SaveState status={saveStatus} className="mb-2.5" />

        <Button
          variant="danger"
          className="ml-auto"
          disabled={remove.isPending}
          onClick={() => remove.mutate()}
        >
          {t('work.delete')}
        </Button>
      </header>

      {profile.config.work_meta_fields.length > 0 && (
        <section className="flex flex-wrap gap-3">
          {profile.config.work_meta_fields.map((field) => (
            <Field key={field.key} label={field.label}>
              {field.type === 'boolean' ? (
                // Declared in the profile schema since v0.1.0 and never drawn;
                // a checkbox is the whole of what it needs.
                <input
                  type="checkbox"
                  className="size-4 accent-[var(--accent)]"
                  checked={current.meta[field.key] === true}
                  onChange={(event) =>
                    patch.mutate({ meta: { ...current.meta, [field.key]: event.target.checked } })
                  }
                />
              ) : (
                <Input
                  className="w-32"
                  type={field.type === 'number' ? 'number' : field.type === 'date' ? 'date' : 'text'}
                  defaultValue={String(current.meta[field.key] ?? '')}
                  onBlur={(event) => setMetaField(field.key, event.target.value, field.type)}
                />
              )}
            </Field>
          ))}
        </section>
      )}

      <VersionPanel workId={workId} />

      <ScorePanel workId={workId} />

      <ReleasePanel workId={workId} workTitle={current.title} />

      <AssistantPanel workId={workId} />

      <PluginBar target="work" id={workId} />

      <NotePanel workId={workId} />

      <WorkHistory workId={workId} />
    </div>
  )
}
