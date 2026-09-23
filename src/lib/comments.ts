import type { Comment } from '@/lib/api'

/**
 * Where a comment stands, as a person reads it: the three stored states, with
 * `open` split by whether a reply has been drafted. The split is read off the
 * reply rather than stored (migration 0027), so it cannot disagree with it.
 */
export type Standing = 'waiting' | 'drafted' | 'posted' | 'archived'

export function standingOf(comment: Pick<Comment, 'state' | 'reply'>): Standing {
  if (comment.state === 'posted') return 'posted'
  if (comment.state === 'archived') return 'archived'
  return comment.reply !== null && comment.reply.trim() !== '' ? 'drafted' : 'waiting'
}

/** A title reduced to what two spellings of it share: case, quotes, spacing. */
function plain(title: string): string {
  return title
    .toLowerCase()
    .replace(/["'«»“”„‘’`]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * The work a screenshot says its comment was written under, when exactly one
 * work goes by that title. A hint for the person, never a decision: two works
 * sharing a title, or none, give no answer rather than a guess.
 */
export function workByTitle(
  about: string | undefined,
  works: readonly { id: string; title: string }[],
): string | undefined {
  if (about === undefined) return undefined
  const wanted = plain(about)
  if (wanted === '') return undefined
  const found = works.filter((work) => plain(work.title) === wanted)
  return found.length === 1 ? found[0]?.id : undefined
}

const CHANNEL_KEY = 'kilna.comments.channel'

/** The channel the last comment was filed under, on this machine: what the
 *  next paste is offered, because comments arrive in runs from one place. */
export function recallChannel(): string {
  try {
    return localStorage.getItem(CHANNEL_KEY) ?? ''
  } catch {
    return ''
  }
}

export function rememberChannel(channel: string) {
  try {
    if (channel.trim() !== '') localStorage.setItem(CHANNEL_KEY, channel.trim())
  } catch {
    // A machine that keeps nothing still files the comment; it just asks again.
  }
}
