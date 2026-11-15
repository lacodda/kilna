import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import type { MonthlyGoal, ProfileConfig, Rhythm } from '@/lib/api/types'
import { goalChoices, goalKey, goalOf } from '@/lib/rhythm'
import { Select } from '@/components/AppSelect'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'

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
          <NumberField
            className="w-44"
            min={1}
            step={1}
            value={rhythm?.every_days ?? null}
            unit={t('editor.rhythmDaysUnit')}
            aria-label={t('editor.rhythmDays')}
            onValueChange={(days) => {
              // Clearing the field removes the rhythm entirely — "no pace"
              // is a valid answer, and the layout button explains it.
              onChange(
                days === null
                  ? null
                  : {
                      every_days: Math.max(1, Math.trunc(days)),
                      default_time: rhythm?.default_time ?? null,
                      monthly: rhythm?.monthly ?? [],
                    },
              )
            }}
          />
          <label className="ml-4 flex items-center gap-2 text-sm">
            {t('editor.rhythmTime')}
            <Input
              className="w-28"
              type="time"
              value={rhythm?.default_time ?? ''}
              disabled={rhythm == null}
              aria-label={t('editor.rhythmTime')}
              onChange={(event) => {
                if (rhythm == null) return
                onChange({
                  ...rhythm,
                  default_time: event.target.value === '' ? null : event.target.value,
                })
              }}
            />
          </label>
        </div>
      </FieldGroup>

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
