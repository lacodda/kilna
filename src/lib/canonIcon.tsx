import {
  BookOpen,
  Box,
  CalendarDays,
  Lightbulb,
  List,
  MapPin,
  Quote,
  Radio,
  Sparkle,
  StickyNote,
  User,
  Users,
  type LucideIcon,
} from 'lucide-react'
import type { NoteKind } from '@/lib/api/types'

/**
 * The glyphs a kind of note may carry, by the name the profile writes (ADR
 * 0011).
 *
 * A short, closed list, for the reason the actions and the styles have one:
 * the name lives in a JSON document the author edits by hand. A name outside
 * it draws the generic note rather than nothing.
 */
const CANON_ICONS: Record<string, LucideIcon> = {
  radio: Radio,
  user: User,
  'map-pin': MapPin,
  box: Box,
  users: Users,
  calendar: CalendarDays,
  sparkle: Sparkle,
  book: BookOpen,
  list: List,
  note: StickyNote,
  lightbulb: Lightbulb,
  quote: Quote,
}

/** The names, for the reference and for a picker. */
export const CANON_ICON_NAMES = Object.keys(CANON_ICONS)

export function canonIconOf(kind: Pick<NoteKind, 'icon'> | undefined): LucideIcon {
  const name = kind?.icon
  return (name !== undefined && name !== null ? CANON_ICONS[name] : undefined) ?? StickyNote
}
