import {
  Clapperboard,
  Eye,
  Film,
  Gauge,
  Image,
  Lightbulb,
  ListOrdered,
  Music,
  Palette,
  PenLine,
  ScrollText,
  Sparkles,
  SpellCheck,
  Tags,
  Wand2,
  type LucideIcon,
} from 'lucide-react'
import type { PromptTemplate } from '@/lib/api'

/**
 * The glyphs an AI action may carry, by the name the profile writes.
 *
 * A short, closed list, for the reason the marks have one: the name lives in
 * a JSON document the author edits by hand, and a list they can read in the
 * profile reference beats a thousand names they cannot. A name outside it
 * draws the default spark — an action is never left unmarked, it just falls
 * back to the generic one.
 *
 * Actions got icons in v0.74.2. A row of five buttons reading "Critique the
 * lyrics / Suggest a revision / Draft a style prompt / Alternative titles /
 * Score the song" was five sentences where the eye wanted five marks: the
 * owner said the buttons "simply do not read and are not remembered". The
 * glyph is what makes one findable at a glance, the short name says which it
 * is, and the long description moved into the tooltip.
 */
export const ACTION_ICONS: Record<string, LucideIcon> = {
  sparkles: Sparkles,
  wand: Wand2,
  pen: PenLine,
  'spell-check': SpellCheck,
  scroll: ScrollText,
  tags: Tags,
  gauge: Gauge,
  music: Music,
  film: Film,
  clapperboard: Clapperboard,
  image: Image,
  list: ListOrdered,
  lightbulb: Lightbulb,
  palette: Palette,
  eye: Eye,
}

/** The names, for the reference and for a picker. */
export const ACTION_ICON_NAMES = Object.keys(ACTION_ICONS)

export function actionIconOf(action: Pick<PromptTemplate, 'icon'>): LucideIcon {
  return (action.icon !== undefined && action.icon !== null
    ? ACTION_ICONS[action.icon]
    : undefined) ?? Sparkles
}
