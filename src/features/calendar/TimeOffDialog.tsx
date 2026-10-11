import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { CalendarOff, Pencil, Trash2 } from 'lucide-react'
import type { ScheduledRelease, TimeOff } from '@/lib/api/types'
import { createTimeOff, deleteTimeOff, updateTimeOff } from '@/lib/api/timeOff'
import { announceEdited } from '@/lib/edited'
import { formatDay } from '@/lib/format'
import { today } from '@/lib/month'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { bookedInside, spanLength, spanText } from '@/lib/timeOff'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'
import { DateRangePicker, type DateRange } from '@/components/ui/date-range-picker'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

/** What the dialog opens on: a day to close, or a span to edit. */
export interface TimeOffStart {
  day?: string
  editing?: string
}

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  start: TimeOffStart
  spans: readonly TimeOff[]
  /** Everything with a date, for what a span would close on. */
  slots: readonly ScheduledRelease[]
  /** Move what is planned inside a span out past it: a shift from its first
   *  day by its length, previewed on the month like any other. */
  onShiftOut: (from: string, days: number) => void
}

/**
 * The days the calendar is closed: closing a span, renaming or moving one,
 * opening it again (v0.96).
 *
 * One dialog for the whole list rather than a form per day: a holiday is
 * added once a year and looked at when the plan is made, and the list is
 * where "what is already closed" is read before a new span overlaps it. A
 * span that closes days something is booked on says so, with the way to move
 * them out past it - closing a day does not move anything by itself, the way
 * a pin does not.
 */
