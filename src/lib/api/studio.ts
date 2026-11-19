import { invoke } from '@tauri-apps/api/core'
import type { StudioDone, StudioPack } from '@/lib/api/types'

// The YouTube Studio pack (v0.159, FR-22 22.1): a batch put out by hand.

/** The steps of a pack: these works' releases on `door` still to go out
 *  with their video made. */
export const studioPack = (workIds: readonly string[], door: string) =>
  invoke<StudioPack>('studio_pack', { workIds, door })

/** Keep the link the platform gave back: out on its day, booked before. */
export const studioLink = (releaseId: string, url: string) =>
  invoke<StudioDone>('studio_link', { releaseId, url })

/** Whether a link is a video's on the platform - the same rule the backend
 *  holds it to, said before the press. */
export function isVideoLink(link: string): boolean {
  return /^(?:https?:\/\/)?(?:www\.|m\.)?(?:youtu\.be\/|youtube\.com\/watch\?v=|youtube\.com\/shorts\/)[\w-]{11}(?![\w-])/.test(
    link.trim(),
  )
}
