import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { RotateCcw, Trash2 } from 'lucide-react'
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
import { Button } from '@/components/ui/button'
import { ConfirmAction } from '@/components/ui/ConfirmAction'
import { EmptyState } from '@/components/ui/EmptyState'
import { SkeletonList } from '@/components/ui/Skeleton'
import { cn } from '@/lib/utils'


/**
 * The word for what an entry used to be, one key per kind.
 *
 * A table rather than a key built from the kind: a kind added in the backend
 * showed as `trash.entity.cut` for as long as nobody noticed, because a key
 * spelled out at run time is invisible both to the compiler and to the locale
 * check. Written out, a new kind is a type error here until it has a word.
 */
const ENTITY_KEYS: Record<DeletedEntity, string> = {
  work: 'trash.entity.work',
  version: 'trash.entity.version',
  score: 'trash.entity.score',
  release: 'trash.entity.release',
  note: 'trash.entity.note',
  collection: 'trash.entity.collection',
  scene: 'trash.entity.scene',
  cut: 'trash.entity.cut',
  comment: 'trash.entity.comment',
  style: 'trash.entity.style',
}

function entityLabel(entity: DeletedEntity, t: (key: string) => string): string {
  return t(ENTITY_KEYS[entity])
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
            size="icon-sm"
            disabled={busy || !entry.restorable}
            // A version whose work is also in the trash cannot come back alone;
            // saying so on hover beats letting the click fail.
            title={entry.restorable ? t('trash.restore') : t('error.notRestorable')}
            aria-label={t('trash.restore')}
            onClick={onRestore}
          >
            <RotateCcw aria-hidden />
          </Button>
          {/* A bin rather than a cross: a cross reads as "close" or "take off
              this list", and this one deletes for good. */}
          <Button
            variant="danger"
            size="icon-sm"
            disabled={busy}
            title={t('trash.purge')}
            aria-label={t('trash.purge')}
            onClick={onPurge}
          >
            <Trash2 aria-hidden />
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
  // The entry waiting on "delete for good?". Kept as the entry rather than an
  // id so the question can name it.
  const [purging, setPurging] = useState<Deletion | null>(null)

  const entries = useQuery({ queryKey: keys.deletions, queryFn: listDeletions })

  // Restoring can put back anything - a work with its versions, scenes, cuts,
  // pictures and comments, a style with its references, a membership - so
  // every query is refreshed rather than a list of them kept here. The list
  // this replaced had already missed scenes, cuts, comments and versions: a
  // restored scene stayed invisible on its card for up to half a minute. The
  // trash is not a hot path; a stale screen after a restore costs more than
  // queries that did not need running.
  const settle = () => {
    void client.invalidateQueries()
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
  // Both ask through ConfirmAction: an alert a stray click cannot dismiss.
  const purge = useMutation({
    mutationFn: (entry: Deletion) => purgeDeletion(entry.id),
    onSuccess: (_result, entry) => {
      // Unsaved drafts are keyed by the work they belong to. Once the work is
      // gone for good nothing can ever reach them again, so they go with it.
      if (entry.entity === 'work') clearDraftsFor(entry.entity_id)
      setPurging(null)
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
              onPurge={() => setPurging(entry)}
            />
          ))}
        </tbody>
      </table>

      <ConfirmAction
        open={purging !== null}
        onOpenChange={(open) => {
          if (!open) setPurging(null)
        }}
        title={t('trash.purgeTitle', { label: purging?.label ?? '' })}
        description={t('trash.purgeBody')}
        actionLabel={t('trash.purge')}
        pending={purge.isPending}
        onConfirm={() => {
          if (purging !== null) purge.mutate(purging)
        }}
      />

      <ConfirmAction
        open={confirmingEmpty}
        onOpenChange={setConfirmingEmpty}
        title={t('trash.emptyTitle')}
        description={t('trash.emptyBody', { count: entries.data.length })}
        actionLabel={t('trash.empty')}
        pending={empty.isPending}
        onConfirm={() => empty.mutate()}
      />
    </div>
  )
}
