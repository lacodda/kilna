import type { PromptTemplate, Standing } from '@/lib/api/types'
import { isButton, scopeOf, type ActionScope } from '@/lib/actions'
import { resolveLabel } from '@/lib/label'

/**
 * The profile's skills (FR-17): an action that is a button with a message, or
 * a playbook - a method an agent reads. One list, read and filtered here.
 */

/** The groups a skill is shown in, in their order. The backend says the same
 *  (`assistant::prompt::CATEGORIES`). */
export const CATEGORIES = [
  'writing',
  'critique',
  'scoring',
  'video',
  'cover',
  'styles',
  'canon',
  'release',
  'workflow',
] as const

export type SkillMode = 'button' | 'playbook'

/** What a skill is to the window: a button, or a method alone. */
export function modeOf(action: PromptTemplate): SkillMode {
  return isButton(action) ? 'button' : 'playbook'
}

/** Where a skill stands against what kilna ships: its owner's own when kilna
 *  ships nothing by its key. */
export type Origin = Standing | 'own'

export interface SkillFilter {
  query: string
  /** A kind of work: the skills for it, every-kind ones included. */
  kind: string
  scope: ActionScope | ''
  mode: SkillMode | ''
  origin: Origin | ''
}

export const NO_FILTER: SkillFilter = { query: '', kind: '', scope: '', mode: '', origin: '' }

/** Whether a skill answers a filter. The words are looked for in its key,
 *  its name in either language, its trigger and its method. */
export function answers(action: PromptTemplate, filter: SkillFilter, origin: Origin): boolean {
  const needle = filter.query.trim().toLowerCase()
  if (needle !== '') {
    const words = [
      action.key,
      ...labelWords(action.label),
      ...labelWords(action.when),
      action.method ?? '',
    ]
    if (!words.some((word) => word.toLowerCase().includes(needle))) return false
  }
  if (
    filter.kind !== '' &&
    (action.kinds ?? []).length > 0 &&
    !action.kinds!.includes(filter.kind)
  ) {
    return false
  }
  if (filter.scope !== '' && scopeOf(action) !== filter.scope) return false
  if (filter.mode !== '' && modeOf(action) !== filter.mode) return false
  if (filter.origin !== '' && origin !== filter.origin) return false
  return true
}

function labelWords(label: PromptTemplate['label'] | null | undefined): string[] {
  if (label === undefined || label === null) return []
  return typeof label === 'string' ? [label] : Object.values(label)
}

/** The skills by group, in the groups' order, those of no group last. */
export function grouped(
  actions: PromptTemplate[],
): { category: string | null; actions: PromptTemplate[] }[] {
  const groups: { category: string | null; actions: PromptTemplate[] }[] = []
  for (const category of [...CATEGORIES, null] as const) {
    const held = actions.filter((action) =>
      category === null
        ? !(CATEGORIES as readonly string[]).includes(action.category ?? '')
        : action.category === category,
    )
    if (held.length > 0) groups.push({ category, actions: held })
  }
  return groups
}

/** A key for a copy of a skill that no skill has yet: `critique-copy`,
 *  `critique-copy-2`. */
export function copyKey(actions: PromptTemplate[], key: string): string {
  const taken = new Set(actions.map((action) => action.key))
  let candidate = `${key}-copy`
  for (let n = 2; taken.has(candidate); n += 1) candidate = `${key}-copy-${n}`
  return candidate
}

/** What a skill is called, in the window's language. */
export function skillName(action: PromptTemplate, language: string): string {
  return resolveLabel(action.label, language) || action.key
}

/** A set of skills as it is written to a file: marked as kilna's, so a file
 *  of something else is told apart on the way in. */
export function skillSet(actions: PromptTemplate[]): string {
  return JSON.stringify({ kilna: 'skills', version: 1, skills: actions }, null, 2) + '\n'
}

/** Skills read from a file laid over the profile's: one with a key the
 *  profile has replaces it in place, one it does not have joins at the end.
 *  Returns the list and how many of each. */
export function laidOver(
  actions: PromptTemplate[],
  incoming: PromptTemplate[],
): { actions: PromptTemplate[]; replaced: number; added: number } {
  const byKey = new Map(incoming.map((action) => [action.key, action]))
  const next = actions.map((action) => byKey.get(action.key) ?? action)
  const known = new Set(actions.map((action) => action.key))
  const fresh = incoming.filter((action) => !known.has(action.key))
  return {
    actions: [...next, ...fresh],
    replaced: incoming.length - fresh.length,
    added: fresh.length,
  }
}
