import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Dialog } from '@/components/AppDialog'
import { DatePicker } from '@/components/DatePicker'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { NumberField } from '@/components/ui/number-field'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'

/** What the dialog opens on: the day the shift starts from, and by how much. */
export interface ShiftStart {
  from: string
  /** Positive is later, negative earlier. */
  days: number
}

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  start: ShiftStart
  /** Draw where everything would go; nothing moves until that is approved. */
  onPreview: (from: string, days: number) => void
  previewing: boolean
}

type Direction = 'later' | 'earlier'

/** The longest shift either way - the backend's limit, said before it is hit. */
const MAX_DAYS = 366

/**
 * Shifting the plan (v0.96): everything planned from a day on, by a number
 * of days, later or earlier - a week lost to illness, a fortnight away added
 * after the plan was made.
 *
 * Asks three things and moves nothing. What it answers with is a picture on
 * the month - every moved release a ghost on its new day, the bar above
 * saying how many and what stays - and only booking that picture moves
 * anything, the way a layout is booked.
 */
export function ShiftDialog({ open, onOpenChange, start, onPreview, previewing }: Props) {
  const { t } = useTranslation()
  const [from, setFrom] = useState(start.from)
  const [days, setDays] = useState<number | null>(Math.abs(start.days))
  const [direction, setDirection] = useState<Direction>(start.days < 0 ? 'earlier' : 'later')

  // Each opening starts from what it was opened on: reset in the render that
  // opens it, so the first paint already shows the day it was opened from.
  const [seen, setSeen] = useState<ShiftStart | null>(null)
  if (open && seen !== start) {
    setSeen(start)
    setFrom(start.from)
    setDays(Math.abs(start.days))
    setDirection(start.days < 0 ? 'earlier' : 'later')
  } else if (!open && seen !== null) {
    setSeen(null)
  }

  const ready = from !== '' && days !== null && days >= 1 && days <= MAX_DAYS

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('shift.title')}
      description={t('shift.hint')}
      size="sm"
      footer={
        <Button
          variant="primary"
          disabled={!ready || previewing}
          onClick={() => {
            if (!ready) return
            onPreview(from, direction === 'later' ? days : -days)
          }}
        >
          {t('shift.preview')}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <FieldGroup label={t('shift.fromLabel')} help={t('shift.fromHelp')}>
          <DatePicker
            value={from}
            onChange={setFrom}
            placeholder={t('shift.fromLabel')}
            aria-label={t('shift.fromLabel')}
          />
        </FieldGroup>
        <FieldGroup label={t('shift.byLabel')}>
          <div className="flex flex-wrap items-center gap-2">
            <NumberField
              value={days}
              onValueChange={setDays}
              min={1}
              max={MAX_DAYS}
              unit={t('shift.daysUnit', { count: days ?? 0 })}
              aria-label={t('shift.byLabel')}
              className="w-36"
            />
            <SegmentedControl
              aria-label={t('shift.direction')}
              value={direction}
              onValueChange={(next) => setDirection(next as Direction)}
            >
              <Segment value="later">{t('shift.later')}</Segment>
              <Segment value="earlier">{t('shift.earlier')}</Segment>
            </SegmentedControl>
          </div>
        </FieldGroup>
      </div>
    </Dialog>
  )
}
