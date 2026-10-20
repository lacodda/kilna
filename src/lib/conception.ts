import type {
  Board,
  Conception,
  MetaField,
  ProfileConfig,
  PromptTemplate,
  VersionRole,
} from '@/lib/api/types'
import { vocabularyOf } from '@/lib/useProfile'

/**
 * A work's conception (v0.99, ADR 0065), as the window reads it off the
 * profile: the role its brief is written in, the field the brief is refined
 * into, the boards it lays out, and the actions that work from it.
 */

/** The conception of a kind, when its works are prepared on one. */
export function conceptionOf(config: ProfileConfig, kind: string): Conception | undefined {
  return config.work_kinds.find((entry) => entry.key === kind)?.conception ?? undefined
}

/** The role a work of this kind writes its brief in. */
export function briefRoleOf(config: ProfileConfig, kind: string): VersionRole | undefined {
  const conception = conceptionOf(config, kind)
  if (conception === undefined) return undefined
  return vocabularyOf(config, kind).version_roles.find((role) => role.key === conception.brief)
}

/** Whether `role` is the brief of a work of this kind: a text the
 *  Conception tab writes, which the Versions tab leaves to it. */
export function isBrief(config: ProfileConfig, kind: string, role: string): boolean {
  return conceptionOf(config, kind)?.brief === role
}

/** The field the brief is refined into: a song's premise. */
export function ideaFieldOf(config: ProfileConfig, kind: string): MetaField | undefined {
  const key = conceptionOf(config, kind)?.idea
  if (key === undefined || key === null) return undefined
  return config.work_meta_fields.find((field) => field.key === key)
}

/** The boards a work of this kind lays out, in its conception's order. */
export function boardsOf(config: ProfileConfig, kind: string): Board[] {
  const keys = conceptionOf(config, kind)?.boards ?? []
  return keys
    .map((key) => (config.boards ?? []).find((board) => board.key === key))
    .filter((board): board is Board => board !== undefined)
}

/** Whether an action is offered on a work of this kind. */
function offeredOn(action: PromptTemplate, kind: string): boolean {
  return (action.kinds ?? []).length === 0 || action.kinds!.includes(kind)
}

/** The profile's action that refines the brief and the idea ("Refine"). */
export function refineActionOf(config: ProfileConfig, kind: string): PromptTemplate | undefined {
  return config.prompts.find(
    (action) => action.produces === 'conception' && offeredOn(action, kind),
  )
}

/** The profile's action that writes the work from the whole order: one about
 *  the work that reads `{order}` ("Write from the brief"). */
export function writeActionOf(config: ProfileConfig, kind: string): PromptTemplate | undefined {
  return config.prompts.find(
    (action) =>
      (action.scope ?? 'work') === 'work' &&
      offeredOn(action, kind) &&
      action.template.includes('{order}'),
  )
}
