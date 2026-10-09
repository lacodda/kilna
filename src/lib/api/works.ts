import { invoke } from '@tauri-apps/api/core'
import type {
  BulkOutcome,
  CardCounts,
  Cloned,
  CoverView,
  FrameView,
  Discarded,
  Moment,
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
export const deleteWorks = (ids: string[]) => invoke<Discarded>('delete_works', { ids })
/** A second attempt at a video: the same donor and board, its own work. The
 * first is left exactly as it was — the two are meant to be compared. */
export const cloneWork = (workId: string, title: string) =>
  invoke<Cloned>('clone_work', { workId, title })
export const setWorksStatus = (workIds: string[], status: string) =>
  invoke<BulkOutcome>('set_works_status', { workIds, status })
/** Give the works that have no code in a numbered field the next codes, in
 * the order they were made (ADR 0059). */
export const numberWorks = (field: string, workIds: string[]) =>
  invoke<BulkOutcome>('number_works', { field, workIds })
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
/** A work's history on one axis: versions, scores, what was made from it and
 *  when it went out, and the journal's lines no row holds (ADR 0056). */
export const workTimeline = (workId: string) => invoke<Moment[]>('work_timeline', { workId })
/** The Cover tab: the prompt written from the work's cover for one of its
 *  doors' shapes, the scheme drawn from the same settings, and what the
 *  channel's card offers - written once, on the Rust side. */
export const coverView = (id: string, format: string | null) =>
  invoke<CoverView>('cover_view', { id, format })
/** The frame as it is copied into a generator: the still - built from the
 *  cover or written whole - the loop written from its settings, the
 *  negative, and the scheme of a built still. */
export const frameView = (id: string) => invoke<FrameView>('frame_view', { id })
