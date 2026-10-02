import { useState, type HTMLAttributes, type ImgHTMLAttributes, type ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * Avatar and AvatarGroup.
 *
 * A person, in the space of a word. Every screen that lists people needs one,
 * and the three things that go wrong with it are always the same:
 *
 * - **The picture fails to load** and a broken-image glyph appears where a
 *   face was. The fallback is not a nicety; it is the state this component
 *   spends most of its life in, because half the people in any list have no
 *   picture at all.
 * - **The initials are cut from the wrong end.** Splitting a name on spaces
 *   and taking the first letter of each part is the version everyone writes,
 *   and it turns a one-word name into one letter, a hyphenated surname into
 *   three, and a name in a script without spaces into whatever fell out.
 * - **It is announced twice.** A picture with the person's name as `alt`, next
 *   to the person's name as text, reads the name twice to a screen reader.
 *
 * So the picture is `alt=""` and the name lives in the container's label; the
 * initials are computed by a rule that says what it does; and the fallback is
 * the default rather than the exception.
 *
 * What it does not do is choose a colour from the name. A hash of a string
 * into a hue looks charming on a design page and produces, in a real list,
 * colours that collide with the status vocabulary - a face is not `--bad` -
 * and that differ between products for the same person. The ground is the
 * neutral `--soft`, always, and identity comes from the picture or the
 * letters.
 */

export const avatarVariants = cva(
  'relative inline-flex shrink-0 items-center justify-center overflow-hidden bg-soft font-medium text-dim select-none',
  {
    variants: {
      size: {
        xs: 'size-5 text-2xs',
        sm: 'size-6 text-2xs',
        md: 'size-8 text-xs',
        lg: 'size-10 text-sm',
        xl: 'size-14 text-lg',
      },
      shape: {
        circle: 'rounded-full',
        /* For a thing rather than a person - a project, a repository. Square
         * is how every other tool says "this is not a face", and borrowing
         * that saves the product a caption. */
        square: 'rounded-md',
      },
    },
    defaultVariants: { size: 'md', shape: 'circle' },
  },
)

/**
 * The letters shown when there is no picture.
 *
 * Exported because it is the part a product is most likely to want on its own -
 * in a mention, in a chart legend - and because a rule you can call is a rule
 * you can test.
 *
 * What it does, and why:
 *
 * - Words are what remain after whitespace; punctuation between them (a
 *   hyphenated surname, an initial with a dot) does not make a new word, so
 *   `Anne-Marie Dubois` is `AD` and not `AMD`.
 * - Two letters when there are two or more words, taken from the first and the
 *   *last* - the last word is the family name in most of the line's languages,
 *   and a middle name has no business on the tile.
 * - Two letters from a single word, because one letter on its own is a smaller
 *   difference between two people than the tile is wide.
 * - Case is raised at the end, once, so a name in a script without case comes
 *   through unchanged rather than mangled.
 *
 * Uses the string iterator rather than `charAt`: a name may begin with an
 * emoji or a character outside the basic plane, and taking half of a surrogate
 * pair puts a replacement glyph on the tile.
 */
export function initialsOf(name: string, count = 2): string {
  const words = name.trim().split(/\s+/).filter(Boolean)
  const first = words[0]
  const last = words[words.length - 1]
  if (first === undefined || last === undefined) return ''

  const firstOf = (word: string) => [...word][0] ?? ''

  const letters =
    words.length === 1 ? [...first].slice(0, count) : [firstOf(first), firstOf(last)].slice(0, count)

  return letters.join('').toLocaleUpperCase()
}

export interface AvatarProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, 'children'>,
    VariantProps<typeof avatarVariants> {
  /** Who or what this stands for. Always required: it is the name a reader who
   * does not see the picture gets, and the source of the initials. */
  name: string
  /** The picture. Absent, or failing to load, falls back to the initials. */
  src?: string
  /** Anything to draw instead of the initials when there is no picture - an
   * icon for a thing that is not a person. */
  fallback?: ReactNode
  /** Passed to the `<img>`, for `loading` or `referrerPolicy`. */
  imgProps?: Omit<ImgHTMLAttributes<HTMLImageElement>, 'src' | 'alt' | 'onError'>
}

export function Avatar({
  name,
  src,
  fallback,
  size,
  shape,
  imgProps,
  className,
  ...props
}: AvatarProps) {
  // A picture that 404s is the common case, not the odd one: profile hosts
  // expire links. Without this the tile shows a browser's broken-image glyph,
  // which is worse than the initials it was covering.
  const [failed, setFailed] = useState(false)
  const showImage = Boolean(src) && !failed

  return (
    <span
      className={cn(avatarVariants({ size, shape }), className)}
      // The tile as a whole is the picture of the person. The `<img>` inside
      // carries `alt=""` so the name is announced once, from here.
      role="img"
      aria-label={name}
      {...props}
    >
      {showImage ? (
        <img
          {...imgProps}
          src={src}
          alt=""
          onError={() => setFailed(true)}
          className={cn('size-full object-cover', imgProps?.className)}
        />
      ) : (
        (fallback ?? initialsOf(name))
      )}
    </span>
  )
}

export interface AvatarGroupProps extends HTMLAttributes<HTMLDivElement> {
  /** How many tiles to show before the rest become a count. */
  max?: number
  /** The people. Given as data rather than as children, because the group has
   * to know how many there are to say how many are hidden. */
  people: Array<{ name: string; src?: string }>
  size?: AvatarProps['size']
  /** What the whole row is called, for a screen reader - "Assignees", in the
   * product's word. */
  label?: string
}

/**
 * A row of faces, overlapping, with a count for the rest.
 *
 * The overlap is what makes this a group rather than a list: it says "these
 * belong to one thing" in less width than the names would take. The order is
 * the caller's, and the first tile is on top - the eye reads left to right and
 * the leftmost should be the whole one.
 *
 * The count tile is not an avatar. It has no name behind it, so it is not
 * `role="img"` and gets no label of its own; the group's label covers the row,
 * and `+3` is read as the text it is.
 */
export function AvatarGroup({
  people,
  max = 4,
  size = 'sm',
  label,
  className,
  ...props
}: AvatarGroupProps) {
  const shown = people.slice(0, Math.max(0, max))
  const hidden = people.length - shown.length

  return (
    <div
      className={cn('flex items-center', className)}
      role={label ? 'group' : undefined}
      aria-label={label}
      {...props}
    >
      {shown.map((person, index) => (
        <Avatar
          key={`${person.name}-${index}`}
          name={person.name}
          src={person.src}
          size={size}
          // The ring is the page's own background, so each tile cuts a clean
          // edge out of the one behind it. `-ml` on all but the first, because
          // the first has nothing to overlap.
          //
          // The stack runs the other way from the DOM: later siblings paint on
          // top by default, which would put the last face over the first. The
          // eye reads left to right and expects the leftmost tile whole, so
          // the order is reversed explicitly.
          style={{ zIndex: shown.length - index }}
          className={cn('relative ring-2 ring-bg', index > 0 && '-ml-2')}
        />
      ))}
      {hidden > 0 && (
        <span
          className={cn(avatarVariants({ size, shape: 'circle' }), 'relative -ml-2 ring-2 ring-bg')}
          // Behind every face, at the end of the row.
          style={{ zIndex: 0 }}
        >
          +{hidden}
        </span>
      )}
    </div>
  )
}
