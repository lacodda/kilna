import { invoke } from '@tauri-apps/api/core'
import type {
  Dismissal,
  DismissalKey,
  FocusNote,
  FocusNotePatch,
  NewFocusNote,
} from '@/lib/api/types'

// The dashboard's board: findings set aside and notes pinned to it.

export const dismissedFindings = () => invoke<Dismissal[]>('dismissed_findings')
export const dismissFinding = (key: DismissalKey) => invoke<Dismissal>('dismiss_finding', { key })
export const restoreFinding = (key: DismissalKey) => invoke<void>('restore_finding', { key })
export const listFocusNotes = () => invoke<FocusNote[]>('list_focus_notes')
export const createFocusNote = (note: NewFocusNote) =>
  invoke<FocusNote>('create_focus_note', { note })
export const updateFocusNote = (id: string, patch: FocusNotePatch) =>
  invoke<FocusNote>('update_focus_note', { id, patch })
export const reorderFocusNotes = (order: string[]) => invoke<void>('reorder_focus_notes', { order })
export const deleteFocusNote = (id: string) => invoke<void>('delete_focus_note', { id })
