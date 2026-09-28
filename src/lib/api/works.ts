import { invoke } from '@tauri-apps/api/core'
import type {
  BulkOutcome,
  CardCounts,
  Cloned,
  NewWork,
  ScoredWork,
  StatusChange,
  Work,
  WorkFilter,
  WorkPatch,
} from '@/lib/api/types'

// Works: made, read, changed, cloned, and the catalogue that lists them.

export const listWorks = (filter?: WorkFilter) => invoke<Work[]>('list_works', { filter })
export const getWork = (id: string) => invoke<Work | null>('get_work', { id })
export const createWork = (work: NewWork) => invoke<Work>('create_work', { work })
export const updateWork = (id: string, patch: WorkPatch) =>
  invoke<Work>('update_work', { id, patch })
export const deleteWork = (id: string) => invoke<string>('delete_work', { id })
export const deleteWorks = (ids: string[]) => invoke<string[]>('delete_works', { ids })
/** A second attempt at a video: the same donor and board, its own work. The
 * first is left exactly as it was — the two are meant to be compared. */
export const cloneWork = (workId: string, title: string) =>
  invoke<Cloned>('clone_work', { workId, title })
export const setWorksStatus = (workIds: string[], status: string) =>
  invoke<BulkOutcome>('set_works_status', { workIds, status })
export const statusDrift = () => invoke<StatusChange[]>('status_drift')
export const resyncStatuses = () => invoke<StatusChange[]>('resync_statuses')
export const unpinStatus = (id: string) => invoke<Work>('unpin_status', { id })
/** Hold a work at a tier by hand, with the reason on record. */
export const pinTier = (id: string, tier: string, reason: string) =>
  invoke<Work>('pin_tier', { id, tier, reason })
/** Let the score speak for the work's tier again. */
export const unpinTier = (id: string) => invoke<Work>('unpin_tier', { id })
/** Tags in use on works, most used first — what the tag box offers. */
export const workTags = () => invoke<[string, number][]>('work_tags')
export const catalogue = () => invoke<ScoredWork[]>('catalogue')
/** The number beside each of a work's tabs, in one answer. */
export const cardCounts = (workId: string) => invoke<CardCounts>('card_counts', { workId })
