import { invoke } from '@tauri-apps/api/core'
import type { NewNote, Note, NoteFilter, NotePatch, Promoted, Promotion } from '@/lib/api/types'

// Notes: ideas, lore and whatever else is kept beside the works.

export const listNotes = (filter?: NoteFilter) => invoke<Note[]>('list_notes', { filter })
export const createNote = (note: NewNote) => invoke<Note>('create_note', { note })
export const updateNote = (id: string, patch: NotePatch) =>
  invoke<Note>('update_note', { id, patch })
export const deleteNote = (id: string) => invoke<string>('delete_note', { id })
export const listTags = () => invoke<[string, number][]>('list_tags')
/** Turn a note into a work whose first version is the note's body. The note
 *  goes to the trash; undo takes the whole gesture back. */
export const promoteNote = (id: string, promotion: Promotion) =>
  invoke<Promoted>('promote_note', { id, promotion })
