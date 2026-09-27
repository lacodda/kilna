import { invoke } from '@tauri-apps/api/core'
import type { Profile, ProfileConfig, Workspace } from '@/lib/api/types'

// The workspace and its profiles: what is open, in which craft's words.

export const getWorkspace = () => invoke<Workspace>('get_workspace')
export const listProfiles = () => invoke<Profile[]>('list_profiles')
export const activateProfile = (id: string) => invoke<void>('activate_profile', { id })
export const updateProfileConfig = (id: string, config: ProfileConfig) =>
  invoke<Profile>('update_profile_config', { id, config })
/** The command that registers this build with Claude Code as an MCP server. */
export const mcpRegistration = () => invoke<string>('mcp_registration')
