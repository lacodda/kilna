import { invoke } from '@tauri-apps/api/core'
import type { JournalEntry } from '@/lib/api/types'

// The journal: what happened here, in sentences.

export const listJournal = () => invoke<JournalEntry[]>('list_journal')
export const unreadJournal = () => invoke<number>('unread_journal')
export const markJournalRead = () => invoke<number>('mark_journal_read')