export function TimeOffDialog({ open, onOpenChange, start, spans, slots, onShiftOut }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const now = today()

  // The form: the span being made, or the one being edited.
  const [editing, setEditing] = useState<string | null>(null)
  const [range, setRange] = useState<DateRange>({})
  const [label, setLabel] = useState('')
  // The range the dialog opened with: a pick away from it is what a stray
  // click outside must not throw away.
  const [opened, setOpened] = useState<DateRange>({})

  // Opened again, on whatever it was opened on: a day closes that day, an
  // edit fills the form with the span. Reset in the render that opens it, so
  // a span half-typed last time does not wait in the form, and the list
  // refreshing under an open dialog leaves the form alone.
  const [seen, setSeen] = useState<TimeOffStart | null>(null)
  if (open && seen !== start) {
    setSeen(start)
    const span = spans.find((entry) => entry.id === start.editing)
    if (span !== undefined) {
      const days = { start: span.starts_on, end: span.ends_on }
      setEditing(span.id)
      setRange(days)
      setOpened(days)
      setLabel(span.label)
    } else {
      const days = start.day === undefined ? {} : { start: start.day, end: start.day }
      setEditing(null)
      setRange(days)
      setOpened(days)
      setLabel(t('timeOff.defaultLabel'))
    }
  } else if (!open && seen !== null) {
    setSeen(null)
  }

  /** Said once a span is saved: it closes days something is booked on. */
  const offerShift = (span: TimeOff) => {
    const inside = bookedInside(span, slots)
    if (inside.length === 0) return
    say.withAction(
      t('timeOff.bookedInside', { count: inside.length, title: span.label }),
      t('timeOff.moveOut'),
      () => onShiftOut(span.starts_on, spanLength(span)),
    )
  }

  const create = useAppMutation({
    mutationFn: () =>
      createTimeOff({ starts_on: range.start!, ends_on: range.end!, label: label.trim() }),
    failure: 'timeOff.saveFailed',
    refresh: refresh.timeOff,
    onSuccess: (span) => {
      say.ok(t('timeOff.closed', { title: span.label }))
      offerShift(span)
      setRange({})
      setLabel(t('timeOff.defaultLabel'))
    },
  })

  const update = useAppMutation({
    mutationFn: (id: string) =>
      updateTimeOff(id, { starts_on: range.start, ends_on: range.end, label: label.trim() }),
    failure: 'timeOff.saveFailed',
    refresh: refresh.timeOff,
    onSuccess: (span) => {
      announceEdited({
        client,
        message: t('timeOff.saved', { title: span.label }),
        refresh: refresh.timeOff,
      })
      offerShift(span)
      setEditing(null)
      setRange({})
      setLabel(t('timeOff.defaultLabel'))
    },
  })

  const remove = useAppMutation({
    mutationFn: (span: TimeOff) => deleteTimeOff(span.id).then(() => span),
    failure: 'timeOff.deleteFailed',
    refresh: refresh.timeOff,
    onSuccess: (span) => {
      if (editing === span.id) setEditing(null)
      announceEdited({
        client,
        message: t('timeOff.opened', { title: span.label }),
        refresh: refresh.timeOff,
      })
    },
  })

  const whole = range.start !== undefined && range.end !== undefined
  const ready = whole && label.trim() !== ''
  const saving = create.isPending || update.isPending

  // Still to come first, the ones over by today after them: a list read to
  // plan ahead, where last year's holiday is the least of the questions.
  const ahead = spans.filter((span) => span.ends_on >= now)
  const past = spans.filter((span) => span.ends_on < now)

  const row = (span: TimeOff) => {
    const inside = bookedInside(span, slots)
    return (
      <li
        key={span.id}
        className={cn(
          'flex items-center gap-2 rounded-md px-2 py-1.5',
          editing === span.id && 'bg-accent-soft',
          span.ends_on < now && 'opacity-60',
        )}
      >
        <CalendarOff aria-hidden className="size-4 shrink-0 text-faint" />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{span.label}</p>
          <p className="font-mono text-xs text-faint tabular-nums">
            {spanText(span, (day) => formatDay(day))}
            {' · '}
            {t('timeOff.days', { count: spanLength(span) })}
          </p>
        </div>
        {inside.length > 0 && span.ends_on >= now && (
          <Button
            size="sm"
            variant="soft"
            title={t('timeOff.bookedInside', { count: inside.length, title: span.label })}
            onClick={() => onShiftOut(span.starts_on, spanLength(span))}
          >
            {t('timeOff.moveOutCount', { count: inside.length })}
          </Button>
        )}
        <Button
          variant="icon"
          size="icon-sm"
          aria-label={t('timeOff.edit', { title: span.label })}
          title={t('timeOff.edit', { title: span.label })}
          onClick={() => {
            setEditing(span.id)
            setRange({ start: span.starts_on, end: span.ends_on })
            setLabel(span.label)
          }}
        >
          <Pencil aria-hidden />
        </Button>
        <Button
          variant="icon"
          size="icon-sm"
          aria-label={t('timeOff.open', { title: span.label })}
          title={t('timeOff.open', { title: span.label })}
          disabled={remove.isPending}
          onClick={() => remove.mutate(span)}
        >
          <Trash2 aria-hidden />
        </Button>
      </li>
    )
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('timeOff.title')}
      description={t('timeOff.hint')}
      size="md"
      dirty={range.start !== opened.start || range.end !== opened.end}
    >
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (!ready || saving) return
          if (editing === null) create.mutate()
          else update.mutate(editing)
        }}
      >
        {/* The span across the dialog's width: both ends written out, which
            half a row cut short. A group rather than a field: the picker is
            a popover's trigger, not a control a label can name by id. */}
        <FieldGroup label={t('timeOff.daysLabel')}>
          <DateRangePicker
            value={range}
            onValueChange={setRange}
            placeholder={t('timeOff.pickDays')}
            aria-label={t('timeOff.daysLabel')}
            previousMonthLabel={t('calendar.previousMonth')}
            nextMonthLabel={t('calendar.nextMonth')}
            className="w-full"
          />
        </FieldGroup>
        <div className="flex flex-wrap items-end gap-2">
          <Field label={t('timeOff.nameLabel')} className="min-w-48 flex-1">
            <Input value={label} onChange={(event) => setLabel(event.target.value)} />
          </Field>
          {editing !== null && (
            <Button
              onClick={() => {
                setEditing(null)
                setRange({})
                setLabel(t('timeOff.defaultLabel'))
              }}
            >
              {t('timeOff.cancelEdit')}
            </Button>
          )}
          <Button type="submit" variant="primary" disabled={!ready || saving}>
            {editing === null ? t('timeOff.closeDays') : t('timeOff.save')}
          </Button>
        </div>
      </form>

      {spans.length === 0 ? (
        <p className="mt-3 text-sm text-faint">{t('timeOff.none')}</p>
      ) : (
        <div className="mt-3 flex flex-col gap-2">
          {ahead.length > 0 && <ul className="flex flex-col gap-0.5">{ahead.map(row)}</ul>}
          {past.length > 0 && (
            <>
              <h3 className="caption">{t('timeOff.past')}</h3>
              <ul className="flex flex-col gap-0.5">{past.map(row)}</ul>
            </>
          )}
        </div>
      )}
    </Dialog>
  )
}
