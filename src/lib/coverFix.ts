import type { ContractFix, Cover, CoverBricks, Frame as WorkFrame } from '@/lib/api/types'

/*
 * The one change that lets a brick its contract left out into the picture
 * (v0.134, FR-07 7.2): the backend says which (`ContractFix`), the window
 * makes it - on the cover, or on the frame's own parts over its cover.
 */

/** The bricks with one place emptied. */
function without(bricks: CoverBricks, place: string): CoverBricks {
  return { ...bricks, [place]: null }
}

/** The cover with `fix` made. A moving camera is a frame's, not a cover's. */
export function fixCover(cover: Cover, fix: ContractFix): Cover {
  switch (fix.kind) {
    case 'crop':
      return cover.framing === null
        ? cover
        : { ...cover, framing: { ...cover.framing, crop: fix.crop } }
    case 'subject':
      // A picture of a thing or of nobody has no hero.
      return {
        ...cover,
        hero: fix.subject === 'person' ? cover.hero : null,
        staging: { ...cover.staging, subject: fix.subject },
      }
    case 'ground':
      return { ...cover, staging: { ...cover.staging, ground: fix.ground } }
    case 'movingCamera':
      return cover
    case 'remove':
      return { ...cover, bricks: without(cover.bricks, fix.place) }
  }
}

/** What changes on a frame to make `fix`, over the cover it is built from. */
export function fixFrame(frame: WorkFrame, cover: Cover, fix: ContractFix): Partial<WorkFrame> {
  switch (fix.kind) {
    case 'crop': {
      const framing = frame.framing ?? cover.framing
      return framing === null ? {} : { framing: { ...framing, crop: fix.crop } }
    }
    case 'subject':
      return fix.subject === 'person'
        ? { subject: fix.subject }
        : { subject: fix.subject, hero: null }
    case 'ground':
      return { ground: fix.ground }
    case 'movingCamera':
      return { still_camera: false }
    case 'remove':
      return fix.place === 'motion'
        ? { motion_bricks: (frame.motion_bricks ?? []).filter((id) => id !== fix.id) }
        : { bricks: without(frame.bricks ?? cover.bricks, fix.place) }
  }
}
