/**
 * The frame of a short: where each thing stands on 1080x1920.
 *
 * The same for every short of every space - what differs is what is drawn in
 * it (the style) and how it is worn (the variant). kilna-plugin-cut lays the
 * moving parts out by the same numbers; a number changed here is changed
 * there.
 */
export const FRAME = {
  width: 1080,
  height: 1920,
  /** The title's top, its largest size and the width it is fitted to. */
  title: { top: 96, size: 80, fit: 1000 },
  /** The call's top and size. */
  call: { top: 200, size: 54 },
  /** The stamp's middle, down the frame; the width it is drawn at. */
  stamp: { middle: 486, width: 650 },
  /** The picture: full width, 16:9. */
  picture: { top: 536, height: 608 },
  /** The strip that fills as the short plays. */
  strip: { top: 1144, height: 8 },
  /** The band the sound is drawn in. */
  wave: { top: 1166, height: 220 },
  /** Where the words stand. */
  words: { top: 1412 },
  /** The square the sign lives in. */
  sign: { top: 1505, box: 420 },
} as const

/** The side of a blot's own canvas, and where its heart is on it. */
export const BLOT = { side: 900, heartX: 450, heartY: 400 } as const
