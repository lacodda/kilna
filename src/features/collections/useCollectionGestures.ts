import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { addToCollection, createCollection, setCollectionContents } from '@/lib/api/collections'
import type { Collection } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'

/**
 * The writes that change what a collection holds, the way every door makes
 * them: the catalogue's bar, a row dragged onto a collection, a work's own
 * header, the collection's page.
 *
 * One hook rather than a mutation at each door, because the bargain is the
 * same at all of them - the change lands, one line says what it did with a
 * way to take it back, and whatever was passed over is named. Four copies of
 * that would come to disagree about the sentence or forget the undo.
 */
export function useCollectionGestures() {
  const { t } = useTranslation()
  const client = useQueryClient()

  const added = (collection: Collection, count: number) =>
    announceEdited({
      client,
      message: t('collections.added', { count, title: collection.title }),
      refresh: refresh.collection,
    })

  /** Put works at the end of a collection. */
  const add = useAppMutation({
    mutationFn: ({ collection, workIds }: { collection: Collection; workIds: string[] }) =>
      addToCollection(collection.id, workIds),
    failure: 'toast.collectionSaveFailed',
    refresh: refresh.collection,
    onSuccess: (outcome, { collection }) => {
      if (outcome.changed > 0) added(collection, outcome.changed)
      say.skipped(outcome.skipped)
    },
  })

  /**
   * Set the whole list: a drag, a removal. `message` is what the person did,
   * in their words - the order changed, a work taken out - since the list
   * alone does not say which.
   */
  const arrange = useAppMutation({
    mutationFn: ({
      collection,
      workIds,
    }: {
      collection: Collection
      workIds: string[]
      message: string
    }) => setCollectionContents(collection.id, workIds),
    failure: 'toast.collectionSaveFailed',
    refresh: refresh.collection,
    onSuccess: (_, { message }) => announceEdited({ client, message, refresh: refresh.collection }),
  })

  /**
   * Make a collection, and fill it with `workIds` when there are any - the
   * "new collection" at the end of every list of collections to put works
   * in. Two writes, so two steps back: the filling first, then the making.
   */
  const make = useAppMutation({
    mutationFn: async ({
      title,
      kind,
      workIds,
    }: {
      title: string
      kind: string
      workIds: string[]
    }) => {
      const made = await createCollection({ title, kind })
      const outcome = workIds.length > 0 ? await addToCollection(made.id, workIds) : null
      return { made, outcome }
    },
    failure: 'toast.collectionSaveFailed',
    refresh: refresh.collection,
    onSuccess: ({ made, outcome }) => {
      if (outcome !== null && outcome.changed > 0) added(made, outcome.changed)
      else say.ok(t('collections.created', { title: made.title }))
      if (outcome !== null) say.skipped(outcome.skipped)
    },
  })

  return { add, arrange, make }
}
