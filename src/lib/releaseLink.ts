import type { ReleaseKind } from '@/lib/api/types'

/*
 * What a pasted link says about where and when a release went out (v0.97).
 *
 * Where: each door of a kind names what its addresses look like (`links` in
 * the profile - `youtube.com/shorts/`, `open.spotify.com`), because the code
 * is not allowed to know which places exist (ADR 0001). When: some addresses
 * carry their day in the path - `/2026/10/14/slug`, `/2026-10-14-slug` - and
 * a blog post's link is the most exact record of its day there is. Neither
 * is fetched: nothing here goes on the network.
 */

/** What a link gave away; null where it said nothing. */
export interface LinkReading {
  /** The door of the work's kind the link belongs to. */
  kind: string | null
  /** The day in its path, `yyyy-mm-dd`. */
  day: string | null
}

/**
 * Read a link against the doors of one kind of work.
 *
 * The most specific pattern wins - a longer host, then a longer path - so
 * `music.youtube.com` beats `youtube.com/watch` for a streaming link, and
 * `youtube.com/shorts/` beats `youtube.com`. Two doors matching equally well
 * (a premiere and an upload are both `youtube.com/watch`) is no answer: a
 * guess between them would be wrong half the time, and silently.
 */
export function readLink(url: string, doors: readonly ReleaseKind[]): LinkReading {
  let parsed: URL
  try {
    parsed = new URL(url.trim())
  } catch {
    return { kind: null, day: null }
  }
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    return { kind: null, day: null }
  }
  const host = parsed.hostname.toLowerCase().replace(/^www\./, '')
  const path = parsed.pathname

  let best: { kind: string; weight: [number, number] } | null = null
  let tied = false
  for (const door of doors) {
    for (const pattern of door.links ?? []) {
      const weight = matches(pattern, host, path)
      if (weight === null) continue
      const better =
        best === null ||
        weight[0] > best.weight[0] ||
        (weight[0] === best.weight[0] && weight[1] > best.weight[1])
      const same = best !== null && weight[0] === best.weight[0] && weight[1] === best.weight[1]
      if (better) {
        best = { kind: door.key, weight }
        tied = false
      } else if (same && best !== null && best.kind !== door.key) {
        tied = true
      }
    }
  }

  return { kind: best === null || tied ? null : best.kind, day: dayIn(path) }
}

/**
 * How well one pattern holds a link: the length of its host and of its
 * path, or null when it does not hold it. A host holds itself and its
 * subdomains - `youtube.com` holds `m.youtube.com` - and never a host that
 * only ends with the same letters: `notyoutube.com` is not YouTube.
 */
function matches(pattern: string, host: string, path: string): [number, number] | null {
  const slash = pattern.indexOf('/')
  const wantHost = (slash === -1 ? pattern : pattern.slice(0, slash)).toLowerCase()
  const wantPath = slash === -1 ? '' : pattern.slice(slash)
  const hostHolds = host === wantHost || host.endsWith(`.${wantHost}`)
  if (!hostHolds || !path.startsWith(wantPath)) return null
  return [wantHost.length, wantPath.length]
}

/** A real calendar day in a path: `/2026/10/14/` or `2026-10-14`, read
 *  whole - a 31st of February is not a day, and is not read as one. */
function dayIn(path: string): string | null {
  const found =
    /(?:^|\/)(\d{4})\/(\d{2})\/(\d{2})(?:\/|$)/.exec(path) ??
    /(?:^|[/_-])(\d{4})-(\d{2})-(\d{2})(?=$|[/_.-])/.exec(path)
  if (found === null) return null
  const [, year, month, day] = found
  const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)))
  const real =
    date.getUTCFullYear() === Number(year) &&
    date.getUTCMonth() === Number(month) - 1 &&
    date.getUTCDate() === Number(day)
  return real ? `${year}-${month}-${day}` : null
}
