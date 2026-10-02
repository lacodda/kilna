import { useTranslation } from 'react-i18next'
import type { RepeatMark as Mark } from '@/lib/api/types'
import { markLabel, repeatHint, statusOf, useRepeatMark } from '@/lib/repeats'
import { cn } from '@/lib/utils'
import { statusBadgeVariants, statusDotVariants } from '@/components/ui/status-dot'
import { Tooltip, TooltipPopup, TooltipTrigger } from '@/components/ui/tooltip'

interface Props {
  /** The work's mark; nothing is drawn without one. */
  mark: Mark | undefined
  /**
   * `dot` for a row or a chip, where the line is one word tall: a dot in the
   * level's hue, with the count beside it when there is more than one.
   * `badge` where there is room to say it - the card's header - "3 repeats"
   * on the level's tinted ground.
   */
  look?: 'dot' | 'badge'
  /**
   * Explain itself in a tooltip on hover and focus. Off inside something that
   * already opens a hover card of its own (a calendar chip), which says the
   * same in its card; two popups over one glyph is one too many.
   */
  tooltip?: boolean
  className?: string
}

/** The level's ink, for the dot's count: the same hue as the dot. */
const INK = { bad: 'text-bad', warn: 'text-warn' } as const

/**
 * The guard's mark on a work (ADR 0054): orange when it says a spent term of
 * the register, or a rare word another song said long ago; red when a song
 * out or booked within the window says the same rare word.
 *
 * Seen from everywhere a work is - the catalogue's rows, the calendar's chips
 * and its queue, the dashboard's week, the card's header - because a repeat
 * is noticed where the song is being placed or picked, not on a screen of
 * its own. A publication wears its song's mark: the clip repeats what its
 * song says.
 *
 * Colour is the emphasis and the words are the message (the rule `StatusDot`
 * is built on): the mark is an image named by the whole hint - the level,
 * the word, the other song and its day - so a reader who does not see the
 * hue, or cannot hover, hears all of it. The tooltip is the same sentence for
 * the eye, plus how many more stand behind it.
 */
export function RepeatMark({ mark, look = 'dot', tooltip = true, className }: Props) {
  const { t } = useTranslation()
  if (mark === undefined) return null

  const status = statusOf(mark.level)
  const label = markLabel(mark)

  const face =
    look === 'dot' ? (
      <span
        role="img"
        aria-label={label}
        data-repeat={mark.level}
        className={cn('inline-flex shrink-0 items-center gap-0.5', INK[status], className)}
      >
        <span aria-hidden className={statusDotVariants({ status, size: 'md' })} />
        {mark.count > 1 && (
          <span aria-hidden className="font-mono text-2xs leading-none tabular-nums">
            {mark.count}
          </span>
        )}
      </span>
    ) : (
      <span
        role="img"
        aria-label={label}
        data-repeat={mark.level}
        className={cn(statusBadgeVariants({ status }), 'shrink-0', className)}
      >
        <span aria-hidden className="inline-block size-1.5 shrink-0 rounded-full bg-current" />
        <span aria-hidden>{t('repeats.count', { count: mark.count })}</span>
      </span>
    )

  if (!tooltip) return face

  return (
    // The trigger is the mark itself, not a button around it: it sits inside
    // rows and chips that are buttons already, and a button inside a button
    // is a control no keyboard can reach on its own. Its name is the label.
    <Tooltip>
      <TooltipTrigger render={face} />
      <TooltipPopup size="wide">
        <p className={cn('font-medium', INK[status])}>{t(`repeats.level.${mark.level}`)}</p>
        <p>{repeatHint(mark.top)}</p>
        {mark.count > 1 && (
          <p className="text-dim">{t('overview.more', { count: mark.count - 1 })}</p>
        )}
      </TooltipPopup>
    </Tooltip>
  )
}

/**
 * The mark of the work `workId`, looked up in the workspace's one answer
 * (`useRepeatMarks`). A component of its own so a table asks once per row
 * that draws it, not once per cell.
 */
export function WorkRepeatMark({ workId, ...props }: Omit<Props, 'mark'> & { workId: string }) {
  return <RepeatMark mark={useRepeatMark(workId)} {...props} />
}
