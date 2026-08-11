import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { deleteWork, getWork, updateWork, type Meta, type Work } from '@/lib/api'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Field, Input } from '@/components/ui/Input'
import { Select } from '@/components/ui/Select'
import { VersionPanel } from '@/components/VersionPanel'
import { ScorePanel } from '@/components/ScorePanel'
import { ReleasePanel } from '@/components/ReleasePanel'
import { AssistantPanel } from '@/components/AssistantPanel'
import { NotePanel } from '@/components/NotePanel'

interface Props {
  workId: string
  onChanged: () => void
  onDeleted: () => void
}

export function WorkCard({ workId, onChanged, onDeleted }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [work, setWork] = useState<Work | null>(null)
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    getWork(workId)
      .then((loaded) => {
        setWork(loaded)
        setTitle(loaded?.title ?? '')
        setError(null)
      })
      .catch((cause: unknown) => setError(String(cause)))
  }, [workId, revision])

  const patch = (changes: Parameters<typeof updateWork>[1]) => {
    updateWork(workId, changes)
      .then((updated) => {
        setWork(updated)
        onChanged()
      })
      .catch((cause: unknown) => setError(String(cause)))
  }

  const setMetaField = (key: string, value: string, type: string) => {
    if (work === null) return

    const meta: Meta = { ...work.meta }
    if (value === '') {
      delete meta[key]
    } else {
      // Numbers are stored as numbers so scoring and sorting can use them;
      // a value that is not a number yet is kept as typed rather than dropped.
      const parsed = type === 'number' ? Number(value) : value
      meta[key] = type === 'number' && Number.isNaN(parsed as number) ? value : parsed
    }
    patch({ meta })
  }

  if (error !== null) {
    return (
      <p role="alert" className="text-sm text-red-600">
        {error}
      </p>
    )
  }

  if (work === null) {
    return <p className="text-sm text-neutral-500">{t('status.loading')}</p>
  }

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-wrap items-end gap-3">
        <Field label={t('work.title')}>
          <Input
            className="min-w-64"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => {
              if (title.trim() !== '' && title !== work.title) patch({ title: title.trim() })
            }}
          />
        </Field>

        <Field label={t('work.status')}>
          <Select
            className="w-44"
            value={work.status}
            onChange={(status) => patch({ status })}
            options={profile.config.statuses.map((s) => ({ value: s.key, label: s.label }))}
          />
        </Field>

        <Field label={t('work.kind')}>
          <Select
            className="w-44"
            value={work.kind}
            onChange={(kind) => patch({ kind })}
            options={profile.config.work_kinds.map((k) => ({ value: k.key, label: k.label }))}
          />
        </Field>

        <Button
          variant="danger"
          className="ml-auto"
          onClick={() => {
            deleteWork(workId)
              .then(onDeleted)
              .catch((cause: unknown) => setError(String(cause)))
          }}
        >
          {t('work.delete')}
        </Button>
      </header>

      {profile.config.work_meta_fields.length > 0 && (
        <section className="flex flex-wrap gap-3">
          {profile.config.work_meta_fields.map((field) => (
            <Field key={field.key} label={field.label}>
              <Input
                className="w-32"
                type={field.type === 'number' ? 'number' : field.type === 'date' ? 'date' : 'text'}
                defaultValue={String(work.meta[field.key] ?? '')}
                onBlur={(event) => setMetaField(field.key, event.target.value, field.type)}
              />
            </Field>
          ))}
        </section>
      )}

      <VersionPanel workId={workId} onChanged={() => setRevision((r) => r + 1)} />

      <ScorePanel workId={workId} onChanged={onChanged} />

      <ReleasePanel workId={workId} workTitle={work.title} onChanged={onChanged} />

      <AssistantPanel workId={workId} />

      <NotePanel workId={workId} />
    </div>
  )
}
