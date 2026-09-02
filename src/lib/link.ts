import { openUrl } from '@tauri-apps/plugin-opener'

/**
 * Opening a link the way a desktop app has to.
 *
 * `<a target="_blank">` does nothing useful inside a WebView: at best the click
 * is swallowed, at worst the page loads over the app itself, with no back
 * button and no way out. Three places shipped such a link before v0.45 - the
 * releases tab, the calendar dialog and the chip's preview card - and none of
 * them reached a browser.
 *
 * Anything that is not plainly `http(s)` is refused rather than handed to the
 * shell. A release url is typed by a person, and `file:` or a scheme belonging
 * to another installed program is not something a note about a publication
 * should be able to launch.
 */
export async function openExternal(url: string): Promise<boolean> {
  if (!isWebLink(url)) return false

  try {
    await openUrl(url)
    return true
  } catch {
    return false
  }
}

/** Whether this is a plain web address, and so safe to hand to the browser. */
export function isWebLink(url: string): boolean {
  let parsed: URL

  try {
    parsed = new URL(url.trim())
  } catch {
    return false
  }

  return parsed.protocol === 'https:' || parsed.protocol === 'http:'
}

/**
 * A link as something a row can hold: the host, and enough of the path to tell
 * two links on the same host apart.
 *
 * A release url is often long enough to push everything else off the row, and
 * the part a person recognises is at the front. The full address stays in the
 * title attribute, and copying still copies the whole thing.
 */
export function shortLink(url: string, budget = 32): string {
  let parsed: URL

  try {
    parsed = new URL(url.trim())
  } catch {
    // Not a url at all: show it as typed, trimmed to the same budget, rather
    // than hiding what was actually recorded.
    return clip(url.trim(), budget)
  }

  const host = parsed.host.replace(/^www\./, '')
  const tail = `${parsed.pathname}${parsed.search}`.replace(/\/$/, '')

  if (tail === '' || tail === '/') return host

  return clip(`${host}${tail}`, budget)
}

function clip(value: string, budget: number): string {
  if (value.length <= budget) return value

  // The ellipsis is part of the budget, so the result never exceeds it.
  return `${value.slice(0, Math.max(1, budget - 1))}…`
}
