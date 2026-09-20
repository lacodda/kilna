import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Calendar as CalendarIcon } from 'lucide-react'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { Calendar } from '@/components/ui/calendar'
import { usePopupContainer } from '@/components/ui/layer'
import { cn } from '@/lib/utils'

interface Props {
  /// ISO date (yyyy-mm-dd) or an empty string for "not picked yet".
  value: string
  onChange: (value: string) => void
  placeholder?: string
  className?: string
  'aria-label'?: string
}

/*
 * A popover calendar instead of a native `<input type="date">`: the platform
 * control cannot be styled to match the rest of the app and reads as foreign.
 *
 * The month and the panel are dowel's, and this file is the app's own shape
 * around them - the empty string for "no date", and the date shown as the ISO
 * string it is stored as. The line's own DatePicker writes the date out long
 * ("30 September 2026"), which is kinder to read in isolation and wrong here:
 * a slot is compared against the grid, the queue and the release row, all of
 * which are `yyyy-mm-dd` in tabular figures, and one long-form date among them
 * is the one thing on the screen that has to be translated back before it can
 * be compared.
 *
 * Until v0.69 the popover was Radix - the only Radix left in the window, every
 * other overlay being Base UI on dowel's tokens. It put `z-index: 50` inline
 * on a wrapper this app never rendered, which is why the month drew under the
 * dialog that opened it no matter what class the content carried. `layer.tsx`
 * is the other half of that fix.
 */
export function DatePicker({
  value,
  onChange,
  placeholder,
  className,
  'aria-label': ariaLabel,
}: Props) {
  const { t, i18n } = useTranslation()
  const [open, setOpen] = useState(false)
  // Empty means no date, and the month has no opinion about an empty string.
  const selected = value === '' ? undefined : value
  const container = usePopupContainer()

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={ariaLabel}
        className={cn(
          'flex h-9 w-full cursor-pointer items-center gap-2 rounded-md border border-line px-2.5 text-sm transition-colors hover:border-line-2',
          'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-accent',
          className,
        )}
      >
        <CalendarIcon aria-hidden className="size-4 shrink-0 text-faint" />
        {selected === undefined ? (
          <span className="text-faint">{placeholder}</span>
        ) : (
          <span className="font-mono tabular-nums">{value}</span>
        )}
      </PopoverTrigger>

      <PopoverPopup container={container} arrow={false} className="w-auto p-3">
        <Calendar
          value={selected}
          // The interface's language, not the machine's. Left to itself the
          // month reads `navigator.language`, and a Russian interface on an
          // en-US machine drew its weeks from Sunday while the big month grid
          // beside it - which takes `i18n.language` - drew them from Monday.
          // Two calendars on one screen disagreeing about which day a week
          // starts on is the kind of thing that is read as a bug in the data.
          locale={i18n.language}
          aria-label={ariaLabel ?? placeholder}
          previousMonthLabel={t('calendar.previousMonth')}
          nextMonthLabel={t('calendar.nextMonth')}
          onValueChange={(next) => {
            onChange(next)
            // Picking a day is the whole errand: the panel closes rather than
            // waiting for a second, dismissing click.
            setOpen(false)
          }}
        />
      </PopoverPopup>
    </Popover>
  )
}
