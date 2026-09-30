import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { saveRelease } from '@/lib/api/releases'
import type { ScheduledRelease } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { changesOf, type ReleaseChanges, type ReleaseDraft } from '@/lib/releaseForm'
import { ReleaseForm, useReleaseDraft } from '@/components/ReleaseForm'
import { ReleaseAssets } from '@/features/work/tabs/releases/ReleaseAssets'
import { ReleaseFields } from '@/features/work/tabs/releases/ReleaseFields'
import { ReleaseProposals } from '@/features/work/tabs/releases/ReleaseProposals'

interface Props {
  release: ScheduledRelease
  /** The query areas an edit of this release disturbs. */
  refreshed: readonly (readonly unknown[])[]
}

/**
 * A release's row, unrolled: its date, its link and its kind, what it goes
 * out as and what is proposed it should, and the files that go with it.
 *
 * In place, under the row, rather than in a dialog. Until v0.80 the date and
 * the link were a dialog away and the fields a box nested under the row, so
 * scheduling a release was three clicks and its text was read in a second
 * frame. Here everything about one release is in one block, and the form is
 * the calendar's own (`components/ReleaseForm`).
 *
 * The form saves as it goes, the way the rest of the card does: a kind or a
 * day saves when it is picked, the link when it is left. Each save is one
 * edit, offered back by the toast that says it was made.
 */
export function ReleaseDetails({ release, refreshed }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [draft, setDraft] = useReleaseDraft(release)

  const save = useAppMutation({
    mutationFn: (changes: ReleaseChanges) => saveRelease(release.id, changes),
    failure: 'toast.releaseSaveFailed',
    onSuccess: () =>
      announceEdited({ client, message: t('toast.releaseEdited'), refresh: refreshed }),
  })

  const settle = (next: ReleaseDraft) => {
    const changes = changesOf(release, next)
    if (changes !== null) save.mutate(changes)
  }

  return (
    // The mockup's fields sit at 31 pixels; the compact row is the scale's
    // nearest, and reaches every control in the block without a size each.
    <div
      data-density="compact"
      className="flex flex-col gap-4 border-t border-line bg-softer px-3.5 py-3"
    >
      {draft !== null && (
        <ReleaseForm
          release={release}
          value={draft}
          onChange={setDraft}
          onSettle={settle}
          layout="row"
        />
      )}
      <ReleaseFields release={release} />
      {/* What the assistant or an agent proposes the fields say, under the
          fields it would change. */}
      <ReleaseProposals release={release} />
      <ReleaseAssets release={release} />
    </div>
  )
}
