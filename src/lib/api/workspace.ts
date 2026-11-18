import { invoke } from '@tauri-apps/api/core'
import type { OfferRef, Profile, ProfileConfig, ShippedOffer, Workspace } from '@/lib/api/types'

// The workspace and its profiles: what is open, in which craft's words.

export const getWorkspace = () => invoke<Workspace>('get_workspace')
export const updateProfileConfig = (id: string, config: ProfileConfig) =>
  invoke<Profile>('update_profile_config', { id, config })
/** The versions kilna ships waiting beside the owner's entries (ADR 0087). */
export const shippedOffers = (id: string) => invoke<ShippedOffer[]>('shipped_offers', { id })
export const takeShipped = (id: string, offer: OfferRef) =>
  invoke<Profile>('take_shipped', { id, offer })
export const keepMine = (id: string, offers: OfferRef[]) =>
  invoke<null>('keep_mine', { id, offers })
/** The command that registers this build with Claude Code as an MCP server. */
export const mcpRegistration = () => invoke<string>('mcp_registration')
