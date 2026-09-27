import { invoke } from '@tauri-apps/api/core'
import type {
  BulkOutcome,
  GeneratedBatch,
  GeneratedFields,
  NewRelease,
  Placement,
  Release,
  ReleaseFieldValue,
  ReleasePatch,
  ScheduledRelease,
  Scheduling,
  SlotPreview,
} from '@/lib/api/types'

// Releases: what goes out, where and when, and what it goes out as.

export const createRelease = (release: NewRelease) => invoke<Release>('create_release', { release })
export const deleteRelease = (id: string) => invoke<string>('delete_release', { id })
export const updateRelease = (id: string, patch: ReleasePatch) =>
  invoke<Release>('update_release', { id, patch })
export const scheduleRelease = (id: string, slot: string) =>
  invoke<Scheduling>('schedule_release', { id, slot })
export const previewSchedule = (id: string, slot: string) =>
  invoke<SlotPreview>('preview_schedule', { id, slot })
// `today` is the user's local date: the backend only knows UTC, which at a
// negative offset is already tomorrow. Returns how many gaps are standing.
export const warnUnreadyReleases = (today: string) =>
  invoke<number>('warn_unready_releases', { today })
export const setSlotPin = (id: string, pinned: boolean) =>
  invoke<Release>('set_slot_pin', { id, pinned })
export const unscheduleRelease = (id: string) => invoke<Release>('unschedule_release', { id })
export const unscheduleWorks = (workIds: string[]) =>
  invoke<BulkOutcome>('unschedule_works', { workIds })
// `at` is the day it went out, when that is not today: a release marked late,
// or one whose real date is known from elsewhere. Left out, the moment is now.
export const markReleased = (id: string, url?: string | null, at?: string | null) =>
  invoke<Release>('mark_released', { id, url, at })
// Undoing the mark. The link is kept - see the Rust side for why.
export const unmarkReleased = (id: string) => invoke<Release>('unmark_released', { id })
export const planLayout = (today: string) => invoke<Placement[]>('plan_layout', { today })
export const applyLayout = (placements: Placement[]) =>
  invoke<number>('apply_layout', { placements })
export const releaseFields = (id: string) => invoke<ReleaseFieldValue[]>('release_fields', { id })
// Keys the profile does not declare are refused: the map is open on purpose,
// but a typed key no field names could only come from a bug.
export const setReleaseFields = (id: string, values: Record<string, string>) =>
  invoke<Release>('set_release_fields', { id, values })
export const generateReleaseFields = (id: string) =>
  invoke<GeneratedFields>('generate_release_fields', { id })
export const generateReleaseFieldsBatch = (ids: string[]) =>
  invoke<GeneratedBatch>('generate_release_fields_batch', { ids })
export const calendar = () => invoke<ScheduledRelease[]>('calendar')
export const releaseQueue = () => invoke<ScheduledRelease[]>('release_queue')
export const releasesForWork = (workId: string) =>
  invoke<ScheduledRelease[]>('releases_for_work', { workId })
