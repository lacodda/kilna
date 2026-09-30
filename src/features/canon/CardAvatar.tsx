import { fileSrc } from '@/lib/api/assets'
import { initialOf } from '@/lib/canon'
import { cn } from '@/lib/utils'

/** The tones an avatar without a portrait is drawn in: the soft fills of the
 *  status vocabulary, each with its own ink, so they follow the theme. */
const TONES = [
  'bg-accent-soft text-accent',
  'bg-info-soft text-info',
  'bg-good-soft text-good',
  'bg-warn-soft text-warn',
  'bg-bad-soft text-bad',
  'bg-soft text-dim',
] as const

/**
 * A card's face: its newest portrait, or the first letter of its name on a
 * tone read off the name, so a list of forty cards is told apart by colour
 * before it is read.
 */
export function CardAvatar({
  title,
  portrait,
  size = 'md',
}: {
  title: string | null | undefined
  portrait: string | null
  size?: 'sm' | 'md' | 'lg'
}) {
  const box =
    size === 'sm'
      ? 'size-5.5 rounded-md text-2xs'
      : size === 'md'
        ? 'size-8 rounded-lg text-xs'
        : 'size-11 rounded-xl text-sm'
  if (portrait !== null) {
    return (
      <img
        src={fileSrc(portrait)}
        alt=""
        className={cn('shrink-0 object-cover', box)}
        draggable={false}
      />
    )
  }
  return (
    <span
      aria-hidden
      className={cn('grid shrink-0 place-items-center font-bold', box, toneOf(title ?? ''))}
    >
      {initialOf(title)}
    </span>
  )
}

/** A steady tone for a name: the same name, the same colour, every time. */
function toneOf(name: string): string {
  let hash = 0
  for (const char of name) hash = (hash * 31 + (char.codePointAt(0) ?? 0)) >>> 0
  return TONES[hash % TONES.length]!
}
