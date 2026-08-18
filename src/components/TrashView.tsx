import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { RotateCcw, X } from 'lucide-react'
import {
  emptyTrash,
  listDeletions,
  purgeDeletion,
  restoreDeletion,
  type DeletedEntity,
  type Deletion,
} from '@/lib/api'
import { clearDraftsFor } from '@/lib/drafts'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/Button'
import { Dialog } from '@/components/ui/Dialog'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'

// Restoring anything can put back a work, a version, a score, a release or a
// membership, so the whole set is refreshed rather than guessed at per entity —
// the trash is not a hot path, and a stale screen after an undo is worse than a
// query that did not need running.
const REFRESHED = [
  keys.works,
  keys.workspace,
  keys.catalogue,
  keys.calendar,
  keys.releases,
  keys.releaseQueue,
  keys.scores,
  keys.notes,
  keys.tags,
  keys.collections,
  keys.deletions,
] as const

/** The word for what an entry used to be. */
function entityLabel(entity: DeletedEntity, t: (key: string) => string): string {
  return t(`trash.entity.${entity}`)
}

function Row({
  entry,
  onRestore,
  onPurge,
  busy,
}: {
  entry: Deletion
  onRestore: () => void
  onPurge: () => void
  busy: boolean
}) {
  const { t } = useTranslation()

  return (
    <tr className="border-b border-line">
      <td className="py-2 pr-3">
        <span className={cn('font-medium', !entry.restorable && 'text-dim')}>{entry.label}</span>
        {entry.origin !== null && <span className="ml-2 text-xs text-dim">{entry.origin}</span>}
      </td>
      <td className="py-2 pr-3">
        <span className="rounded bg-soft px-1.5 py-0.5 text-xs text-dim">
          {entityLabel(entry.entity, t)}
        </span>
      </td>
      <td className="py-2 pr-3 text-xs text-dim">{entry.deleted_at.slice(0, 10)}</td>
      <td className="py-2">
        <div className="flex justify-end gap-1">
          <Button
            variant="icon"
            size="iconSm"
            disabled={busy || !entry.restorable}
            // A version whose work is also in the trash cannot come back alone;
            // saying so on hover beats letting the click fail.
            title={entry.restorable ? t('trash.restore') : t('error.notRestorable')}
            aria-label={t('trash.restore')}
            onClick={onRestore}
          >
            <RotateCcw aria-hidden />
          </Button>
          <Button
            variant="icon"
            size="iconSm"
            disabled={busy}
            className="hover:text-bad"
            title={t('trash.purge')}
            aria-label={t('trash.purge')}
            onClick={onPurge}
          >
            <X aria-hidden />
          </Button>
        </div>
      </td>
    </tr>
  )
}

/**
 * What was thrown away, and the way back.
 *
 * Deleting anywhere in the app lands here, which is what lets the rest of the
 * app delete without asking first.
 */
export function TrashView() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [confirmingEmpty, setConfirmingEmpty] = useState(false)

  const entries = useQuery({ queryKey: keys.deletions, queryFn: listDeletions })

  const settle = () => {
    for (const key of REFRESHED) void client.invalidateQueries({ queryKey: key })
  }

  const restore = useMutation({
    mutationFn: restoreDeletion,
    onSuccess: () => {
      settle()
      say.ok(t('trash.restored'))
    },
    onError: (cause) => say.failedTo(t('trash.restoreFailed'), cause),
  })

  // Purging and emptying are the only irreversible actions in the app, so they
  // are the only ones that still ask — there is no trash behind the trash.
  const purge = useMutation({
    mutationFn: (entry: Deletion) => purgeDeletion(entry.id),
    onSuccess: (_result, entry) => {
      // Unsaved drafts are keyed by the work they belong to. Once the work is
      // gone for good nothing can ever reach them again, so they go with it.
      if (entry.entity === 'work') clearDraftsFor(entry.entity_id)
      void client.invalidateQueries({ queryKey: keys.deletions })
    },
    onError: (cause) => say.failedTo(t('trash.purgeFailed'), cause),
  })

  const empty = useMutation({
    mutationFn: emptyTrash,
    onSuccess: (count) => {
      // Same reasoning as purge, for everything at once.
      for (const entry of entries.data ?? []) {
        if (entry.entity === 'work') clearDraftsFor(entry.entity_id)
      }
      setConfirmingEmpty(false)
      void client.invalidateQueries({ queryKey: keys.deletions })
      say.ok(t('trash.emptied', { count }))
    },
    onError: (cause) => say.failedTo(t('trash.emptyFailed'), cause),
  })

  if (entries.isPending) return <SkeletonList rows={5} />

  if (entries.isError) {
    return (
      <p role="alert" className="text-sm text-bad">
        {t('toast.loadFailed')}
      </p>
    )
  }

  if (entries.data.length === 0) {
    return <EmptyState title={t('empty.trashTitle')} body={t('empty.trashBody')} />
  }

  const busy = restore.isPending || purge.isPending || empty.isPending

  return (
    <div className="flex flex-col gap-3">
      <header className="flex items-center gap-3">
        <h2 className="text-sm font-semibold">{t('trash.title')}</h2>
        <p className="text-xs text-dim">{t('trash.hint')}</p>
        <Button
          variant="danger"
          size="sm"
          className="ml-auto"
          disabled={busy}
          onClick={() => setConfirmingEmpty(true)}
        >
          {t('trash.empty')}
        </Button>
      </header>

      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-line text-left text-xs uppercase tracking-wide text-dim">
            <th className="py-2 pr-3 font-medium">{t('trash.what')}</th>
            <th className="py-2 pr-3 font-medium">{t('trash.kind')}</th>
            <th className="py-2 pr-3 font-medium">{t('trash.when')}</th>
            <th className="py-2" />
          </tr>
        </thead>
        <tbody>
          {entries.data.map((entry) => (
            <Row
              key={entry.id}
              entry={entry}
              busy={busy}
              onRestore={() => restore.mutate(entry.id)}
              onPurge={() => purge.mutate(entry)}
            />
          ))}
        </tbody>
      </table>

      <Dialog
        open={confirmingEmpty}
        onOpenChange={setConfirmingEmpty}
        title={t('trash.emptyTitle')}
        description={t('trash.emptyBody', { count: entries.data.length })}
        footer={
          <Button variant="primary" disabled={empty.isPending} onClick={() => empty.mutate()}>
            {t('trash.empty')}
          </Button>
        }
      />
    </div>
  )
}
