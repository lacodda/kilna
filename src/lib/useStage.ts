import { useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'

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

  return useAppMutation({
    // One of the five stops. There is no taking a stage back to nothing
    // (FR-03): a work always stands somewhere, and the first stop is the way
    // back to the start.
    mutationFn: (percent: number) => updateWork(workId, { stage: percent }),
    failure: 'toast.workSaveFailed',
    onSuccess: (updated) => {
      client.setQueryData(keys.work(workId), updated)
      announceEdited({ client, message: t('toast.stageSet'), refresh: refresh.work })
    },
  })
}
