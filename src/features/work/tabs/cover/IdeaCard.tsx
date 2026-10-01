import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Star, Trash2, X } from 'lucide-react'
import { fileSrc } from '@/lib/api/assets'
import type { IdeaLook, IdeaSource, IdeaVerdict } from '@/lib/api/types'
import { styleName } from '@/lib/styleBrick'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Skeleton } from '@/components/ui/skeleton'
import { SchemeView } from '@/features/cover/SchemeView'

interface Props {
  /** Where it came from; a neighbour's cover not on the board yet is
   *  `offered`. */
  source: IdeaSource | 'offered'
  /** The neighbour it came from, for a copied or an offered cover. */
  from?: string | null
  angle: string
  headline: string
  /** What the cover says, in the person's words. */
  idea: string
  look: IdeaLook
  /** A neighbour's final picture, drawn in place of the scheme. */
  picture?: string | null
  verdict: IdeaVerdict | null
  /** Arrived while the board was watched. */
  fresh?: boolean
  busy?: boolean
  onVerdict: (verdict: IdeaVerdict | null) => void
  onTake: () => void
  /** Off the board - offered on a turned-down idea only. */
  onDelete?: () => void
}

/**
 * One card of the board of ideas (the mockup's `.icard`): where it came from
 * and its angle across the top, the built frame drawn small with the style's
 * picture in its corner, the headline and the idea, what it is built from in
 * the mono line, and the three things a person does with it - star it, turn
 * it down, take it into the constructor.
 *
 * A star rings the card in the accent; a turned-down card steps back. The
 * two are one verdict: starring a turned-down idea takes the "not that" back.
 */
export function IdeaCard({
  source,
  from,
  angle,
  headline,
  idea,
  look,
  picture,
  verdict,
  fresh,
  busy,
  onVerdict,
  onTake,
  onDelete,
}: Props) {
  const { t } = useTranslation()
  const starred = verdict === 'star'
  const rejected = verdict === 'rejected'
  const title = headline.trim() || look.hero || t(`ideas.untitled.${source}`)
  const built = [
    [look.style && styleName(look.style), look.background && styleName(look.background)],
    [
      look.layout && t(`cover.layout.${look.layout}`),
      look.mark && t('ideas.mark', { code: look.mark.code ?? look.mark.name }),
    ],
  ]
    .map((line) => line.filter(Boolean).join(' · '))
    .filter((line) => line !== '')

  return (
    <article
      aria-label={title}
      className={cn(
        'relative flex flex-col gap-1.5 rounded-lg border border-line bg-raise px-2.5 pt-2 pb-2.5',
        starred && 'border-accent ring-1 ring-accent',
        rejected && 'opacity-45',
      )}
    >
      <div className="flex min-w-0 items-center gap-1.5">
        <SourceBadge source={source} from={from ?? null} />
        {angle.trim() !== '' && (
          <span className="truncate font-mono text-2xs tracking-caption text-faint uppercase">
            {angle}
          </span>
        )}
        {fresh === true && (
          <Chip variant="good" className="ml-auto shrink-0">
            {t('ideas.fresh')}
          </Chip>
        )}
      </div>

      {/* The picture's box is the card's 16:9 whatever the board's shape: a
          tall frame is drawn inside it at its own ratio. Absolutely placed,
          so the drawing measures a box of a definite size - a percentage
          height in a grid cell resolved to the drawing's own, and a 9:16
          frame came out full width and cut off at the bottom. */}
      <div className="relative aspect-video overflow-hidden rounded-md bg-soft">
        <div className="absolute inset-0 flex items-center justify-center p-1">
          {picture != null ? (
            <img src={picture} alt="" className="size-full object-contain" draggable={false} />
          ) : look.scheme !== null ? (
            <SchemeView scheme={look.scheme} label={t('ideas.scheme')} fit className="size-full" />
          ) : (
            <span className="px-3 text-center text-2xs text-faint">{t('ideas.noFrame')}</span>
          )}
        </div>
        {look.style?.picture != null && (
          <img
            src={fileSrc(look.style.picture.path)}
            alt=""
            className="absolute right-1.5 bottom-1.5 size-8.5 rounded-md object-cover ring-2 ring-raise"
            draggable={false}
          />
        )}
      </div>

      <h4 className="text-sm leading-tight font-semibold">{title}</h4>
      {idea.trim() !== '' && <p className="line-clamp-4 text-xs text-dim">{idea}</p>}
      {built.length > 0 && (
        <p className="font-mono text-2xs leading-normal text-faint uppercase">
          {built.map((line) => (
            <span key={line} className="block">
              {line}
            </span>
          ))}
        </p>
      )}

      <div className="mt-auto flex items-center gap-1.5 pt-1">
        <Verdict
          label={t('ideas.star')}
          on={starred}
          disabled={busy}
          onPress={() => onVerdict(starred ? null : 'star')}
        >
          <Star aria-hidden />
        </Verdict>
        <Verdict
          label={t('ideas.reject')}
          on={rejected}
          disabled={busy}
          onPress={() => onVerdict(rejected ? null : 'rejected')}
        >
          <X aria-hidden />
        </Verdict>
        {rejected && onDelete !== undefined && (
          <Button
            size="icon-sm"
            variant="icon"
            aria-label={t('ideas.delete')}
            title={t('ideas.delete')}
            disabled={busy}
            onClick={onDelete}
          >
            <Trash2 aria-hidden />
          </Button>
        )}
        <Button size="xs" variant="ghost" className="ml-auto" disabled={busy} onClick={onTake}>
          {t('ideas.take')}
        </Button>
      </div>
    </article>
  )
}

/** A verdict's button: pressed while the idea has it. */
function Verdict({
  label,
  on,
  disabled,
  onPress,
  children,
}: {
  label: string
  on: boolean
  disabled?: boolean
  onPress: () => void
  children: ReactNode
}) {
  return (
    <Button
      size="icon-sm"
      variant={on ? 'soft' : 'ghost'}
      aria-label={label}
      aria-pressed={on}
      title={label}
      disabled={disabled}
      onClick={onPress}
    >
      {children}
    </Button>
  )
}

/** Where an idea came from, as the card's first word. */
function SourceBadge({ source, from }: { source: Props['source']; from: string | null }) {
  const { t } = useTranslation()
  const tone =
    source === 'own'
      ? 'bg-info-soft text-info'
      : source === 'refined'
        ? 'bg-good-soft text-good'
        : source === 'sibling' || source === 'offered'
          ? 'bg-warn-soft text-warn'
          : 'bg-soft text-dim'
  const word =
    (source === 'sibling' || source === 'offered') && from !== null
      ? t('ideas.source.from', { title: from })
      : t(`ideas.source.${source}`)
  return (
    <span
      className={cn(
        'max-w-1/2 shrink-0 truncate rounded-xs px-1.5 py-px font-mono text-2xs tracking-caption uppercase',
        tone,
      )}
    >
      {word}
    </span>
  )
}

/** A card while its idea is being written. */
export function IdeaSkeleton() {
  return (
    <div
      aria-hidden
      className="flex flex-col gap-1.5 rounded-lg border border-line bg-raise px-2.5 pt-2 pb-2.5"
    >
      <Skeleton className="h-3.5 w-1/2" />
      <Skeleton className="aspect-video w-full" />
      <Skeleton className="h-4 w-4/5" />
      <Skeleton className="h-12 w-full" />
      <Skeleton className="h-3 w-3/5" />
    </div>
  )
}
