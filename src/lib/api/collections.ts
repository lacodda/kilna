import { invoke } from '@tauri-apps/api/core'
import type { BulkOutcome, Collection, CollectionPatch, NewCollection } from '@/lib/api/types'

// Collections: albums, books, seasons - one level, never a collection of collections.

export const listCollections = () => invoke<Collection[]>('list_collections')
export const createCollection = (collection: NewCollection) =>
  invoke<Collection>('create_collection', { collection })
export const updateCollection = (id: string, patch: CollectionPatch) =>
  invoke<Collection>('update_collection', { id, patch })
/** Into the trash; its works stay, loose. Answers the trash entry's id. */
export const deleteCollection = (id: string) => invoke<string>('delete_collection', { id })
/** The whole list as it is to stand, in order: works left out leave it. */
export const setCollectionContents = (id: string, workIds: string[]) =>
  invoke<void>('set_collection_contents', { id, workIds })
/** Works put at the end, each taken out of whatever collection held it. */
export const addToCollection = (id: string, workIds: string[]) =>
  invoke<BulkOutcome>('add_to_collection', { id, workIds })
