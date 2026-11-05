import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { setWeeklyBudget } from '@/lib/api/assistant'
import { formatCost } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { NumberField } from '@/components/ui/number-field'

interface Props {
  /** What the week may cost now; none when unset. */
  budget: number | null
  /** Told once the budget is saved or cleared. */
  onDone?: () => void
}

/**
 * Setting what a week of runs may cost: one form, for the assistant's head and
 * for Settings. The budget is soft (`spend.rs`): crossing it is said, and
 * nothing is refused.
 */
export function BudgetForm({ budget, onDone }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const [draft, setDraft] = useState<number | null>(budget)

  const save = useMutation({
    mutationFn: setWeeklyBudget,
    onSuccess: (saved) => {
      void client.invalidateQueries({ queryKey: keys.spend })
      say.ok(
        saved === null
          ? t('assistant.budgetCleared')
          : t('assistant.budgetSaved', { amount: formatCost(saved) }),
      )
      onDone?.()
    },
    onError: (cause) => say.failedTo(t('assistant.budgetFailed'), cause),
  })

  return (
    // `noValidate`: the field keeps a hidden number input for forms, and with a
    // minimum of 0.01 and a step of 1 the browser holds only 0.01, 1.01, 2.01...
    // valid - a budget of 0.50 or of 15 was refused in silence (v0.107
    // rehearsal). The amount is checked here.
    <form
      noValidate
      className="flex flex-col gap-2"
      onSubmit={(event) => {
        event.preventDefault()
        if (draft !== null && draft > 0) save.mutate(draft)
      }}
    >
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
          disabled={budget === null || save.isPending}
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
  )
}
