import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
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
import { cn } from '@/lib/utils'
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
  // A version named in the address wins until something else is picked. That
  // is what lets a score row open the very draft it judged — and what makes
  // that link work from a note or a message, the same promise the tabs made.
  const [params, setParams] = useSearchParams()
  const asked = params.get('version')
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

  // The asked-for version also decides which lane is open: a link to a draft
  // that lands on the style tab has not opened it.
  const askedRole = (versions.data ?? []).find((version) => version.id === asked)?.role
  const shownRole = askedRole !== undefined && selectedId === null ? askedRole : role
  const summaries = (versions.data ?? []).filter((version) => version.role === shownRole)
  // Open the newest of this role by default, so the panel is never blank; an
  // explicit choice wins until it disappears.
  const openId =
    selectedId !== null && summaries.some((v) => v.id === selectedId)
      ? selectedId
      : asked !== null && summaries.some((v) => v.id === asked)
        ? asked
        : (summaries[0]?.id ?? null)

  const open = useQuery({
    queryKey: keys.version(openId ?? ''),
    queryFn: () => getVersion(openId!),
    enabled: openId !== null,
  })

  // Roles written *about* this one. A profile that names none leaves the panel
  // exactly as it was: one role at a time, nothing beside it.
  const commentRoles = roles.filter((r) => r.comments_on === role)
  const openSummary = summaries.find((version) => version.id === openId) ?? null

  // The commentary on the open revision — not the newest commentary there is.
  // A review of revision 2 says nothing about revision 5, and showing it beside
  // 5 would be the panel asserting something nobody wrote.
  const comments = (versions.data ?? []).filter(
    (version) =>
      openSummary !== null &&
      commentRoles.some((r) => r.key === version.role) &&
      version.revision === openSummary.revision,
  )
  const [openCommentId, setOpenCommentId] = useState<string | null>(null)
  const commentId =
    openCommentId !== null && comments.some((c) => c.id === openCommentId)
      ? openCommentId
      : (comments[0]?.id ?? null)

  const comment = useQuery({
    queryKey: keys.version(commentId ?? ''),
    queryFn: () => getVersion(commentId!),
    enabled: commentId !== null,
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
            value={shownRole}
            onChange={(next) => {
              setRole(next)
              // Picking a lane by hand ends the link's claim on the panel.
              if (asked !== null) setParams({}, { replace: true })
              // The selection and the comparison belonged to the role being left.
              setSelectedId(null)
              setComparedId(null)
            }}
            options={roles
              // Commentary is not a lane: it belongs beside what it comments
              // on, and offering it here would show it stripped of that.
              .filter((r) => r.comments_on === undefined)
              .map((r) => ({ value: r.key, label: r.label }))}
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

          {/* The text and what was written about it, side by side. Reading a
              review away from the lines it discusses is reading half of it —
              which is exactly what the predecessor's two panels got right. The
              split only happens when there is a review of this very revision
              and room for both; below that the review sits underneath. */}
          {!comparing && open.data != null && (
            <div
              className={cn(
                'grid min-w-0 gap-4',
                comments.length > 0 && '2xl:grid-cols-2',
              )}
            >
            <article className="min-w-0 overflow-hidden rounded-xl border border-line">
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

            {comments.length > 0 && (
              <article className="min-w-0 overflow-hidden rounded-xl border border-line">
                <header className="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2 text-xs text-dim">
                  {/* One tab per kind of commentary — a look at the axes and a
                      critique of the lines answer different questions, and the
                      predecessor kept them as separate documents for that
                      reason. With one kind this is a label. */}
                  {comments.map((entry) => (
                    <button
                      key={entry.id}
                      type="button"
                      onClick={() => setOpenCommentId(entry.id)}
                      className={cn(
                        'cursor-pointer rounded-[7px] px-2 py-0.5 transition-colors',
                        entry.id === commentId
                          ? 'bg-soft text-text'
                          : 'hover:text-text',
                      )}
                    >
                      {labelOf(roles, entry.role)}
                    </button>
                  ))}
                  <span className="ml-auto">
                    {t('versions.revision', { number: openSummary?.revision ?? 0 })}
                  </span>
                </header>

                <div className="max-h-[28rem] overflow-auto px-3 py-2.5">
                  {comment.isPending && <Skeleton className="h-32 w-full" />}
                  {comment.data != null && <Markdown body={comment.data.body} />}
                </div>
              </article>
            )}
            </div>
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
