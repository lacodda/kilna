import { invoke } from '@tauri-apps/api/core'

// These mirror the Rust structs in src-tauri/src. Nothing enforces that they
// agree — see ADR 0003 — so a change on one side means a change here.

export interface Axis {
  key: string
  label: string
  weight: number
  scale: number
  description?: string
}

export interface Tier {
  key: string
  label: string
  min: number
}

export interface Kind {
  key: string
  label: string
}

export interface MetaField {
  key: string
  label: string
  type: 'text' | 'number' | 'date' | 'boolean'
}

export interface ProfileConfig {
  work_kinds: Kind[]
  release_kinds: Kind[]
  collection_kinds: Kind[]
  version_roles: Kind[]
  statuses: Kind[]
  axes: Axis[]
  tiers: Tier[]
  work_meta_fields: MetaField[]
}

export interface Profile {
  id: string
  key: string
  name: string
  description: string | null
  config: ProfileConfig
  is_active: boolean
  is_builtin: boolean
}

export interface Workspace {
  schema_version: number
  profile: Profile | null
  works: number
  releases: number
}

export type Meta = Record<string, unknown>

export interface Work {
  id: string
  profile_id: string
  collection_id: string | null
  kind: string
  title: string
  status: string
  meta: Meta
  current_version_id: string | null
  position: number
  created_at: string
  updated_at: string
}

export interface NewWork {
  kind: string
  title: string
  status?: string | null
  collection_id?: string | null
  meta?: Meta | null
}

// A field left out is untouched; `null` inside a nullable field clears it.
export interface WorkPatch {
  title?: string
  status?: string
  kind?: string
  collection_id?: string | null
  meta?: Meta
  current_version_id?: string | null
}

export interface WorkFilter {
  status?: string
  kind?: string
  collection_id?: string
  search?: string
}

export interface Version {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  body: string
  meta: Meta
  created_at: string
}

export interface VersionSummary {
  id: string
  work_id: string
  role: string
  revision: number
  label: string | null
  length: number
  created_at: string
  is_current: boolean
}

export interface NewVersion {
  role: string
  body: string
  label?: string | null
  meta?: Meta | null
  make_current?: boolean
}

export interface Note {
  id: string
  profile_id: string
  work_id: string | null
  kind: string
  title: string | null
  body: string
  tags: string[]
  created_at: string
  updated_at: string
}

export interface NewNote {
  body: string
  kind?: string | null
  title?: string | null
  work_id?: string | null
  tags?: string[]
}

export interface NotePatch {
  title?: string | null
  body?: string
  kind?: string
  tags?: string[]
  work_id?: string | null
}

export interface NoteFilter {
  work_id?: string
  kind?: string
  tag?: string
  search?: string
}

export const getWorkspace = () => invoke<Workspace>('get_workspace')
export const listProfiles = () => invoke<Profile[]>('list_profiles')
export const activateProfile = (id: string) => invoke<void>('activate_profile', { id })

export const listWorks = (filter?: WorkFilter) => invoke<Work[]>('list_works', { filter })
export const getWork = (id: string) => invoke<Work | null>('get_work', { id })
export const createWork = (work: NewWork) => invoke<Work>('create_work', { work })
export const updateWork = (id: string, patch: WorkPatch) => invoke<Work>('update_work', { id, patch })
export const deleteWork = (id: string) => invoke<void>('delete_work', { id })

export const listVersions = (workId: string) => invoke<VersionSummary[]>('list_versions', { workId })
export const getVersion = (id: string) => invoke<Version | null>('get_version', { id })
export const createVersion = (workId: string, version: NewVersion) =>
  invoke<Version>('create_version', { workId, version })
export const setCurrentVersion = (workId: string, versionId: string) =>
  invoke<void>('set_current_version', { workId, versionId })
export const deleteVersion = (id: string) => invoke<void>('delete_version', { id })

export const listNotes = (filter?: NoteFilter) => invoke<Note[]>('list_notes', { filter })
export const createNote = (note: NewNote) => invoke<Note>('create_note', { note })
export const updateNote = (id: string, patch: NotePatch) => invoke<Note>('update_note', { id, patch })
export const deleteNote = (id: string) => invoke<void>('delete_note', { id })
export const listTags = () => invoke<[string, number][]>('list_tags')
