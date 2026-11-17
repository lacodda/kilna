import { invoke } from '@tauri-apps/api/core'
import type { TextFile, WorkFolder } from '@/lib/api/types'

// Folders on disk the workspace looks at (ADR 0057): the media folder this
// machine keeps for the open workspace, and each work's folder under it.

/** The media folder of the open workspace on this machine, if one is set. */
export const mediaRoot = () => invoke<string | null>('media_root')
/** Set the media folder, or forget it with `null`; answers with it as it now stands. */
export const setMediaRoot = (path: string | null) =>
  invoke<string | null>('set_media_root', { path })
/** A work's folder on disk, and the files in it. */
export const workFolder = (workId: string) => invoke<WorkFolder>('work_folder', { workId })
/** Open a work's folder in the system's file manager, making it first if it
    is not there: a folder is made when it is first needed (FR-04). Refused
    when there is no telling where it would be. */
export const openWorkFolder = (workId: string) => invoke<WorkFolder>('open_work_folder', { workId })
/** Where the workspace keeps its own files: what `media/<name>` in a note is read against. */
export const mediaDirectory = () => invoke<string>('media_directory')
/** Open a picture, a clip, a sound or a document kilna looks at in the
    program the system opens it with. Anything else is refused by the backend. */
export const openMedia = (path: string) => invoke<void>('open_media', { path })
/** A text read for the viewer: a lyric, subtitles, notes. Only a text under
    the media folder or among the workspace's own files, and only so much
    of it - the answer says when there is more. */
export const readTextFile = (path: string) => invoke<TextFile>('read_text_file', { path })
/** Save a copy of a file kilna looks at where the person chose. */
export const saveFileCopy = (source: string, destination: string) =>
  invoke<void>('save_file_copy', { source, destination })
/** Rename a file of a work's folder, by its path inside it. Refused onto a
    name that is taken; a finished file chosen by hand follows its name. */
export const renameFolderFile = (workId: string, relative: string, name: string) =>
  invoke<WorkFolder>('rename_folder_file', { workId, relative, name })
/** Move a file of a work's folder to the system's recycle bin. */
export const trashFolderFile = (workId: string, relative: string) =>
  invoke<WorkFolder>('trash_folder_file', { workId, relative })
