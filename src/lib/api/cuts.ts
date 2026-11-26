import { invoke } from '@tauri-apps/api/core'
import type {
  Cut,
  CutPatch,
  DonorParts,
  JsonValue,
  NewCut,
  ShortStyle,
  ShortView,
  Shot,
} from '@/lib/api/types'

// A short cut out of another work: its stretches and the list for the cutter.

export const listCuts = (workId: string) => invoke<Cut[]>('list_cuts', { workId })
/** What has been taken out of a work: its stretches in every short cut from
    it, in the order they fall on it. */
export const listCutsFrom = (sourceId: string) => invoke<Cut[]>('list_cuts_from', { sourceId })
export const createCut = (cut: NewCut) => invoke<Cut>('create_cut', { cut })
export const updateCut = (id: string, patch: CutPatch) => invoke<Cut>('update_cut', { id, patch })
export const deleteCut = (id: string) => invoke<string>('delete_cut', { id })
/** Put a splice in the order given, 1..N, in one change. */
export const reorderCuts = (workId: string, ids: string[]) =>
  invoke<Cut[]>('reorder_cuts', { workId, ids })
/** The stretches in order, each beside the donor's file: what the cutting
    plugin is handed. The core never opens the file itself. */
export const cutShotList = (workId: string) => invoke<Shot[]>('cut_shot_list', { workId })
/** Where a short could be cut from, video by video: where its parts begin
    and the parts worth a short, as a plugin heard them and as the words place
    them, and what the short's siblings already took (v0.103). */
export const cutParts = (workId: string) => invoke<DonorParts[]>('cut_parts', { workId })

/** A short on its Build tab (v0.168): the look it wears, what it lacks, its
    words and its newest build. */
export const shortView = (workId: string) => invoke<ShortView>('short_view', { workId })
/** Lay the layers the window drew for a short where its builder is handed
    them: `placed` says what each is and where it stands. */
export const stageShortLayers = (
  workId: string,
  placed: JsonValue,
  layers: { name: string; bytes: number[] }[],
) => invoke<null>('stage_short_layers', { workId, placed, layers })
/** The style a space's shorts are built in while it has set none. */
export const shortStyleDefault = () => invoke<ShortStyle>('short_style_default')
