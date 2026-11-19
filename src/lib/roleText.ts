import { getVersion, listVersions } from '@/lib/api/versions'
import type { ProfileConfig, Work } from '@/lib/api/types'

/**
 * The text that stands for a role of a work, as the backend reads it
 * (`work::version::for_role`): the work's current version when it is of that
 * role, the one marked main, and the newest otherwise. What "copy the Suno
 * prompt" copies (v0.153).
 */
export async function roleText(workId: string, role: string): Promise<string | null> {
  const versions = (await listVersions(workId)).filter((one) => one.role === role)
  const standing =
    versions.find((one) => one.is_current) ??
    versions.find((one) => one.is_main) ??
    [...versions].sort((a, b) => b.revision - a.revision)[0]
  if (standing === undefined) return null
  return (await getVersion(standing.id))?.body ?? null
}

/** The role a composition writes for works of `kind` - a track's Suno prompt,
 *  a song's style prompt - when the profile has one. */
export function composedRole(config: ProfileConfig, kind: string): string | null {
  return (
    (config.compose ?? []).find((one) => one.kinds.length === 0 || one.kinds.includes(kind))
      ?.role ?? null
  )
}

/** What a work's picture is drawn from: the prompt last copied out of its
 *  cover, or the scene its cover says. */
export function coverText(work: Work): string | null {
  const said = work.cover.sent?.picture ?? work.cover.scene
  return said.trim() === '' ? null : said.trim()
}
