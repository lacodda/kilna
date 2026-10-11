import { invoke } from '@tauri-apps/api/core'
import type { NewTimeOff, TimeOff, TimeOffPatch } from '@/lib/api/types'

// The calendar's closed days: spans nothing should go out on.

export const listTimeOff = () => invoke<TimeOff[]>('list_time_off')
export const createTimeOff = (timeOff: NewTimeOff) =>
  invoke<TimeOff>('create_time_off', { timeOff })
export const updateTimeOff = (id: string, patch: TimeOffPatch) =>
  invoke<TimeOff>('update_time_off', { id, patch })
export const deleteTimeOff = (id: string) => invoke<null>('delete_time_off', { id })
