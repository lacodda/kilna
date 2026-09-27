import { invoke } from '@tauri-apps/api/core'
import type { Cut, CutPatch, NewCut, Shot } from '@/lib/api/types'

// A short cut out of another work: its stretches and the list for the cutter.

export const listCuts = (workId: string) => invoke<Cut[]>('list_cuts', { workId })
export const createCut = (cut: NewCut) => invoke<Cut>('create_cut', { cut })
export const updateCut = (id: string, patch: CutPatch) => invoke<Cut>('update_cut', { id, patch })
export const deleteCut = (id: string) => invoke<string>('delete_cut', { id })
/** Put a splice in the order given, 1..N, in one change. */
export const reorderCuts = (workId: string, ids: string[]) =>
  invoke<Cut[]>('reorder_cuts', { workId, ids })
/** The stretches in order, each beside the donor's file: what the cutting
    plugin is handed. The core never opens the file itself. */
export const cutShotList = (workId: string) => invoke<Shot[]>('cut_shot_list', { workId })
