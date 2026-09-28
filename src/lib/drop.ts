import { mediaKindOf } from '@/lib/media'

/*
 * Files dropped on the window from a folder.
 *
 * Tauri takes a drop from the desktop before the page sees it and hands it
 * over as one event for the whole window: the paths, and where the pointer
 * was. Which part of the screen it was meant for is the page's question, so
 * the answer is geometry - the pointer against a box - and the answer is
 * here, pure, where it is tested rather than tried by hand.
 */

/** The part of a box the question needs: `DOMRect` has these and more. */
interface Box {
  left: number
  top: number
  right: number
  bottom: number
}

/**
 * Whether a drop at `position` landed on `box`.
 *
 * The event reports the pointer in physical pixels and the page lays itself
 * out in CSS ones, so the position is divided by the window's scale first -
 * at 150% a drop on the right half of the screen would otherwise land on
 * nothing, or on the wrong thing. No box, no landing: an element that is not
 * on screen is not a target.
 */
export function landsOn(
  box: Box | null | undefined,
  position: { x: number; y: number },
  scale: number,
): boolean {
  if (box === null || box === undefined) return false
  const x = position.x / (scale || 1)
  const y = position.y / (scale || 1)
  return x >= box.left && x <= box.right && y >= box.top && y <= box.bottom
}

/**
 * The dropped paths split into the pictures and the rest. A drop from a
 * folder is often a selection of everything in it; the pictures are taken,
 * and the rest is counted so the person hears what was left behind.
 */
export function picturesAmong(paths: readonly string[]): { pictures: string[]; others: number } {
  const pictures = paths.filter((path) => mediaKindOf(path) === 'picture')
  return { pictures, others: paths.length - pictures.length }
}
