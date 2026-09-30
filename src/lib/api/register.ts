import { invoke } from '@tauri-apps/api/core'
import type { NewTerm, RegisterEntry, Term, TermPatch, TermUse, TextCheck } from '@/lib/api/types'

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
/** The words a text leans on, the terms it takes, and where to mark both. */
export const checkText = (text: string) => invoke<TextCheck>('check_text', { text })
