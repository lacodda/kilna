import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { PeriodMonth, PeriodSlot } from '@/lib/api/types'
import { layOutPeriod } from '@/lib/api/workPlans'
import { formatDayOfMonth, formatMonth, formatWeekday } from '@/lib/format'
import { monthOf, shiftMonth, today } from '@/lib/month'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Dialog } from '@/components/AppDialog'
import { Select } from '@/components/AppSelect'
import { Loaded } from '@/components/Loaded'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

/** How much of the year is laid out at once. */
const LENGTHS = { month: 1, quarter: 3, year: 12 } as const
type Length = keyof typeof LENGTHS

/** No collection: the drafts gather nowhere. */
const NONE = ''

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * A month, a quarter or a year laid out ahead (v0.152, FR-20 20.3): the days
 * the profile's rhythm gives - its weekdays, its target, a floating peak for
 * a short month - shown before anything is made, a day off and a day already
 * taken said and passed over; then one press makes a collection for each
 * month and a draft for each day, planned on it. One undo takes it back.
 */
export function PlanPeriodDialog({ open, onOpenChange }: Props) {
  const { t, i18n } = useTranslation()
  const { config } = useProfile()
  const next = shiftMonth(monthOf(today()), 1)
  const [from, setFrom] = useState(`${String(next.year)}-${String(next.month).padStart(2, '0')}`)
  const [length, setLength] = useState<Length>('month')
  const [kind, setKind] = useState(config.work_kinds[0]?.key ?? '')
  const [gather, setGather] = useState(config.collection_kinds[0]?.key ?? NONE)
  const valid = /^\d{4}-(0[1-9]|1[0-2])$/.test(from)
  const plan = useQuery({ ...queries.periodPlan(from, LENGTHS[length]), enabled: open && valid })

  const kindLabel = sayLabel(config.work_kinds.find((one) => one.key === kind)?.label ?? kind)
  const monthTitle = (month: string) => {
    const { year, month: number } = monthOf(month)
    return formatMonth(year, number, i18n.language)
  }
  const free = (slot: PeriodSlot) => !slot.closed && slot.taken === null
  const months: PeriodMonth[] = plan.data?.months ?? []
  const count = months.reduce((sum, month) => sum + month.slots.filter(free).length, 0)

  const make = useAppMutation({
    mutationFn: () =>
      layOutPeriod({
        kind,
        collection_kind: gather === NONE ? null : gather,
        months: months.map((month) => ({
          month: month.month,
          title: monthTitle(month.month),
          drafts: month.slots.filter(free).map((slot) => ({
            date: slot.date,
            title: t('period.draftTitle', {
              kind: kindLabel,
              day: formatWeekday(slot.date, i18n.language),
            }),
          })),
        })),
      }),
    failure: 'period.failed',
    refresh: refresh.period,
    onSuccess: (made) => {
      say.ok(t('period.made', { count: made.works, collections: made.collections }))
      onOpenChange(false)
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('period.title')}
      description={t('period.description')}
      size="xl"
      footer={
        <Button
          variant="primary"
          disabled={!valid || count === 0 || kind === '' || make.isPending}
          onClick={() => make.mutate()}
        >
          {t('period.make', { count })}
        </Button>
      }
    >
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          aria-label={t('period.length')}
          value={length}
          onValueChange={(value) => setLength(value as Length)}
        >
          {(Object.keys(LENGTHS) as Length[]).map((one) => (
            <Segment key={one} value={one}>
              {t(`period.lengths.${one}`)}
            </Segment>
          ))}
        </SegmentedControl>
        <Input
          value={from}
          onChange={(event) => setFrom(event.target.value.trim())}
          aria-label={t('period.from')}
          placeholder="2027-01"
          className="w-28 font-mono"
        />
        <Select
          value={kind}
          onChange={setKind}
          options={config.work_kinds.map((one) => ({ value: one.key, label: sayLabel(one.label) }))}
          aria-label={t('period.kind')}
          className="w-40"
        />
        <Select
          value={gather}
          onChange={setGather}
          options={[
            { value: NONE, label: t('period.noCollection') },
            ...config.collection_kinds.map((one) => ({
              value: one.key,
              label: sayLabel(one.label),
            })),
          ]}
          aria-label={t('period.collectionKind')}
          className="w-48"
        />
      </div>

      {valid ? (
        <Loaded query={plan} skeleton={<SkeletonList rows={3} />} plain>
          {(data) => (
            <div className="flex flex-col gap-3">
              {data.months.map((month) => {
                const kept = month.slots.filter(free).length
                return (
                  <section key={month.month} className="flex flex-col gap-1.5">
                    <h3 className="flex items-baseline gap-2 text-sm font-semibold">
                      {monthTitle(month.month)}
                      <span className="font-mono text-xs font-normal text-faint">
                        {month.target === null
                          ? t('period.days', { count: kept })
                          : t('period.ofTarget', { count: kept, target: month.target })}
                      </span>
                    </h3>
                    <ul className="flex flex-wrap gap-1">
                      {month.slots.map((slot) => (
                        <li
                          key={slot.date}
                          title={
                            slot.closed
                              ? t('period.closed')
                              : slot.taken !== null
                                ? t('period.taken', { title: slot.taken })
                                : slot.floating
                                  ? t('period.floating')
                                  : undefined
                          }
                          className={cn(
                            'rounded-md border border-line px-1.5 py-0.5 font-mono text-xs',
                            slot.floating && 'border-dashed border-accent text-accent-2',
                            !free(slot) && 'text-faint line-through',
                          )}
                        >
                          {formatDayOfMonth(slot.date, i18n.language)}
                        </li>
                      ))}
                    </ul>
                  </section>
                )
              })}
            </div>
          )}
        </Loaded>
      ) : (
        <p className="text-sm text-bad">{t('period.badMonth')}</p>
      )}
    </Dialog>
  )
}
