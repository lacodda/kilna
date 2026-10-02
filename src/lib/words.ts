import type { Bank, BlockView, RegisterEntry, WordsPackage } from '@/lib/api/types'
import { BANK } from '@/lib/register'

/*
 * The bank of words as the window reads it (ADR 0052): which words a part of
 * the bank shows, in what order, and how a typed list becomes words.
 *
 * A word of the bank is a term of the record with its `bank` facet set, and a
 * block is a list of term ids in the order they were put there. Everything
 * here is a pure function of those two lists, so the screen and its tests
 * read the same answer.
 */

/** The part of the bank on show: `ALL_WORDS`, `NO_BLOCK`, or one block by its
 *  id. Block ids are UUIDs, so neither word can be taken for one. */
export type BankScope = string

export const ALL_WORDS = 'all'
export const NO_BLOCK = 'none'

/** The block a scope is, when it is one. */
export function blockOf(scope: BankScope, blocks: readonly BlockView[]): BlockView | null {
  return blocks.find((block) => block.id === scope) ?? null
}

/** Where a state stands in the bank's reading order: fresh first, because
 *  what is still there to use is what a bank is opened for. A word that is in
 *  a block but out of the bank reads last. */
function rank(bank: Bank | null): number {
  return bank === null ? BANK.length : BANK.indexOf(bank)
}

/** Fresh before set aside before dropped, then by the word. */
function bankOrder(a: RegisterEntry, b: RegisterEntry): number {
  return rank(a.bank) - rank(b.bank) || a.word.localeCompare(b.word)
}

/**
 * The words a scope shows. A block keeps the order its words were put there,
 * which is the owner's; the bank as a whole, and its words in no block, read
 * by state and then by the word. A block's id that the register no longer
 * has - a word deleted a moment ago - is passed over rather than drawn empty.
 */
export function wordsIn(
  scope: BankScope,
  entries: readonly RegisterEntry[],
  blocks: readonly BlockView[],
): RegisterEntry[] {
  const block = blockOf(scope, blocks)
  if (block !== null) {
    const byId = new Map(entries.map((entry) => [entry.id, entry]))
    return block.term_ids.flatMap((id) => {
      const entry = byId.get(id)
      return entry === undefined ? [] : [entry]
    })
  }
  const placed = new Set(blocks.flatMap((one) => one.term_ids))
  return entries
    .filter((entry) =>
      scope === NO_BLOCK
        ? entry.bank !== null && !placed.has(entry.id)
        : entry.bank !== null || placed.has(entry.id),
    )
    .sort(bankOrder)
}

/**
 * Words as a person types or pastes them: one, or a list by commas or by
 * lines. Not by spaces - "on the edge" is one entry of the bank. A word given
 * twice is kept once, the first spelling winning; the backend matches the
 * record whatever the case, and asking it for the same word twice in one list
 * would only say "already there" about the list itself.
 */
export function splitWords(text: string): string[] {
  const seen = new Set<string>()
  const words: string[] = []
  for (const part of text.split(/[,\n\r]+/)) {
    const word = part.trim()
    const key = word.toLowerCase()
    if (word === '' || seen.has(key)) continue
    seen.add(key)
    words.push(word)
  }
  return words
}

/** Every item of a words package, by the key an apply names it with. */
export function wordItems(pkg: WordsPackage): string[] {
  return pkg.words.map((_, index) => `word:${String(index)}`)
}
