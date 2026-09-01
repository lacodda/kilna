import {
  AudioLines,
  Book,
  BookOpen,
  CalendarDays,
  Disc,
  Film,
  Globe,
  Image,
  Mail,
  Mic,
  Music,
  Newspaper,
  Radio,
  Rss,
  Send,
  Share2,
  Smartphone,
  Users,
  Video,
  type LucideIcon,
} from 'lucide-react'

/**
 * The glyph a release kind is drawn with.
 *
 * The profile names the kinds and the code is not allowed to know which ones
 * exist (ADR 0001) -- a clip, a beta read and a newsletter send have nothing in
 * common but the row they sit in. So the profile also names the glyph, and this
 * is the vocabulary it may name from.
 *
 * A closed set rather than a lookup into `lucide-react`: the name arrives from
 * a file the owner can edit, and an open lookup would let a typo pull down the
 * calendar and a stray string reach anything the icon module exports. Adding a
 * glyph here is a one-line change; the profile that wants it says the word.
 */
const ICONS: Readonly<Record<string, LucideIcon>> = {
  'audio-lines': AudioLines,
  book: Book,
  'book-open': BookOpen,
  disc: Disc,
  film: Film,
  globe: Globe,
  image: Image,
  mail: Mail,
  mic: Mic,
  music: Music,
  newspaper: Newspaper,
  radio: Radio,
  rss: Rss,
  send: Send,
  'share-2': Share2,
  smartphone: Smartphone,
  users: Users,
  video: Video,
}

/**
 * What the calendar draws beside a release.
 *
 * Falls back to a calendar page for a kind whose profile names no glyph, or
 * names one this set does not hold. The fallback is deliberately not blank: a
 * chip with a gap where its neighbours have a mark reads as broken, whereas a
 * neutral mark reads as "a release of some kind", which is exactly true.
 */
export function releaseIcon(icon: string | null | undefined): LucideIcon {
  if (icon == null || !Object.hasOwn(ICONS, icon)) return CalendarDays
  // `hasOwn` rather than a plain lookup with a fallback: the name comes from a
  // file the owner edits, and `ICONS['constructor']` is not undefined -- it is
  // `Object.prototype.constructor`, which React would then try to render as a
  // component. Caught by a test, not by reasoning.
  return ICONS[icon] ?? CalendarDays
}

/**
 * The glyph itself, ready to draw.
 *
 * Takes the name rather than the component so the choosing happens inside a
 * component that is declared once, at module level. `const Glyph = releaseIcon(...)`
 * at the top of a render reads to React's lint rules as a component built
 * during render -- it is not one, the set is fixed and the identity stable, but
 * a rule that has to be silenced at every call site is a rule that will be
 * silenced somewhere it mattered.
 */
export function KindGlyph({ icon, className }: { icon: string | null | undefined; className?: string }) {
  const Glyph = releaseIcon(icon)
  return <Glyph aria-hidden className={className} />
}

/** Whether the set holds a glyph by this name -- for tests and the profile editor. */
export function knownIcon(icon: string): boolean {
  return Object.hasOwn(ICONS, icon)
}

/** Every glyph name the profile may choose from, sorted for a stable listing. */
export function iconNames(): string[] {
  return Object.keys(ICONS).sort()
}
