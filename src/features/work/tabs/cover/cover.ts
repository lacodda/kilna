import type { ProfileConfig, ReleaseKind, SceneBlock, Work } from '@/lib/api/types'
import { textOf } from '@/lib/json'
import { allOf, vocabularyOf } from '@/lib/useProfile'

/** A door a work goes out through that shows a picture, and its shape. */
export interface CoverFormat {
  door: ReleaseKind
  /** Width to height, as the profile writes it: `16:9`, `1:1`. */
  format: string
}

/**
 * The shapes a work's pictures take, one per kind of release its kind names
 * that states one, in the profile's order.
 *
 * The place decides the shape (v0.86): a video platform's preview is wide, a
 * streaming cover square. So the shape is read off the door, never off the
 * work - an audio release that goes out on both needs two pictures.
 */
export function coverFormatsOf(config: ProfileConfig, kind: string | undefined): CoverFormat[] {
  return vocabularyOf(config, kind).release_kinds.flatMap((door) => {
    const format = door.cover_format?.trim() ?? ''
    return format === '' ? [] : [{ door, format }]
  })
}

/**
 * The parts a work's cover prompt is written in.
 *
 * The kind's own, when it names them. A work can hold a prompt its kind no
 * longer names - its kind was changed, or the profile dropped the list - and
 * what was written must not vanish with the words for it: then the parts are
 * read in the words of any kind that has them, and a part none of them names
 * that holds words is shown under its own key. A kind that names none, on a
 * work that holds none, has no prompt at all.
 */
export function coverBlocksOf(
  config: ProfileConfig,
  work: Pick<Work, 'kind' | 'cover'>,
): SceneBlock[] {
  const own = vocabularyOf(config, work.kind).cover_blocks
  const held = Object.entries(work.cover)
    .filter(([, value]) => textOf(value).trim() !== '')
    .map(([key]) => key)
  // Nothing named and nothing written: a song whose cover is the album's.
  if (own.length === 0 && held.length === 0) return []
  const named = own.length > 0 ? own : allOf(config, 'cover_blocks')
  const strays = held
    .filter((key) => !named.some((block) => block.key === key))
    .map((key): SceneBlock => ({ key, label: key }))
  return [...named, ...strays]
}
