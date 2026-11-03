import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { setWeeklyBudget } from '@/lib/api/assistant'
import { formatCost } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { NumberField } from '@/components/ui/number-field'
import { Popover, PopoverPopup, PopoverTitle, PopoverTrigger } from '@/components/ui/popover'

/**
 * What the runs have cost today and this week, in the assistant's head, and
 * the week's budget behind it (v0.107).
 *
 * The price is the CLI's own, failed runs included (`spend.rs`). The budget
 * is soft: crossing it turns the week red here, writes one line into the
 * history and says so once - nothing is refused. Setting it is a click on
 * the figures; the settings screen of v0.108 will hold the same control.
 */
export function SpendMeter() {
  const { t } = useTranslation()
  const client = useQueryClient()
  const spend = useQuery(queries.spend()).data
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState<number | null>(null)

  const save = useMutation({
    mutationFn: setWeeklyBudget,
    onSuccess: (budget) => {
      void client.invalidateQueries({ queryKey: keys.spend })
      say.ok(
        budget === null
          ? t('assistant.budgetCleared')
          : t('assistant.budgetSaved', { amount: formatCost(budget) }),
      )
      setOpen(false)
    },
    onError: (cause) => say.failedTo(t('assistant.budgetFailed'), cause),
  })

  if (spend === undefined) return null

  const over = spend.budget !== null && spend.week > spend.budget
  const week =
    spend.budget === null
      ? t('assistant.spendWeek', { amount: formatCost(spend.week) })
      : t('assistant.spendWeekOf', {
          amount: formatCost(spend.week),
          budget: formatCost(spend.budget),
        })

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) setDraft(spend.budget)
      }}
    >
      <PopoverTrigger
        render={
          <Button
            size="sm"
            variant="ghost"
            className={cn('text-xs font-normal', over ? 'text-bad' : 'text-faint')}
            title={t('assistant.spendTitle')}
          />
        }
      >
        {t('assistant.spendToday', { amount: formatCost(spend.today) })} · {week}
      </PopoverTrigger>

      <PopoverPopup size="md" align="end" arrow={false} className="flex flex-col gap-2 p-3">
        <PopoverTitle>
          <span className="caption">{t('assistant.spendTitle')}</span>
        </PopoverTitle>
        <p className="text-xs text-dim">{t('assistant.spendHint')}</p>
        {/* `noValidate`: the field keeps a hidden number input for forms, and
            with a minimum of 0.01 and a step of 1 the browser holds only
            0.01, 1.01, 2.01... valid - a budget of 0.50 or of 15 was refused
            in silence (v0.107 rehearsal). The amount is checked here. */}
        <form
          noValidate
          className="flex flex-col gap-2 border-t border-line pt-2"
          onSubmit={(event) => {
            event.preventDefault()
            if (draft !== null && draft > 0) save.mutate(draft)
          }}
        >
          <span className="caption">{t('assistant.budget')}</span>
          <NumberField
            value={draft}
            onValueChange={setDraft}
            min={0.01}
            step={1}
            format={{ style: 'currency', currency: 'USD' }}
            aria-label={t('assistant.budget')}
            className="w-40"
          />
          <p className="text-xs text-dim">{t('assistant.budgetHint')}</p>
          <div className="flex justify-end gap-1.5">
            <Button
              size="sm"
              variant="ghost"
              disabled={spend.budget === null || save.isPending}
              onClick={() => save.mutate(null)}
            >
              {t('assistant.budgetClear')}
            </Button>
            <Button
              type="submit"
              size="sm"
              variant="primary"
              disabled={draft === null || draft <= 0 || save.isPending}
            >
              {t('assistant.budgetSave')}
            </Button>
          </div>
        </form>
      </PopoverPopup>
    </Popover>
  )
}
