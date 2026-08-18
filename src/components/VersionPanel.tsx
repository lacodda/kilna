import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  createVersion,
  deleteVersion,
  getVersion,
  listVersions,
  setCurrentVersion,
} from '@/lib/api'
import { clearDraft, readDraft, writeDraft } from '@/lib/drafts'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { Markdown } from '@/components/ui/Markdown'
import { Select } from '@/components/ui/Select'
import { Skeleton } from '@/components/ui/Skeleton'
import { VersionDiff } from '@/components/versions/VersionDiff'
import { VersionEditor } from '@/components/versions/VersionEditor'
import { VersionList } from '@/components/versions/VersionList'

interface Props {
  workId: string
}

/** How the body of the open version is shown. */
const READINGS = ['text', 'preview'] as const

// Versions of one role at a time: lyrics and style advance independently, and
// showing them interleaved would suggest otherwise.
export function VersionPanel({ workId }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const roles = profile.config.version_roles

  const [role, setRole] = useState(roles[0]?.key ?? '')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [comparedId, setComparedId] = useState<string | null>(null)
  const [reading, setReading] = useState<(typeof READINGS)[number]>('text')

  // Drafts belong to the work and the role they were typed under, and outlive
  // the window: losing a page of writing to a closed window is the kind of
  // thing a writing tool is never forgiven for. See `lib/drafts`.
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const draft = drafts[role] ?? readDraft(workId, role)
  const setDraft = (body: string) => {
    setDrafts((all) => ({ ...all, [role]: body }))
    writeDraft(workId, role, body)
  }

  const [label, setLabel] = useState('')
  // A new draft is almost always the one being worked on — but not always, and
  // the backend has always accepted `make_current: false`.
  const [makeCurrentOnSave, setMakeCurrentOnSave] = useState(true)

  const versions = useQuery({
    queryKey: keys.versions(workId),
    queryFn: () => listVersions(workId),
  })

  const summaries = (versions.data ?? []).filter((version) => version.role === role)
  // Open the newest of this role by default, so the panel is never blank; an
  // explicit choice wins until it disappears.
  const openId =
    selectedId !== null && summaries.some((v) => v.id === selectedId)
      ? selectedId
      : (summaries[0]?.id ?? null)

  const open = useQuery({
    queryKey: keys.version(openId ?? ''),
    queryFn: () => getVersion(openId!),
    enabled: openId !== null,
  })

  const compared = useQuery({
    queryKey: keys.version(comparedId ?? ''),
    queryFn: () => getVersion(comparedId!),
    enabled: comparedId !== null,
  })

  // A version changes the list, the work's current pointer, the summary the
  // works list shows, and the history kept underneath the card.
  const refreshed = [keys.journal, keys.versions(workId), keys.work(workId), keys.works]

  const settle = () => {
    for (const key of refreshed) void client.invalidateQueries({ queryKey: key })
  }

  const save = useMutation({
    mutationFn: () =>
      createVersion(workId, {
        role,
        body: draft,
        label: label.trim() === '' ? null : label.trim(),
        make_current: makeCurrentOnSave,
      }),
    onSuccess: (version) => {
      // The draft became a version; there is nothing left to keep.
      setDrafts((all) => ({ ...all, [role]: '' }))
      clearDraft(workId, role)
      setLabel('')
      setSelectedId(version.id)
      settle()
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const makeCurrent = useMutation({
    mutationFn: (versionId: string) => setCurrentVersion(workId, versionId),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteVersion,
    onSuccess: (deletionId, versionId) => {
      if (selectedId === versionId) setSelectedId(null)
      if (comparedId === versionId) setComparedId(null)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.versionDeleted'),
        refresh: refreshed,
        // Reopen what was being read when it was thrown away.
        onUndone: () => setSelectedId(versionId),
      })
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const nameOf = (id: string | null): string => {
    const found = summaries.find((version) => version.id === id)
    if (found === undefined) return ''
    return found.label ?? t('versions.revision', { number: found.revision })
  }

  const comparing = comparedId !== null && open.data != null && compared.data != null

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('versions.title')}</h3>
        {roles.length > 1 && (
          <Select
            className="w-40"
            aria-label={t('versions.role')}
            value={role}
            onChange={(next) => {
              setRole(next)
              // The selection and the comparison belonged to the role being left.
              setSelectedId(null)
              setComparedId(null)
            }}
            options={roles.map((r) => ({ value: r.key, label: r.label }))}
          />
        )}
      </div>

      {/* The list only takes a column of its own once there is room for the
          text beside it. Below that it sits above, full width, because a
          comparison squeezed into 130px is two columns of hyphens. */}
      <div className="grid gap-4 xl:grid-cols-[15rem_minmax(0,1fr)]">
        <VersionList
          versions={summaries}
          loading={versions.isPending}
          openId={openId}
          comparedId={comparedId}
          onOpen={(id) => {
            setSelectedId(id)
            // Comparing something against itself is not a state worth having.
            if (comparedId === id) setComparedId(null)
          }}
          onCompare={(id) => setComparedId(comparedId === id ? null : id)}
          onMakeCurrent={(id) => makeCurrent.mutate(id)}
          onDelete={(id) => remove.mutate(id)}
        />

        <div className="flex min-w-0 flex-col gap-4">
          {open.isPending && openId !== null && <Skeleton className="h-32 w-full" />}

          {comparing && open.data != null && compared.data != null && (
            <VersionDiff
              before={compared.data.body}
              after={open.data.body}
              beforeLabel={nameOf(comparedId)}
              afterLabel={nameOf(openId)}
            />
          )}

          {!comparing && open.data != null && (
            <article className="overflow-hidden rounded-xl border border-line">
              <header className="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2 text-xs text-dim">
                <span>
                  {labelOf(roles, open.data.role)}
                  {' · '}
                  {open.data.label ?? t('versions.revision', { number: open.data.revision })}
                  {' · '}
                  {open.data.created_at.slice(0, 10)}
                </span>
                <div className="ml-auto flex gap-1">
                  {READINGS.map((mode) => (
                    <Button
                      key={mode}
                      variant={reading === mode ? 'soft' : 'icon'}
                      size="sm"
                      aria-pressed={reading === mode}
                      onClick={() => setReading(mode)}
                    >
                      {t(`versions.${mode}`)}
                    </Button>
                  ))}
                </div>
              </header>

              <div className="max-h-[28rem] overflow-auto px-3 py-2.5">
                {reading === 'preview' ? (
                  <Markdown body={open.data.body} />
                ) : (
                  <pre className="whitespace-pre-wrap font-mono text-sm">{open.data.body}</pre>
                )}
              </div>
            </article>
          )}

          <VersionEditor
            draft={draft}
            onDraftChange={setDraft}
            label={label}
            onLabelChange={setLabel}
            makeCurrent={makeCurrentOnSave}
            onMakeCurrentChange={setMakeCurrentOnSave}
            onSave={() => {
              if (draft.trim() !== '') save.mutate()
            }}
            saving={save.isPending}
            kept={draft.trim() !== ''}
          />
        </div>
      </div>
    </section>
  )
}
