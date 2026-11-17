import type {
  ProfileConfig,
  Vibe,
  VibeAxes,
  VibeCharacter,
  VibeComplexity,
  VibeMood,
} from '@/lib/api/types'
import { resolveLabel } from '@/lib/label'

/*
 * What a work feels like (FR-02, v0.124): three lines - its mood, its
 * character, how complex it is - each said with one of the profile's words,
 * and one mark built from all three: the mood's colour, the character's
 * shape, as many strokes as it is complex.
 *
 * The mark is worked out and never chosen: seventy-five pictures nobody could
 * tell apart at fourteen pixels are replaced by thirteen parts anybody can
 * read once they have seen the three lines.
 */

/** The three lines, in the order they are said. */
export const VIBE_LINES = ['mood', 'character', 'complexity'] as const
export type VibeLine = (typeof VIBE_LINES)[number]

/** The lines as the open profile words them; with none there is nothing to
 *  pick from and nothing is drawn. */
export function axesOf(config: ProfileConfig): VibeAxes | undefined {
  const axes = config.vibe
  if (axes === undefined || axes === null) return undefined
  return axes.moods.length > 0 && axes.characters.length > 0 && axes.complexities.length > 0
    ? axes
    : undefined
}

/** A vibe with every line standing somewhere: what a mark is drawn from. */
export interface ResolvedVibe {
  mood: VibeMood
  character: VibeCharacter
  complexity: VibeComplexity
  /** The lines the person said themselves; the rest stand at the neutral. */
  said: VibeLine[]
}

/**
 * The mark's three parts for a vibe: the word said on each line, and for a
 * line left unsaid the neutral one - the value the profile names, or the
 * middle of the line. Nothing when no line is said: a work nobody has said
 * anything of has no mark, not a neutral one.
 *
 * A key the profile no longer has reads as unsaid: the profile is the
 * owner's to change, and a mood renamed away is not an error on a row.
 */
export function resolveVibe(
  axes: VibeAxes | undefined,
  vibe: Vibe | null | undefined,
): ResolvedVibe | null {
  if (axes === undefined || vibe === undefined || vibe === null) return null
  const mood = axes.moods.find((one) => one.key === vibe.mood)
  const character = axes.characters.find((one) => one.key === vibe.character)
  const complexity = axes.complexities.find((one) => one.key === vibe.complexity)
  const said: VibeLine[] = []
  if (mood !== undefined) said.push('mood')
  if (character !== undefined) said.push('character')
  if (complexity !== undefined) said.push('complexity')
  if (said.length === 0) return null

  const middle = <T extends { key: string }>(line: T[], named: string | undefined): T =>
    line.find((one) => one.key === named) ?? line[Math.floor(line.length / 2)]!
  return {
    mood: mood ?? middle(axes.moods, axes.neutral?.mood),
    character: character ?? middle(axes.characters, axes.neutral?.character),
    complexity: complexity ?? middle(axes.complexities, axes.neutral?.complexity),
    said,
  }
}

/** What a mark says when it is pointed at: the three words, the ones the
 *  person said - "Melancholy · Raw · Complex". */
export function vibeWords(resolved: ResolvedVibe, language: string): string {
  return VIBE_LINES.filter((line) => resolved.said.includes(line))
    .map((line) => resolveLabel(resolved[line].label, language))
    .join(' · ')
}

/** A vibe with one line set, or unset when it is `undefined`; nothing when
 *  no line is left - which is how the last word taken away clears it. */
export function withLine(
  vibe: Vibe | null | undefined,
  line: VibeLine,
  key: string | undefined,
): Vibe | null {
  const next: Vibe = { ...(vibe ?? {}) }
  if (key === undefined) delete next[line]
  else next[line] = key
  return VIBE_LINES.some((one) => next[one] !== undefined && next[one] !== null) ? next : null
}
