import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { emptyTrash, purgeDeletion, restoreDeletion } from '@/lib/api/trash'
import type { DeletedEntity, Deletion } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { clearDraftsFor } from '@/lib/drafts'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useCovers } from '@/lib/useCovers'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ConfirmAction } from '@/components/ConfirmAction'
import { Frame, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { cn } from '@/lib/utils'
import { formatStamp } from '@/lib/format'

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
  fact: 'trash.entity.fact',
  term: 'trash.entity.term',
  idea: 'trash.entity.idea',
  block: 'trash.entity.block',
  trial: 'trash.entity.trial',
}

function entityLabel(entity: DeletedEntity, t: (key: string) => string): string {
  return t(ENTITY_KEYS[entity])
}

/*
 * A danger button in a ghost's outline: the mockup's `.ghost.bad`. The set's
 * danger button has no border, and beside a bordered "Restore" it read as a
 * link rather than as the other of two choices.
 */
const DANGER_GHOST = 'border border-line'

function Row({
  entry,
  cover,
  onRestore,
  onPurge,
  busy,
}: {
  entry: Deletion
  /** The picture of the work it belongs to, when that work has one. */
  cover: string | undefined
  onRestore: () => void
  onPurge: () => void
  busy: boolean
}) {
  const { t } = useTranslation()

  return (
    <TableRow>
      {/* The column that takes the rest of the width, and the one that
          truncates: `max-w-0` is what lets a table cell give up width to
          its neighbours at all. */}
      <TableCell className="w-full max-w-0">
        <span className="flex min-w-0 items-center gap-2">
          {/* The cover of the work it is or came from, so a version sits
              beside its song in the song's colour. What hangs off no work
              gets a plain square rather than a colour that belongs to
              nothing. */}
          <span
            aria-hidden
            className={cn(
              'size-5 shrink-0 rounded-sm border border-line/60',
              entry.work_id === null && 'bg-soft',
              !entry.restorable && 'opacity-50',
            )}
            style={
              entry.work_id === null
                ? undefined
                : { background: coverImageFor(entry.work_id, cover) }
            }
          />
          {/* The whole of it on hover, since a narrow window cuts it short. */}
          <span
            className="min-w-0 truncate"
            title={
              entry.origin === null
                ? entry.label
                : `${entry.label} ${t('trash.from', { origin: entry.origin })}`
            }
          >
            <span className={cn('font-semibold', !entry.restorable && 'text-dim')}>
              {entry.label}
            </span>
            {entry.origin !== null && (
              <span className="ml-1 text-xs text-faint">
                {t('trash.from', { origin: entry.origin })}
              </span>
            )}
          </span>
        </span>
      </TableCell>
      <TableCell className="whitespace-nowrap text-dim">{entityLabel(entry.entity, t)}</TableCell>
      <TableCell className="font-mono text-xs whitespace-nowrap text-dim tabular-nums">
        <time dateTime={entry.deleted_at} title={entry.deleted_at}>
          {formatStamp(entry.deleted_at)}
        </time>
      </TableCell>
      <TableCell>
        <div className="flex justify-end gap-1.5">
          <Button
            variant="ghost"
            size="xs"
            disabled={busy || !entry.restorable}
            // A version whose work is also in the trash cannot come back
            // alone. The button stays where the pointer and the Tab key
            // reach it, and says why - the tooltip on a disabled button was
            // dead until the set's Button learned to keep it.
            disabledReason={entry.restorable ? undefined : t('error.notRestorable')}
            onClick={onRestore}
          >
            {t('trash.restore')}
          </Button>
          <Button
            variant="danger"
            size="xs"
            className={DANGER_GHOST}
            disabled={busy}
            onClick={onPurge}
          >
            {t('trash.purge')}
          </Button>
        </div>
      </TableCell>
    </TableRow>
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
  const covers = useCovers()
  const [confirmingEmpty, setConfirmingEmpty] = useState(false)
  // The entry waiting on "delete for good?". Kept as the entry rather than an
  // id so the question can name it.
  const [purging, setPurging] = useState<Deletion | null>(null)

  const entries = useQuery(queries.deletions())

  const restore = useAppMutation({
    mutationFn: restoreDeletion,
    failure: 'trash.restoreFailed',
    onSuccess: () => {
      // Restoring can put back anything - a work with its versions, scenes,
      // cuts, pictures and comments, a style with its references, a
      // membership - so every query is refreshed rather than a list of them
      // kept here. The list this replaced had already missed scenes, cuts,
      // comments and versions: a restored scene stayed invisible on its card
      // for up to half a minute. The trash is not a hot path; a stale screen
      // after a restore costs more than queries that did not need running.
      void client.invalidateQueries()
      say.ok(t('trash.restored'))
    },
  })

  // Purging and emptying are the only irreversible actions in the app, so they
  // are the only ones that still ask — there is no trash behind the trash.
  // Both ask through ConfirmAction: an alert a stray click cannot dismiss.
  const purge = useAppMutation({
    mutationFn: (entry: Deletion) => purgeDeletion(entry.id),
    failure: 'trash.purgeFailed',
    refresh: [keys.deletions],
    onSuccess: (_result, entry) => {
      // Unsaved drafts are keyed by the work they belong to. Once the work is
      // gone for good nothing can ever reach them again, so they go with it.
      if (entry.entity === 'work') clearDraftsFor(entry.entity_id)
      setPurging(null)
    },
  })

  const empty = useAppMutation({
    mutationFn: emptyTrash,
    failure: 'trash.emptyFailed',
    refresh: [keys.deletions],
    onSuccess: (count) => {
      // Same reasoning as purge, for everything at once.
      for (const entry of entries.data ?? []) {
        if (entry.entity === 'work') clearDraftsFor(entry.entity_id)
      }
      setConfirmingEmpty(false)
      say.ok(t('trash.emptied', { count }))
    },
  })

  const busy = restore.isPending || purge.isPending || empty.isPending
  const count = entries.data?.length ?? 0

  return (
    <Frame
      head={
        <>
          <p className="text-sm text-faint">{t('trash.hint')}</p>
          <Button
            variant="danger"
            size="sm"
            className={cn('ml-auto', DANGER_GHOST)}
            disabled={busy || count === 0}
            disabledReason={
              entries.data !== undefined && count === 0 ? t('empty.trashTitle') : undefined
            }
            onClick={() => setConfirmingEmpty(true)}
          >
            {t('trash.empty')}
          </Button>
        </>
      }
    >
      <Loaded
        query={entries}
        fill
        skeleton={<SkeletonList rows={5} />}
        isEmpty={(data) => data.length === 0}
        emptyState={
          <EmptyState
            title={t('empty.trashTitle')}
            body={t('empty.trashBody')}
            className="flex-1"
          />
        }
      >
        {(data) => (
          <Pane label={t('nav.trash')}>
            <Table>
              {/* Sticky inside the pane's scroll: two hundred rows down, the
                  columns still say what they are. */}
              <TableHead sticky className="bg-raise">
                <tr className="border-b border-line">
                  <TableHeader className="caption font-semibold">{t('trash.what')}</TableHeader>
                  <TableHeader className="caption font-semibold">{t('trash.kind')}</TableHeader>
                  <TableHeader className="caption font-semibold">{t('trash.when')}</TableHeader>
                  <TableHeader />
                </tr>
              </TableHead>
              <TableBody>
                {data.map((entry) => (
                  <Row
                    key={entry.id}
                    entry={entry}
                    cover={entry.work_id === null ? undefined : covers.get(entry.work_id)}
                    busy={busy}
                    onRestore={() => restore.mutate(entry.id)}
                    onPurge={() => setPurging(entry)}
                  />
                ))}
              </TableBody>
            </Table>
          </Pane>
        )}
      </Loaded>

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
        description={t('trash.emptyBody', { count })}
        actionLabel={t('trash.empty')}
        pending={empty.isPending}
        onConfirm={() => empty.mutate()}
      />
    </Frame>
  )
}
