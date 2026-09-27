import { useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'

/**
 * The star: "come back to this one".
 *
 * It is the bookmark the schema has carried since v0.12 (ADR 0013) — not a
 * status, because it derives nothing; not a mark, because it is not a word
 * from the profile; not a pin. One toggle for the header and the catalogue
 * row alike, written as the same `work.update` a hand's edit is, so undo
 * takes it back like any other edit.
 */
export function useStar(workId: string) {
  const { t } = useTranslation()
  const client = useQueryClient()

  return useAppMutation({
    mutationFn: (on: boolean) => updateWork(workId, { bookmarked: on }),
    failure: 'toast.workSaveFailed',
    onSuccess: (updated, on) => {
      client.setQueryData(keys.work(workId), updated)
      announceEdited({
        client,
        message: t(on ? 'toast.starred' : 'toast.unstarred'),
        refresh: refresh.work,
      })
    },
  })
}
