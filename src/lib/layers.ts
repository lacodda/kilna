/*
 * The stacking scale, as numbers this side of the stylesheet can do sums on.
 *
 * dowel states the ladder in CSS - `--z-modal: 60`, `--z-palette: 70` - and a
 * component reads it with `var(...)`, which is right and also unreadable to a
 * test. What matters is not any single value but the ORDER: a popup opened
 * inside a dialog has to come out above that dialog and below whatever is
 * meant to cover both. That is an arithmetic claim, and it is the one that
 * rots silently when someone renumbers the scale upstream.
 */

/** The rungs dowel defines, at the values its stylesheet gives them. Mirrored
 * rather than read: a test cannot evaluate a custom property, and a mirror
 * that has drifted is exactly what `floorsSitBetweenTheirNeighbours` catches. */
export const DOWEL_SCALE = {
  popup: 10,
  sticky: 20,
  menu: 30,
  floating: 40,
  overlay: 50,
  modal: 60,
  palette: 70,
  toast: 80,
} as const

/** The floors kilna threads between them, for a popup that belongs to an
 * overlay rather than to the page. */
export const POPUP_FLOORS = {
  'stage-popup': 51,
  'modal-popup': 61,
  'palette-popup': 71,
} as const

export type Rung = keyof typeof POPUP_FLOORS

/** Which rung of the ladder each floor has to clear, and which it must stay
 * under. A floor that escaped its band would cover something it does not
 * belong to - the bug this whole mechanism exists to avoid, in the other
 * direction. */
export const FLOOR_BANDS: Record<Rung, { above: number; below: number }> = {
  'stage-popup': { above: DOWEL_SCALE.overlay, below: DOWEL_SCALE.modal },
  'modal-popup': { above: DOWEL_SCALE.modal, below: DOWEL_SCALE.palette },
  'palette-popup': { above: DOWEL_SCALE.palette, below: DOWEL_SCALE.toast },
}

/** Where a full-screen stage sits: the version panel's focus mode covers the
 * page but is not a dialog, so it takes the overlay rung rather than a raw
 * Tailwind `z-50` beside the ladder. It had one, and the ± menu - a `--z-menu`
 * of 30 - opened underneath it, which read as a button that did nothing. */
export const STAGE_LAYER = DOWEL_SCALE.overlay
