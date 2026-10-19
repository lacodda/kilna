import { invoke } from '@tauri-apps/api/core'
import type {
  Kept,
  LineRow,
  NewNote,
  Note,
  NoteFilter,
  NotePatch,
  Promoted,
  Promotion,
  Shelf,
  ShelfView,
} from '@/lib/api/types'

// Notes: the two shelves of material for works to come - phrases and notes
// (ADR 0064) - and the lore and references kept beside the works.

export const listNotes = (filter?: NoteFilter) => invoke<Note[]>('list_notes', { filter })
export const createNote = (note: NewNote) => invoke<Note>('create_note', { note })
export const updateNote = (id: string, patch: NotePatch) =>
  invoke<Note>('update_note', { id, patch })
export const deleteNote = (id: string) => invoke<string>('delete_note', { id })
export const listTags = () => invoke<[string, number][]>('list_tags')
/** Rename a tag on every note that carries it, or take it off them with no
 *  new name: how many notes changed. */
export const retagNotes = (from: string, to: string | null) =>
  invoke<number>('retag_notes', { from, to })
/** Turn a note into a work it stays tied to. Nothing is copied; a phrase is
 *  spent as well. Undo takes the whole gesture back. */
export const promoteNote = (id: string, promotion: Promotion) =>
  invoke<Promoted>('promote_note', { id, promotion })

// The phrases and their shelves.

/** The phrases a filter leaves, each with its shelves, how many works say it
 *  and how it is sung. */
export const listLines = (filter?: NoteFilter) => invoke<LineRow[]>('list_lines', { filter })
/** Lines typed or pasted as a list, kept as phrases of `kind` - and on a
 *  shelf, when one is named. A line kept already is not kept twice. */
export const keepLines = (lines: string[], kind: string, shelfId: string | null, tags: string[]) =>
  invoke<Kept>('keep_lines', { lines, kind, shelfId, tags })
/** The shelves in the owner's order, each with its phrases. */
export const listShelves = () => invoke<ShelfView[]>('list_shelves')
export const createShelf = (name: string) => invoke<Shelf>('create_shelf', { name })
export const renameShelf = (id: string, name: string) => invoke<Shelf>('rename_shelf', { id, name })
/** Put a shelf at `index` in the order; the shelves as they now stand. */
export const moveShelf = (id: string, index: number) => invoke<Shelf[]>('move_shelf', { id, index })
/** Into the trash, with what stood on it; the phrases stay. */
export const deleteShelf = (id: string) => invoke<string>('delete_shelf', { id })
export const addToShelf = (shelfId: string, noteId: string) =>
  invoke<null>('add_to_shelf', { shelfId, noteId })
export const removeFromShelf = (shelfId: string, noteId: string) =>
  invoke<null>('remove_from_shelf', { shelfId, noteId })
