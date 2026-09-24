import { createContext, useContext } from 'react'
import { useQuery } from '@tanstack/react-query'
import i18n from '@/i18n'
import {
  getWork,
  type Axis,
  type Kind,
  type Label,
  type Profile,
  type ProfileConfig,
  type ReleaseKind,
  type SceneBlock,
  type Status,
  type StyleType,
  type Tier,
  type VersionRole,
} from '@/lib/api'
import { keys } from '@/lib/query'
import { resolveLabel } from '@/lib/label'

// The active profile is the vocabulary every screen speaks in, so it is read
// once and shared rather than fetched per component.
export const ProfileContext = createContext<Profile | null>(null)

export function useProfile(): Profile {
  const profile = useContext(ProfileContext)
  if (profile === null) {
    throw new Error('useProfile used outside a loaded workspace')
  }
  return profile
}

/**
 * One word of the craft's vocabulary, in the language the window is in.
 *
 * A shipped profile carries its words in both languages (`{ en, ru }`); a word
 * the author typed is one string and stays exactly as typed, in whatever
 * language they typed it. So this reads: the interface language if the word
 * has it, else English as the language every shipped profile is written in,
 * else whatever the map does hold — a profile carrying only Russian should
 * show Russian rather than nothing.
 *
 * Deliberately NOT a fallback chain onto the key: a key is a machine's name
 * for the word, and `version_roles.lyrics` on screen is worse than the English
 * the author would at least recognise.
 */
export function say(label: Label | null | undefined, language?: string): string {
  return resolveLabel(label, language ?? i18n.resolvedLanguage ?? 'en')
}

// Label for a key from one of the profile's vocabularies, falling back to the
// raw key so an unknown value is visible rather than blank.
export function labelOf(kinds: { key: string; label: Label }[], key: string): string {
  const found = kinds.find((kind) => kind.key === key)
  return found === undefined ? key : say(found.label)
}

/** The vocabulary of one kind of work, every list present (empty when the
    kind leaves it out). */
export interface Vocabulary {
  axes: Axis[]
  tiers: Tier[]
  version_roles: VersionRole[]
  release_kinds: ReleaseKind[]
  statuses: Status[]
  shot_types: Kind[]
  scene_blocks: SceneBlock[]
  cover_blocks: SceneBlock[]
}

const NOTHING: Vocabulary = {
  axes: [],
  tiers: [],
  version_roles: [],
  release_kinds: [],
  statuses: [],
  shot_types: [],
  scene_blocks: [],
  cover_blocks: [],
}

/**
 * What the profile says about works of `kind`.
 *
 * Since v0.57 the vocabulary belongs to the kind, not the profile: a song and
 * a video are judged on different axes and go out through different doors.
 * A kind the profile does not know reads as empty rather than as someone
 * else's — nothing to score, no roles, no releases — so a work whose kind was
 * removed still opens.
 */
export function vocabularyOf(config: ProfileConfig, kind: string | undefined): Vocabulary {
  const found = kind === undefined ? undefined : config.work_kinds.find((k) => k.key === kind)
  if (found === undefined) return NOTHING
  return {
    axes: found.axes ?? [],
    tiers: found.tiers ?? [],
    version_roles: found.version_roles ?? [],
    release_kinds: found.release_kinds ?? [],
    statuses: found.statuses ?? [],
    shot_types: found.shot_types ?? [],
    scene_blocks: found.scene_blocks ?? [],
    cover_blocks: found.cover_blocks ?? [],
  }
}

/**
 * The types a style brick can be. On the profile rather than on a kind: the
 * same character stands in the videos and in the shorts (ADR 0031).
 *
 * Empty means the craft has no style dictionary, and the screen that keeps one
 * does not appear — the same rule a kind with no `shot_types` follows.
 */
export function styleTypesOf(config: ProfileConfig): StyleType[] {
  return config.style_types ?? []
}

/**
 * Whether works of `kind` have a storyboard: the kind names kinds of shot or
 * prompt blocks. A song does not, and its card draws no Scenes tab.
 */
export function hasScenes(config: ProfileConfig, kind: string | undefined): boolean {
  const vocabulary = vocabularyOf(config, kind)
  return vocabulary.shot_types.length > 0 || vocabulary.scene_blocks.length > 0
}

/**
 * One list across every kind, once per key, first label wins.
 *
 * For the screens that speak to every kind at once — the catalogue's filters,
 * the calendar's kind bar, a batch moving many works — and for nothing that
 * judges a single work: those read the work's own kind.
 */
export function allOf<K extends keyof Vocabulary>(config: ProfileConfig, list: K): Vocabulary[K] {
  const seen = new Set<string>()
  const out: { key: string; label: Label }[] = []
  for (const kind of config.work_kinds) {
    for (const entry of (kind[list] ?? []) as { key: string; label: Label }[]) {
      if (seen.has(entry.key)) continue
      seen.add(entry.key)
      out.push(entry)
    }
  }
  return out as Vocabulary[K]
}

/** The kind of a work, once it is known; `undefined` while loading. */
export function useWorkKind(workId: string | undefined): string | undefined {
  const work = useQuery({
    queryKey: keys.work(workId ?? ''),
    queryFn: () => getWork(workId!),
    enabled: workId !== undefined,
  })
  return work.data?.kind
}

/** The vocabulary of a work's kind, empty until the work is known. */
export function useVocabulary(workId: string | undefined): Vocabulary {
  const profile = useProfile()
  const kind = useWorkKind(workId)
  return vocabularyOf(profile.config, kind)
}
