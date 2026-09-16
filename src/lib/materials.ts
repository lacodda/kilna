import type { Asset } from '@/lib/api'

/*
 * A work's files, in groups by what they are for.
 *
 * Until v0.69 this list was flat, and that was right while the only things
 * in it were a cover and a handful of references. Then scenes started keeping
 * their stills and their clips, and those are assets on the same work: a board
 * of fifty scenes with four candidates each puts two hundred pictures into a
 * list that used to hold three, and the cover is somewhere in the middle of
 * them. A flat list stopped being a list and became a haystack.
 *
 * So the files are grouped by kind. The order is the order the work happens
 * in rather than alphabetical or by count: the cover is what the work wears,
 * the stills and the clips are what a video is built from, and everything
 * else is reference. A kind with nothing in it is not shown at all - an empty
 * heading is furniture.
 */

/** The kinds this screen knows, in the order it shows them. Anything else is
 * gathered under `other`, so a kind invented later still appears. */
export const MATERIAL_ORDER = ['cover', 'frame', 'video'] as const

/** What a file with no kind, or an unknown one, is filed under. */
export const OTHER = 'other'

export interface MaterialGroup {
  /** The asset kind, or `other`. Also the key for the heading's wording. */
  kind: string
  assets: Asset[]
}

/**
 * The work's files in groups, in the order the screen shows them.
 *
 * Within a group the order the backend gave them is kept: it is the order
 * they arrived in, and re-sorting here would be a second opinion about a
 * sequence the board already has one about.
 */
export function groupMaterials(assets: Asset[]): MaterialGroup[] {
  const byKind = new Map<string, Asset[]>()
  for (const asset of assets) {
    const kind =
      asset.kind != null && (MATERIAL_ORDER as readonly string[]).includes(asset.kind)
        ? asset.kind
        : OTHER
    const list = byKind.get(kind)
    if (list) list.push(asset)
    else byKind.set(kind, [asset])
  }

  const groups: MaterialGroup[] = []
  for (const kind of MATERIAL_ORDER) {
    const assets = byKind.get(kind)
    if (assets !== undefined) groups.push({ kind, assets })
  }
  const rest = byKind.get(OTHER)
  if (rest !== undefined) groups.push({ kind: OTHER, assets: rest })
  return groups
}
