import { invoke } from '@tauri-apps/api/core'

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

export interface ProfileConfig {
  work_kinds: Kind[]
  release_kinds: Kind[]
  collection_kinds: Kind[]
  version_roles: Kind[]
  statuses: Kind[]
  axes: Axis[]
  tiers: Tier[]
  work_meta_fields: { key: string; label: string; type: string }[]
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

export const getWorkspace = () => invoke<Workspace>('get_workspace')
export const listProfiles = () => invoke<Profile[]>('list_profiles')
export const activateProfile = (id: string) => invoke<void>('activate_profile', { id })
