import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import type { MonthlyGoal, ProfileConfig, Rhythm } from '@/lib/api/types'
import type { Weekday } from '@/lib/api/generated/Weekday'
import { goalChoices, goalKey, goalOf } from '@/lib/rhythm'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { TimeField } from '@/components/ui/time-field'
import { NumberField } from '@/components/ui/number-field'

/** The days of the week, in a week's order. */
const WEEKDAYS: readonly Weekday[] = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun']

/**
 * How often releases go out, at what time of day by default, and what a
 * month should hold (v0.111).
 *
 * The pace is one number and says nothing of what goes out at it; the goals
 * under it say what a month is held to - so many of a tier, so many of a kind
 * of work - and the dashboard says which one a month falls short of. They
 * belong to the rhythm and stand only while there is one: a profile with no
 * pace sets no goals for it.
 */
export function RhythmEditor({
  config,
  onChange,
}: {
  config: Pick<ProfileConfig, 'rhythm' | 'work_kinds'>
  onChange: (rhythm: Rhythm | null) => void
}) {
  const { t } = useTranslation()
  const rhythm = config.rhythm
  const goals = rhythm?.monthly ?? []
  const choices = goalChoices(config)
  const said = (choice: { what: 'tier' | 'kind'; label: string }) =>
    t(choice.what === 'tier' ? 'editor.rhythmGoalTier' : 'editor.rhythmGoalKind', {
      label: choice.label,
    })
  // What no goal counts yet: a month has one goal for each thing counted.
  const taken = new Set(goals.map(goalKey))
  const free = choices.filter((choice) => !taken.has(choice.key))

  const setGoals = (monthly: MonthlyGoal[]) => {
    if (rhythm != null) onChange({ ...rhythm, monthly })
  }

  return (
    <>
      <FieldGroup label={t('editor.rhythm')} help={t('editor.rhythmHint')}>
        <div className="flex items-center gap-2">
          {/* Wide enough for its unit in every language: "дней между
              выпусками" took the number's room at w-44 (v0.150 live run). */}
          <NumberField
            className="w-72"
            min={1}
            step={1}
            value={rhythm?.every_days ?? null}
            unit={t('editor.rhythmDaysUnit')}
            aria-label={t('editor.rhythmDays')}
            onValueChange={(days) => {
              // Clearing the field removes the rhythm entirely — "no pace"
              // is a valid answer, and the layout button explains it.
              // The rest of the rhythm stays as it is: a new spacing is not a
              // new rhythm.
              onChange(
                days === null
                  ? null
                  : {
                      ...(rhythm ?? { default_time: null, monthly: [] }),
                      every_days: Math.max(1, Math.trunc(days)),
                    },
              )
            }}
          />
          <label className="ml-4 flex items-center gap-2 text-sm">
            {t('editor.rhythmTime')}
            {/* The line's own time field (v0.166), not the browser's: it
                reads a time the way it is typed - "9", "930", "9:30" - and
                tells it once, when the box is left, where the native one
                saved the rhythm on every digit. */}
            <TimeField
              className="w-28"
              value={rhythm?.default_time ?? null}
              disabled={rhythm == null}
              aria-label={t('editor.rhythmTime')}
              onValueChange={(time) => {
                if (rhythm == null) return
                onChange({ ...rhythm, default_time: time })
              }}
            />
          </label>
        </div>
      </FieldGroup>

      {/* A grid of weekdays (v0.150, FR-20): the days of the week a release
          goes out on, a month's target, what a month of too many or too few
          does, and the channel's time zone. */}
      {rhythm != null && (
        <FieldGroup label={t('editor.rhythmWeekdays')} help={t('editor.rhythmWeekdaysHint')}>
          <div className="flex flex-col items-start gap-2">
            <ChipGroup
              multiple
              aria-label={t('editor.rhythmWeekdays')}
              value={rhythm.weekdays ?? []}
              onValueChange={(next) =>
                onChange({
                  ...rhythm,
                  weekdays: WEEKDAYS.filter((day) => next.includes(day)),
                })
              }
            >
              {WEEKDAYS.map((day) => (
                <Chip key={day} value={day}>
                  {t(`editor.weekday.${day}`)}
                </Chip>
              ))}
            </ChipGroup>
            {(rhythm.weekdays ?? []).length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <NumberField
                  className="w-40"
                  min={1}
                  step={1}
                  value={rhythm.monthly_target ?? null}
                  unit={t('editor.rhythmTargetUnit')}
                  aria-label={t('editor.rhythmTarget')}
                  onValueChange={(count) =>
                    onChange({
                      ...rhythm,
                      monthly_target: count === null ? undefined : Math.max(1, Math.trunc(count)),
                    })
                  }
                />
                <Select
                  className="w-56"
                  value={rhythm.overflow ?? 'keep_all'}
                  aria-label={t('editor.rhythmOverflow')}
                  onChange={(next) =>
                    onChange({ ...rhythm, overflow: next as NonNullable<Rhythm['overflow']> })
                  }
                  options={[
                    { value: 'keep_all', label: t('editor.rhythmOverflowKeep') },
                    { value: 'skip_weakest', label: t('editor.rhythmOverflowSkip') },
                  ]}
                />
                <Select
                  className="w-60"
                  value={rhythm.shortage ?? 'keep_grid'}
                  aria-label={t('editor.rhythmShortage')}
                  onChange={(next) =>
                    onChange({ ...rhythm, shortage: next as NonNullable<Rhythm['shortage']> })
                  }
                  options={[
                    { value: 'keep_grid', label: t('editor.rhythmShortageKeep') },
                    { value: 'add_floating_peak', label: t('editor.rhythmShortageFloat') },
                  ]}
                />
              </div>
            )}
            <label className="flex items-center gap-2 text-sm">
              {t('editor.rhythmZone')}
              <Input
                className="w-56"
                value={rhythm.timezone ?? ''}
                placeholder="Europe/Berlin"
                aria-label={t('editor.rhythmZone')}
                onChange={(event) =>
                  onChange({
                    ...rhythm,
                    timezone:
                      event.target.value.trim() === '' ? undefined : event.target.value.trim(),
                  })
                }
              />
            </label>
          </div>
        </FieldGroup>
      )}

      {rhythm != null && (
        <FieldGroup label={t('editor.rhythmMonthly')} help={t('editor.rhythmMonthlyHint')}>
          <div className="flex flex-col items-start gap-1.5">
            {goals.map((goal, index) => {
              const key = goalKey(goal)
              return (
                <div key={key} className="flex items-center gap-2">
                  <Select
                    className="w-56"
                    value={key}
                    aria-label={t('editor.rhythmGoalOf')}
                    onChange={(next) =>
                      setGoals(
                        goals.map((held, at) => (at === index ? goalOf(next, held.count) : held)),
                      )
                    }
                    // Its own, and what no other goal counts: two goals of
                    // one thing is not a profile the backend keeps.
                    options={choices
                      .filter((choice) => choice.key === key || !taken.has(choice.key))
                      .map((choice) => ({ value: choice.key, label: said(choice) }))}
                  />
                  <NumberField
                    className="w-36"
                    min={1}
                    step={1}
                    value={goal.count}
                    unit={t('editor.rhythmGoalUnit')}
                    aria-label={t('editor.rhythmGoalCount')}
                    onValueChange={(count) =>
                      setGoals(
                        goals.map((held, at) =>
                          at === index ? goalOf(key, Math.max(1, Math.trunc(count ?? 1))) : held,
                        ),
                      )
                    }
                  />
                  <Button
                    variant="icon"
                    size="icon-sm"
                    title={t('editor.rhythmGoalRemove')}
                    aria-label={t('editor.rhythmGoalRemove')}
                    onClick={() => setGoals(goals.filter((_, at) => at !== index))}
                  >
                    <X aria-hidden />
                  </Button>
                </div>
              )
            })}
            <Button
              size="sm"
              variant="ghost"
              disabled={free.length === 0}
              onClick={() => {
                const first = free[0]
                if (first !== undefined) setGoals([...goals, goalOf(first.key, 1)])
              }}
            >
              <Plus aria-hidden />
              {t('editor.rhythmGoalAdd')}
            </Button>
          </div>
        </FieldGroup>
      )}
    </>
  )
}
