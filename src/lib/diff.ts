/**
 * A line-by-line comparison of two drafts.
 *
 * Words would be finer, but a version here is prose — lyrics, an outline, a
 * script — and prose is revised by the line. Seeing "this line went, that one
 * arrived" answers *what changed between v3 and v4* directly; a word-level
 * diff of a rewritten verse is confetti.
 *
 * Plain longest-common-subsequence over lines. Bodies are a page or two, so an
 * O(n·m) table costs nothing and the result is exact rather than heuristic.
 */

export type Change =
  | { kind: 'same'; text: string }
  | { kind: 'added'; text: string }
  | { kind: 'removed'; text: string }

/** How many lines each side may have before the comparison gives up. */
const LIMIT = 2000

export function diffLines(before: string, after: string): Change[] {
  const a = before.split('\n')
  const b = after.split('\n')

  // A table of 2000×2000 is already 4M cells; past that the honest answer is
  // "too long to compare", not a frozen window.
  if (a.length > LIMIT || b.length > LIMIT) {
    return [
      { kind: 'removed', text: before },
      { kind: 'added', text: after },
    ]
  }

  // lcs[i][j] — length of the longest common subsequence of a[i..] and b[j..].
  const lcs: number[][] = Array.from({ length: a.length + 1 }, () =>
    new Array<number>(b.length + 1).fill(0),
  )

  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      lcs[i]![j] = a[i] === b[j] ? lcs[i + 1]![j + 1]! + 1 : Math.max(lcs[i + 1]![j]!, lcs[i]![j + 1]!)
    }
  }

  const changes: Change[] = []
  let i = 0
  let j = 0

  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      changes.push({ kind: 'same', text: a[i]! })
      i += 1
      j += 1
    } else if (lcs[i + 1]![j]! >= lcs[i]![j + 1]!) {
      changes.push({ kind: 'removed', text: a[i]! })
      i += 1
    } else {
      changes.push({ kind: 'added', text: b[j]! })
      j += 1
    }
  }

  while (i < a.length) {
    changes.push({ kind: 'removed', text: a[i]! })
    i += 1
  }
  while (j < b.length) {
    changes.push({ kind: 'added', text: b[j]! })
    j += 1
  }

  return changes
}

/** How much moved, for a one-line summary above the comparison. */
export function countChanges(changes: Change[]): { added: number; removed: number } {
  let added = 0
  let removed = 0
  for (const change of changes) {
    if (change.kind === 'added') added += 1
    else if (change.kind === 'removed') removed += 1
  }
  return { added, removed }
}
