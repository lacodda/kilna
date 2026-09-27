import { invoke } from '@tauri-apps/api/core'
import type { Undoable } from '@/lib/api/types'

// Taking back the last thing that changed the workspace.

/** What undo would take back right now, or null if nothing can be. */
export const lastUndoable = () => invoke<Undoable | null>('last_undoable')
/** Take back that operation. Fails if something else has happened since. */
export const undoLast = (operation: string) => invoke<Undoable>('undo_last', { operation })
