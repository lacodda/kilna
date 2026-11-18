import { createElement, type SVGProps } from 'react'
import {
  Briefcase,
  Globe,
  Hash,
  Mic,
  Newspaper,
  BookOpen,
  Box,
  CalendarDays,
  ClipboardList,
  Library,
  Lightbulb,
  List,
  MapPin,
  Quote,
  Radio,
  Sparkle,
  StickyNote,
  Target,
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
  library: Library,
  globe: Globe,
  newspaper: Newspaper,
  mic: Mic,
  hash: Hash,
  briefcase: Briefcase,
  target: Target,
  'clipboard-list': ClipboardList,
}

/** The names, for the reference and for a picker. */
export const CANON_ICON_NAMES = Object.keys(CANON_ICONS)

export function canonIconOf(kind: Pick<NoteKind, 'icon'> | undefined): LucideIcon {
  const name = kind?.icon
  return (name !== undefined && name !== null ? CANON_ICONS[name] : undefined) ?? StickyNote
}

/**
 * A kind of card's glyph, ready to draw - in a list or inside a graph's
 * drawing, where `x`, `y` and the size place it (v0.143). The choosing
 * happens in a component declared once, as `KindGlyph` does it for a door.
 */
export function CanonGlyph({
  kind,
  ...props
}: { kind: Pick<NoteKind, 'icon'> | undefined } & SVGProps<SVGSVGElement>) {
  // Made by `createElement`: the set is fixed and each glyph's identity
  // stable, which the lint cannot see through a lookup.
  return createElement(canonIconOf(kind), { 'aria-hidden': true, ...props })
}
