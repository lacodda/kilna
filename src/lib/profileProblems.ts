import type { Reason } from '@/lib/api/generated/Reason'

/*
 * What is wrong with a profile, read by where it is wrong (v0.112).
 *
 * The backend names each problem by a code and hands its place over as a
 * value - `place.axis` inside `place.workKind`, each with its number and its
 * key - rather than as a sentence. So the editor can do what a sentence never
 * let it: say a problem beside the row it is about, and mark the part of the
 * profile that will not save.
 */

/** One step of a place: an axis, a kind of work, an action. */
interface PlaceStep {
  /** `axis` of `place.axis`. */
  what: string
  key: string | null
  /** Its number in its list, from 1. */
  n: number | null
}

function isReason(value: unknown): value is Reason {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as Reason).key === 'string' &&
    typeof (value as Reason).params === 'object' &&
    (value as Reason).params !== null
  )
}

/** The steps of one place, outermost first. */
function steps(place: Reason): PlaceStep[] {
  const outer = place.params.in
  const here: PlaceStep = {
    what: place.key.replace(/^place\./, ''),
    key: typeof place.params.key === 'string' ? place.params.key : null,
    n: typeof place.params.n === 'number' ? place.params.n : null,
  }
  return isReason(outer) ? [...steps(outer), here] : [here]
}

/**
 * Where a problem is, outermost first: the kind of work, then its axis. Empty
 * for a problem of the profile as a whole.
 */
function placeOf(problem: Reason): PlaceStep[] {
  for (const value of Object.values(problem.params)) {
    if (isReason(value) && value.key.startsWith('place.')) return steps(value)
  }
  return []
}

/** A step to look for: what it is and, when it matters, its key. */
export type Wanted = readonly [what: string, key?: string]

/**
 * The problems whose place begins with `path`: every problem of a kind, or of
 * one axis of it.
 */
export function problemsAt(problems: readonly Reason[], path: readonly Wanted[]): Reason[] {
  return problems.filter((problem) => {
    const place = placeOf(problem)
    return path.every(([what, key], index) => {
      const step = place[index]
      return step !== undefined && step.what === what && (key === undefined || step.key === key)
    })
  })
}

/** The parts the profile editor is laid out in. */
export type ProfilePart = 'craft' | 'fields' | 'actions' | `kind:${string}`

/** The part of the editor a problem is shown in. */
export function partOf(problem: Reason): ProfilePart {
  const [first] = placeOf(problem)
  if (first === undefined) return 'craft'
  if (first.what === 'workKind' && first.key !== null) return `kind:${first.key}`
  if (first.what === 'action') return 'actions'
  if (first.what === 'metaField') return 'fields'
  return 'craft'
}

/** A list of the profile whose entries are rows of the editor, each named
 *  by its key. */
type Keyed = readonly { key: string }[] | undefined

/**
 * Whether the editor has a row a problem is said under: an axis, a tier, a
 * version role or a kind of release of a kind of work; a field; an action; a
 * type of the style dictionary - and the entry is there. Everything else is
 * said at the head of its part: a problem of the profile as a whole, of a
 * list (a key twice), or of something the editor draws no row for.
 */
export function hasRow(
  problem: Reason,
  config: {
    work_kinds: readonly {
      key: string
      axes?: Keyed
      tiers?: Keyed
      version_roles?: Keyed
      release_kinds?: Keyed
    }[]
    work_meta_fields: Keyed
    prompts: Keyed
    style_types?: Keyed
  },
): boolean {
  const [first, second] = placeOf(problem)
  if (first === undefined || first.key === null) return false
  const holds = (list: Keyed, key: string | null) =>
    key !== null && (list ?? []).some((entry) => entry.key === key)
  switch (first.what) {
    case 'metaField':
      return holds(config.work_meta_fields, first.key)
    case 'action':
      return holds(config.prompts, first.key)
    case 'styleType':
      return holds(config.style_types, first.key)
    case 'workKind': {
      const kind = config.work_kinds.find((entry) => entry.key === first.key)
      if (kind === undefined || second === undefined) return false
      switch (second.what) {
        case 'axis':
          return holds(kind.axes, second.key)
        case 'tier':
          return holds(kind.tiers, second.key)
        case 'versionRole':
          return holds(kind.version_roles, second.key)
        case 'releaseKind':
          return holds(kind.release_kinds, second.key)
        default:
          return false
      }
    }
    default:
      return false
  }
}
