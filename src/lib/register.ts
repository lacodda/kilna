import type { Bank, Strictness, Term, TermKind } from '@/lib/api/types'
import type { Status } from '@/components/ui/status-dot'

/*
 * The register of repeats as the window reads it (ADR 0044): the words the
 * code keeps for strictness and kind, and how a term is marked in a text.
 *
 * Which works carry a term, and where a text takes one, are never worked out
 * here: the backend reads every text through one analyser, and a second one in
 * the window would be the place the marks and the counts disagreed.
 */

/** Strictest first, the order the register reads in. */
export const STRICTNESS: readonly Strictness[] = ['ban', 'limit', 'rare']

/** Wording first, then the meanings. */
export const TERM_KINDS: readonly TermKind[] = [
  'noun',
  'adjective',
  'verb',
  'phrase',
  'image',
  'scene',
  'pattern',
]

/** Whether a term of this kind is found in a text by its words. A meaning is
 *  not: the works that carry it are named. */
export function isWording(kind: TermKind): boolean {
  return kind === 'noun' || kind === 'adjective' || kind === 'verb' || kind === 'phrase'
}

/** The kind a term written with these words most likely is: one word is a
 *  noun until the person says otherwise, several are a phrase. The backend
 *  makes the same guess for a caller that names no kind. */
export function guessKind(word: string): TermKind {
  return word.trim().split(/\s+/).length > 1 ? 'phrase' : 'noun'
}

/** The colour a strictness is drawn in where a term is named rather than
 *  marked in a text - a row, a chip: a ban red, a limit amber, a rare word
 *  blue, the same colours the lines under the words take. */
export function strictnessStatus(strictness: Strictness): Status {
  return strictness === 'ban' ? 'bad' : strictness === 'limit' ? 'warn' : 'info'
}

/** How a term of this strictness is marked in a text: see `styles.css`. */
export function termMark(strictness: Strictness): string {
  return `term-${strictness}`
}

/** Forms as a person types them - "окна, окон" - one per comma. */
export function splitForms(text: string): string[] {
  return text
    .split(',')
    .map((form) => form.trim())
    .filter((form) => form !== '')
}

/** Where a word stands in the bank, in the order the bank reads them. */
export const BANK: readonly Bank[] = ['fresh', 'parked', 'dropped']

/**
 * The facets a word of the record can be read under on the register's screen
 * (ADR 0052): one record per word, and a word can be spent, kept in the bank
 * and sung its own way at once.
 */
export type Facet = 'all' | Strictness | 'bank' | 'sung'

/** The facets the screen filters by, in the order its chips stand. */
export const FACETS: readonly Facet[] = ['all', 'ban', 'limit', 'rare', 'bank', 'sung']

/** Whether a word is read under a facet. */
export function inFacet(word: Pick<Term, 'strictness' | 'bank' | 'sung'>, facet: Facet): boolean {
  switch (facet) {
    case 'all':
      return true
    case 'bank':
      return word.bank !== null
    case 'sung':
      return word.sung.length > 0
    default:
      return word.strictness === facet
  }
}
