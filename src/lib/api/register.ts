import { invoke } from '@tauri-apps/api/core'
import type {
  Banked,
  BlockView,
  NewTerm,
  RegisterEntry,
  RepeatMark,
  Repeats,
  Term,
  TermBlock,
  TermPatch,
  TermUse,
  TextCheck,
  WordsPackage,
} from '@/lib/api/types'

// The register of repeats (ADR 0044), and a text checked against it.

/** Every term, with how many works carry it now. */
export const listTerms = () => invoke<RegisterEntry[]>('list_terms')
/** The works a term is in: found in their current text, or named. */
export const termUses = (id: string) => invoke<TermUse[]>('term_uses', { id })
/** How many works these words are in, before they are a term. */
export const previewTerm = (word: string, forms: string[]) =>
  invoke<number>('preview_term', { word, forms })
export const listTermTopics = () => invoke<[string, number][]>('list_term_topics')
export const createTerm = (term: NewTerm) => invoke<Term>('create_term', { term })
export const updateTerm = (id: string, patch: TermPatch) =>
  invoke<Term>('update_term', { id, patch })
/** Into the trash, with the works it named. */
export const deleteTerm = (id: string) => invoke<string>('delete_term', { id })
export const linkTerm = (termId: string, workId: string) =>
  invoke<null>('link_term', { termId, workId })
export const unlinkTerm = (termId: string, workId: string) =>
  invoke<null>('unlink_term', { termId, workId })
/** The words a text leans on, the terms it takes, and where to mark both -
 *  and, for a text that is `sung`, where its stresses fall (ADR 0053). */
export const checkText = (text: string, sung = false) =>
  invoke<TextCheck>('check_text', { text, sung })
/** A sung text as the public reads it: the stress marks off, the respellings
 *  back (ADR 0053). */
export const cleanText = (text: string) => invoke<string>('clean_text', { text })

// The bank of words: blocks, and the words put in them (ADR 0052).

/** The blocks in the owner's order, each with its words. */
export const listBlocks = () => invoke<BlockView[]>('list_blocks')
export const createBlock = (name: string) => invoke<TermBlock>('create_block', { name })
export const renameBlock = (id: string, name: string) =>
  invoke<TermBlock>('rename_block', { id, name })
/** Put a block at `index` in the order; the blocks as they now stand. */
export const moveBlock = (id: string, index: number) =>
  invoke<TermBlock[]>('move_block', { id, index })
/** Into the trash, with the words put in it; the words stay in the bank. */
export const deleteBlock = (id: string) => invoke<string>('delete_block', { id })
export const addToBlock = (blockId: string, termId: string) =>
  invoke<null>('add_to_block', { blockId, termId })
export const removeFromBlock = (blockId: string, termId: string) =>
  invoke<null>('remove_from_block', { blockId, termId })
/** Words into the bank - and into a block, when one is named. */
export const bankWords = (words: string[], blockId: string | null) =>
  invoke<Banked>('bank_words', { words, blockId })
/** What the owner's sung texts already say about singing, proposed. */
export const wordsFromTexts = () => invoke<WordsPackage>('words_from_texts')
/** Keep a package of words whole, or the items named (`word:N`). */
export const keepWords = (pkg: WordsPackage, items: string[] | null) =>
  invoke<string[]>('keep_words', { package: pkg, items })

// The guard of repeats (ADR 0054).

/** The mark every work wears on `today` (`yyyy-mm-dd`). */
export const repeatMarks = (today: string) => invoke<RepeatMark[]>('repeat_marks', { today })
/** What the guard says about the song a work is, or is made from. */
export const workRepeats = (workId: string, today: string) =>
  invoke<Repeats | null>('work_repeats', { workId, today })
/** "I know, I am keeping it" on a word of a song. */
export const keepRepeat = (workId: string, word: string) =>
  invoke<null>('keep_repeat', { workId, word })
export const unkeepRepeat = (workId: string, word: string) =>
  invoke<null>('unkeep_repeat', { workId, word })
