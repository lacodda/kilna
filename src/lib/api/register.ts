import { invoke } from '@tauri-apps/api/core'
import type {
  NewTerm,
  RegisterEntry,
  RepeatMark,
  Repeats,
  Term,
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
 *  and, for a text that is `sung`, where its stresses fall (ADR 0053). A
 *  version's text names its work and role: a role a composition writes is
 *  read against the dictionary instead (v0.94). */
export const checkText = (text: string, sung = false, workId?: string, role?: string) =>
  invoke<TextCheck>('check_text', { text, sung, workId, role })
/** A sung text as the public reads it: the stress marks off, the respellings
 *  back (ADR 0053). */
export const cleanText = (text: string) => invoke<string>('clean_text', { text })

// Words for the record: ways of singing, terms (ADR 0052).

/** What the owner's sung texts already say about singing, proposed. */
export const wordsFromTexts = () => invoke<WordsPackage>('words_from_texts')
/** Keep a package of words whole, or the items named (`word:N`): the ids of
 *  the words written, then of the phrases kept. */
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
