import type { WordsPackage } from '@/lib/api/types'

/*
 * A package of words as the window reads it (ADR 0052): a phrase to keep, a
 * way of singing, a term for the register, one item per word.
 */

/** Every item of a words package, by the key an apply names it with. */
export function wordItems(pkg: WordsPackage): string[] {
  return pkg.words.map((_, index) => `word:${String(index)}`)
}
