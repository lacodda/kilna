/**
 * A work's cover, until it has a real one.
 *
 * Every work gets a gradient derived from its id, so a card is recognisable at a
 * glance and stays the same colour for as long as the work exists. Deriving it
 * rather than storing it means nothing has to be migrated when real covers
 * arrive in v0.36 — the gradient simply becomes the fallback.
 *
 * The palette is the mockup's: seven pairs, each a saturated hue falling into a
 * dark violet, so a row of covers reads as one family rather than a paint chart.
 */
const GRADIENTS: readonly (readonly [string, string])[] = [
  ['#8a2f68', '#2b1f4e'],
  ['#b4562e', '#4e1f35'],
  ['#2e7a8c', '#1f2b4e'],
  ['#5f6672', '#23202b'],
  ['#3f8a5e', '#1f3b4e'],
  ['#9e3f8a', '#1f2140'],
  ['#7a5abf', '#20163a'],
]

/**
 * Stable index for an id.
 *
 * A plain multiply-and-add hash: ids are uuids, so there is nothing to defend
 * against here beyond giving neighbouring works different colours.
 */
function pick(id: string): number {
  let hash = 0
  for (let index = 0; index < id.length; index += 1) {
    hash = (hash * 31 + id.charCodeAt(index)) >>> 0
  }
  return hash % GRADIENTS.length
}

/** `background` value for a work's cover. */
export function coverFor(id: string): string {
  const [from, to] = GRADIENTS[pick(id)] ?? GRADIENTS[0]!
  return `linear-gradient(135deg, ${from}, ${to} 70%)`
}
