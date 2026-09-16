import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { updateWork } from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'

/**
 * The dial: "how far along is this".
 *
 * The fourth thing a work says about itself, and it overlaps none of the other
 * three. A status is derived from facts — it was scored, it was booked, it
 * shipped. A mark is a flag, on or off. The star is a bookmark. How finished
 * the work itself is, no fact can answer: only the author knows that a song
 * with a complete lyric is still three verses of placeholder.
 *
 * One mutation for the card and the catalogue row alike, written as the same
 * `work.update` a hand's edit is, so undo takes it back like any other edit.
 */
export function useStage(workId: string) {
  const { t } = useTranslation()
  const client = useQueryClient()

  return useMutation({
    // `null` is not "leave it": it takes the work back to unjudged, which is a
    // state of its own and the only way out of having answered.
    mutationFn: (percent: number | null) => updateWork(workId, { stage: percent }),
    onSuccess: (updated, percent) => {
      client.setQueryData(keys.work(workId), updated)
      announceEdited({
        client,
        message: percent === null ? t('toast.stageCleared') : t('toast.stageSet'),
        refresh: [keys.works, keys.catalogue, keys.journal],
      })
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })
}
