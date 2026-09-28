import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import type { VersionRole, VersionSummary } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { labelOf } from '@/lib/useProfile'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { BodyPane, type Reading } from '@/features/work/tabs/versions/BodyPane'
import { Stage } from '@/features/work/tabs/versions/Stage'

interface Props {
  workId: string
  roles: VersionRole[]
  /** What was written about the open revision; see `commentaryOn`. */
  comments: VersionSummary[]
  /** The open revision's number, for the bar. */
  revision: number
  staged: boolean
  onStage: (staged: boolean) => void
}

/**
 * What was written about the open revision, beside it.
 *
 * Reading a review away from the lines it discusses is reading half of it,
 * so it stands to the right of the text, the same height, each scrolling on
 * its own - never under it, where the two shared one column's height and
 * neither had enough (the audit of 24.09). One segment per kind: a look at
 * the axes and a critique of the lines answer different questions. The
 * commentary is a version of its own role, with its own sitting, so it is
 * edited here the way the text is edited beside it.
 *
 * Its state lives here rather than in the pane, so putting it on the stage -
 * a portal, where the pane mounts anew - keeps the kind picked and the way it
 * is being read.
 */
export function Commentary({ workId, roles, comments, revision, staged, onStage }: Props) {
  const { t } = useTranslation()
  const [pickedId, setPickedId] = useState<string | null>(null)
  const [reading, setReading] = useState<Reading>('view')

  const openId =
    pickedId !== null && comments.some((c) => c.id === pickedId)
      ? pickedId
      : (comments[0]?.id ?? null)

  const comment = useQuery({
    ...queries.version(openId ?? ''),
    enabled: openId !== null,
    placeholderData: keepPreviousData,
  })

  // Handed a version only once it is the one asked for: the placeholder is
  // the previous one, and adopting its text would overwrite what was typed.
  const editing = useBodyEditing({
    workId,
    role: comment.data?.role ?? '',
    open: comment.data?.id === openId ? comment.data : undefined,
    onMinted: setPickedId,
    failure: t('toast.versionSaveFailed'),
    current: false,
  })

  const role = comment.data?.role ?? comments[0]?.role ?? ''

  const pane = (
    <BodyPane
      label={t('versions.commentary')}
      who={
        <>
          {comments.length > 1 ? (
            <SegmentedControl
              aria-label={t('versions.commentary')}
              value={openId ?? undefined}
              onValueChange={(id) => {
                setPickedId(id)
                setReading('view')
              }}
            >
              {comments.map((entry) => (
                <Segment key={entry.id} value={entry.id}>
                  {labelOf(roles, entry.role)}
                </Segment>
              ))}
            </SegmentedControl>
          ) : (
            <span className="truncate font-mono text-xs text-faint">{labelOf(roles, role)}</span>
          )}
          <span className="truncate font-mono text-xs text-faint">
            {t('versions.revision', { number: revision })}
          </span>
        </>
      }
      markdown={roles.find((r) => r.key === role)?.body === 'markdown'}
      body={comment.data?.body ?? null}
      reading={reading}
      onReading={setReading}
      editing={editing}
      staged={staged}
      onStage={onStage}
    />
  )

  return staged ? <Stage onClose={() => onStage(false)}>{pane}</Stage> : pane
}
