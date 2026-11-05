import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { formatCost } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Popover, PopoverPopup, PopoverTitle, PopoverTrigger } from '@/components/ui/popover'
import { BudgetForm } from '@/features/assistant/BudgetForm'

/**
 * What the runs have cost today and this week, in the assistant's head, and
 * the week's budget behind it (v0.107).
 *
 * The price is the CLI's own, failed runs included (`spend.rs`). The budget
 * is soft: crossing it turns the week red here, writes one line into the
 * history and says so once - nothing is refused. Setting it is a click on
 * the figures, or in Settings, which holds the same form.
 */
export function SpendMeter() {
  const { t } = useTranslation()
  const spend = useQuery(queries.spend()).data
  const [open, setOpen] = useState(false)

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
    <Popover open={open} onOpenChange={setOpen}>
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
        <div className="flex flex-col gap-2 border-t border-line pt-2">
          <span className="caption">{t('assistant.budget')}</span>
          <BudgetForm budget={spend.budget} onDone={() => setOpen(false)} />
        </div>
      </PopoverPopup>
    </Popover>
  )
}
