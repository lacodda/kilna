import { invoke } from '@tauri-apps/api/core'
import type { Deletion } from '@/lib/api/types'

// The trash: nothing leaves for good until it is emptied.

export const listDeletions = () => invoke<Deletion[]>('list_deletions')
export const restoreDeletion = (id: string) => invoke<void>('restore_deletion', { id })
export const purgeDeletion = (id: string) => invoke<void>('purge_deletion', { id })
export const emptyTrash = () => invoke<number>('empty_trash')
