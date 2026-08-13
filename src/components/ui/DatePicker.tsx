import { useState } from 'react'
import * as PopoverPrimitive from '@radix-ui/react-popover'
import { DayPicker } from 'react-day-picker'
import { Calendar as CalendarIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

interface Props {
  /// ISO date (yyyy-mm-dd) or an empty string for "not picked yet".
  value: string
  onChange: (value: string) => void
  placeholder?: string
  className?: string
  'aria-label'?: string
}

function toDate(value: string): Date | undefined {
  if (value === '') return undefined
  const [year, month, day] = value.split('-').map(Number)
  if (year === undefined || month === undefined || day === undefined) return undefined
  return new Date(year, month - 1, day)
}

function toIso(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

// A popover calendar instead of a native <input type="date">: the platform
// control cannot be styled to match the rest of the app and reads as foreign.
// The value stays ISO underneath, shown as-is — a stable format on every OS.
export function DatePicker({
  value,
  onChange,
  placeholder,
  className,
  'aria-label': ariaLabel,
}: Props) {
  const [open, setOpen] = useState(false)
  const selected = toDate(value)

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen}>
      <PopoverPrimitive.Trigger
        aria-label={ariaLabel}
        className={cn(
          'flex h-9 w-full cursor-pointer items-center gap-2 rounded-[9px] border border-line px-2.5 text-sm transition-colors hover:border-line-2',
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
      </PopoverPrimitive.Trigger>

      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          align="start"
          sideOffset={4}
          className="z-50 rounded-2xl border border-line bg-raise p-3 shadow-raise"
        >
          <DayPicker
            mode="single"
            selected={selected}
            defaultMonth={selected}
            onSelect={(date) => {
              if (date !== undefined) onChange(toIso(date))
              setOpen(false)
            }}
            classNames={{
              months: 'relative',
              month_caption: 'flex h-8 items-center px-2 text-sm font-semibold',
              nav: 'absolute top-0 right-0 flex gap-1',
              button_previous:
                'grid size-8 cursor-pointer place-items-center rounded-[9px] text-dim hover:bg-soft hover:text-text',
              button_next:
                'grid size-8 cursor-pointer place-items-center rounded-[9px] text-dim hover:bg-soft hover:text-text',
              chevron: 'size-4 fill-current',
              month_grid: 'mt-2 border-collapse',
              weekday: 'size-8 text-[10.5px] font-medium uppercase text-faint',
              day: 'p-0 text-center',
              day_button:
                'size-8 cursor-pointer rounded-[9px] text-sm tabular-nums hover:bg-soft',
              today: 'font-semibold text-accent-2',
              selected: '[&>button]:bg-accent [&>button]:font-semibold [&>button]:text-on-accent [&>button]:hover:bg-accent-2',
              outside: 'text-faint opacity-60',
            }}
          />
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  )
}
