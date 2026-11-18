import { invoke } from '@tauri-apps/api/core'
import type {
  EntryStanding,
  OfferRef,
  Profile,
  ProfileConfig,
  PromptTemplate,
  ShippedOffer,
  SkillPreview,
  Workspace,
} from '@/lib/api/types'

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
/** Where each shipped entry of one list stands, with what a reset puts back. */
export const shippedStanding = (id: string, collection: string) =>
  invoke<EntryStanding[]>('shipped_standing', { id, collection })
/** A skill as the assistant would be handed it, for a kind of work (FR-17). */
export const previewSkill = (kind: string, action: PromptTemplate) =>
  invoke<SkillPreview>('preview_skill', { kind, action })
/** The skills a file holds, to lay over the profile's draft. */
export const readSkillSet = (path: string) => invoke<PromptTemplate[]>('read_skill_set', { path })
/** The command that registers this build with Claude Code as an MCP server. */
export const mcpRegistration = () => invoke<string>('mcp_registration')
