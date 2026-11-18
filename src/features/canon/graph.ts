/*
 * The canon drawn as a graph (v0.143, FR-18 18.4): where the cards stand and
 * what their nodes wear. Plain functions with no randomness - the same cards
 * and relations land in the same places on every render, so a person who
 * learned where a hero stands finds them there again, and a test can say so.
 */

/** A point on the drawing. */
export interface Spot {
  x: number
  y: number
}

/** Around one card: `count` others on a ring, evenly, the first at the top. */
export function ring(count: number, radius: number, centre: Spot): Spot[] {
  return Array.from({ length: count }, (_, at) => {
    const angle = -Math.PI / 2 + (2 * Math.PI * at) / Math.max(1, count)
    return { x: centre.x + radius * Math.cos(angle), y: centre.y + radius * Math.sin(angle) }
  })
}

/**
 * Where every card of the canon stands: a force layout - related cards pull
 * together, every card pushes the others away - run a fixed number of steps
 * from a fixed start, a circle in the cards' order, and kept inside the
 * drawing with a margin.
 */
export function forceLayout(
  ids: readonly string[],
  links: readonly { from: string; to: string }[],
  size: { width: number; height: number },
  steps = 240,
): Map<string, Spot> {
  const margin = 40
  const placed = new Map<string, Spot>()
  if (ids.length === 0) return placed
  const centre = { x: size.width / 2, y: size.height / 2 }
  const start = ring(ids.length, Math.min(size.width, size.height) / 3, centre)
  ids.forEach((id, at) => placed.set(id, { ...start[at]! }))
  if (ids.length === 1) {
    placed.set(ids[0]!, centre)
    return placed
  }
  const known = new Set(ids)
  const edges = links.filter((link) => known.has(link.from) && known.has(link.to))
  const area = (size.width - 2 * margin) * (size.height - 2 * margin)
  const k = Math.sqrt(area / ids.length)
  let heat = size.width / 10
  for (let step = 0; step < steps; step++) {
    const push = new Map(ids.map((id) => [id, { x: 0, y: 0 }]))
    for (let a = 0; a < ids.length; a++) {
      for (let b = a + 1; b < ids.length; b++) {
        const one = placed.get(ids[a]!)!
        const other = placed.get(ids[b]!)!
        const dx = one.x - other.x || 0.01
        const dy = one.y - other.y || 0.01
        const distance = Math.max(0.01, Math.hypot(dx, dy))
        const force = (k * k) / distance
        const fx = (dx / distance) * force
        const fy = (dy / distance) * force
        push.get(ids[a]!)!.x += fx
        push.get(ids[a]!)!.y += fy
        push.get(ids[b]!)!.x -= fx
        push.get(ids[b]!)!.y -= fy
      }
    }
    for (const edge of edges) {
      const one = placed.get(edge.from)!
      const other = placed.get(edge.to)!
      const dx = one.x - other.x
      const dy = one.y - other.y
      const distance = Math.max(0.01, Math.hypot(dx, dy))
      const force = (distance * distance) / k
      const fx = (dx / distance) * force
      const fy = (dy / distance) * force
      push.get(edge.from)!.x -= fx
      push.get(edge.from)!.y -= fy
      push.get(edge.to)!.x += fx
      push.get(edge.to)!.y += fy
    }
    for (const id of ids) {
      const spot = placed.get(id)!
      const move = push.get(id)!
      const length = Math.max(0.01, Math.hypot(move.x, move.y))
      const by = Math.min(length, heat)
      spot.x = Math.min(size.width - margin, Math.max(margin, spot.x + (move.x / length) * by))
      spot.y = Math.min(size.height - margin, Math.max(margin, spot.y + (move.y / length) * by))
    }
    heat = Math.max(0.5, heat * 0.97)
  }
  return placed
}

/** A label cut to fit beside a line: the whole of it is in the tooltip. */
export function clipped(text: string, most = 14): string {
  return text.length > most ? `${text.slice(0, most - 1)}…` : text
}

/** The palette roles a node may wear, one per kind of card in the
 *  profile's order, round again after the last. */
const TONES = [
  'fill-accent-soft stroke-accent',
  'fill-info-soft stroke-info',
  'fill-good-soft stroke-good',
  'fill-warn-soft stroke-warn',
  'fill-bad-soft stroke-bad',
] as const

/** What a node of this kind wears: its fill and its ring. */
export function toneOf(kind: string, kinds: readonly string[]): string {
  const at = kinds.indexOf(kind)
  return at < 0 ? 'fill-soft stroke-line-2' : TONES[at % TONES.length]!
}
