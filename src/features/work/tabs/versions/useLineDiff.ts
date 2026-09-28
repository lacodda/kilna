import { useMemo } from 'react'
import type { LineMark } from '@/components/ui/marked-text'
import { changedLines, countChanges, diffLines } from '@/lib/diff'

interface LineDiff {
  /** Lines of the newer text that the older one does not have. */
  added: LineMark[]
  /** Lines of the older text that the newer one dropped. */
  removed: LineMark[]
  /** How many of each, or null with nothing to compare against. */
  counts: { added: number; removed: number } | null
}

const NONE: LineMark[] = []

/**
 * Two texts, line against line, as marks for either side.
 *
 * Read off whatever is on screen, so while someone types into the newer side
 * the marks follow the keystrokes: a comparison with the original is what
 * keeps a rewrite honest, and a comparison with the text as it was when the
 * editor opened would stop being true at the first word.
 */
export function useLineDiff(older: string | null, newer: string): LineDiff {
  const diff = useMemo(() => (older === null ? null : diffLines(older, newer)), [older, newer])
  return useMemo(() => {
    if (diff === null) return { added: NONE, removed: NONE, counts: null }
    const moved = changedLines(diff)
    return {
      added: [...moved.added].map((line) => ({ line, className: 'bg-good-soft' })),
      removed: [...moved.removed].map((line) => ({ line, className: 'bg-bad-soft' })),
      counts: countChanges(diff),
    }
  }, [diff])
}
