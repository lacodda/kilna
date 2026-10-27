import { invoke } from '@tauri-apps/api/core'
import type { JournalEntry, SinceVisit, WeekReport } from '@/lib/api/types'

// The journal: what happened here, in sentences.

export const listJournal = () => invoke<JournalEntry[]>('list_journal')
export const unreadJournal = () => invoke<number>('unread_journal')
export const markJournalRead = () => invoke<number>('mark_journal_read')
/** What happened since the history was last marked seen: unread entries and
    the assistant runs that ended since (v0.105). */
export const sinceVisit = () => invoke<SinceVisit>('since_visit')
/** What the seven days from `from` came to, from the operations log (v0.105). */
export const weekReport = (from: string) => invoke<WeekReport>('week_report', { from })
