import { invoke } from '@tauri-apps/api/core'
import type { MediaBatchReport, MediaMatch, MediaPick } from '@/lib/api/types'
import { bytesOf } from '@/lib/compose'

// A batch of files sorted into their works (v0.157, FR-21 21.2).

/** Match files - or the files of a folder - against a collection's works,
 *  or every work of the space. */
export const matchMedia = (paths: readonly string[], collectionId: string | null) =>
  invoke<MediaMatch[]>('match_media', { paths, collectionId })

/** A picture of the batch, for its row. */
export async function mediaPreview(path: string): Promise<ArrayBuffer> {
  return bytesOf(await invoke<ArrayBuffer | number[]>('media_preview', { path }))
}

/** Take the confirmed files into their works; let the others go to the
 *  recycle bin. */
export const takeMediaBatch = (picks: readonly MediaPick[], trash: readonly string[]) =>
  invoke<MediaBatchReport>('take_media_batch', { picks, trash })
