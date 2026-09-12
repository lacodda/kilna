/**
 * Seconds as a person types them on a storyboard: `1:23`, `0:04.5`, `83`.
 *
 * The scene stores plain seconds — sums and gaps (v0.67) are arithmetic on
 * numbers — and the screen shows minutes and seconds, because "1:23" is how
 * anyone reads a timeline and "83" is not. Both directions live here so the
 * field cannot show one thing and store another.
 */

/** What a typed timecode means: seconds, nothing, or not a timecode. */
export type Parsed = { ok: true; seconds: number | null } | { ok: false }

/**
 * Read a timecode. Empty is "unset"; `ss`, `m:ss` and `h:mm:ss` are read,
 * with a decimal fraction on the last part. Anything else — letters, a
 * negative, a minute of sixty seconds — is refused rather than guessed.
 */
export function parseTimecode(text: string): Parsed {
  const trimmed = text.trim()
  if (trimmed === '') return { ok: true, seconds: null }

  const parts = trimmed.split(':')
  if (parts.length > 3) return { ok: false }

  let seconds = 0
  for (const [index, part] of parts.entries()) {
    const last = index === parts.length - 1
    // Only the last part may carry a fraction; the others are whole units.
    if (!(last ? /^\d+(\.\d+)?$/ : /^\d+$/).test(part)) return { ok: false }
    const value = Number(part)
    // A minute has sixty seconds and an hour sixty minutes: "1:75" is a typo,
    // not seventy-five seconds.
    if (index > 0 && value >= 60) return { ok: false }
    seconds = seconds * 60 + value
  }
  return { ok: true, seconds }
}

/**
 * Seconds as `m:ss`, with the fraction kept to one place when there is one:
 * `83` → `1:23`, `4.5` → `0:04.5`, `3600` → `60:00`. Hours are not split
 * out — a video is minutes long, and `60:00` reads better on a board than
 * `1:00:00` beside `0:04`.
 */
export function formatSeconds(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return ''
  const minutes = Math.floor(seconds / 60)
  const rest = seconds - minutes * 60
  // Rounded to a tenth before splitting, so 59.96 does not read as 0:60.0.
  const tenths = Math.round(rest * 10)
  if (tenths >= 600) return formatSeconds((minutes + 1) * 60)
  const whole = Math.floor(tenths / 10)
  const fraction = tenths % 10
  const padded = String(whole).padStart(2, '0')
  return fraction === 0 ? `${minutes}:${padded}` : `${minutes}:${padded}.${fraction}`
}
