import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { Meta, Work, WorkPatch } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { say } from '@/lib/toast'

/**
 * A change to the work made on its overview: a field, the status, the kind.
 *
 * Optimistic, because the header above shows the same values and should not
 * lag behind the field just left; a refusal puts back what was there and the
 * toast says why it moved.
 *
 * A field is written alone: `{ meta: { bpm: 92 } }`, and `null` to empty one.
 * The backend merges fields by key since v0.82, so an edit of the tempo can no
 * longer carry a stale copy of the key signature back over a change made to it
 * a moment ago - by a plugin, by an undo - and undoing it takes back the tempo
 * alone. Until v0.82 the overview sent every field to change one.
 */
export function useWorkEdit(work: Work) {
  const { t } = useTranslation()
  const client = useQueryClient()

  const patch = useMutation({
    mutationFn: (changes: WorkPatch) => updateWork(work.id, changes),
    onMutate: async (changes) => {
      await client.cancelQueries({ queryKey: keys.work(work.id) })
      const previous = client.getQueryData<Work | null>(keys.work(work.id))

      if (previous != null) {
        const { meta: fields, ...rest } = changes
        client.setQueryData<Work>(keys.work(work.id), {
          ...previous,
          ...rest,
          meta: fields === undefined ? previous.meta : merged(previous.meta, fields),
        })
      }
      return { previous }
    },
    onError: (cause, _changes, context) => {
      if (context?.previous !== undefined) {
        client.setQueryData(keys.work(work.id), context.previous)
      }
      say.failedTo(t('toast.workSaveFailed'), cause)
    },
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.workEdited'),
        refresh: [keys.works, keys.catalogue, keys.journal],
      })
    },
  })

  /** Write one field; nothing, an empty word or `undefined`, empties it. */
  const setField = (key: string, value: Meta[string]) => {
    const empty = value === '' || value === undefined || value === null
    patch.mutate({ meta: { [key]: empty ? null : value } })
  }

  return { patch, setField }
}

/** The fields after a patch, the way the backend merges them. */
function merged(meta: Meta, fields: Meta): Meta {
  const next: Meta = { ...meta }
  for (const [key, value] of Object.entries(fields)) {
    if (value === null) delete next[key]
    else next[key] = value
  }
  return next
}
