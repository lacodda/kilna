import { invoke } from '@tauri-apps/api/core'
import type { NewVersion, Version, VersionSummary } from '@/lib/api/types'

// A work's versions: every text it has been, whole.

export const listVersions = (workId: string) =>
  invoke<VersionSummary[]>('list_versions', { workId })
export const getVersion = (id: string) => invoke<Version | null>('get_version', { id })
/** Change the open version's text in place. Refused with kind `frozen` once
    a score has read it — then the change belongs in the next revision. */
export const updateVersionBody = (id: string, body: string) =>
  invoke<Version>('update_version_body', { id, body })
export const createVersion = (workId: string, version: NewVersion) =>
  invoke<Version>('create_version', { workId, version })
export const setCurrentVersion = (workId: string, versionId: string) =>
  invoke<void>('set_current_version', { workId, versionId })
export const deleteVersion = (id: string) => invoke<string>('delete_version', { id })
