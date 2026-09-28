import { useTranslation } from 'react-i18next'
import type { Tier } from '@/lib/api/types'
import { say as sayLabel } from '@/lib/useProfile'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'

/** A kind's tiers: what each is called and the total it takes to reach it. */
export function TiersEditor({
  tiers,
  onChange,
}: {
  tiers: Tier[]
  onChange: (tiers: Tier[]) => void
}) {
  const { t } = useTranslation()

  const setTier = (index: number, changes: Partial<Tier>) => {
    onChange(tiers.map((tier, i) => (i === index ? { ...tier, ...changes } : tier)))
  }

  return (
    <FieldGroup label={t('editor.tiers')} help={t('editor.tiersHint')}>
      <ul className="flex flex-col gap-1.5">
        {tiers.map((tier, index) => (
          <li key={tier.key} className="flex items-center gap-2">
            <code className="w-28 shrink-0 font-mono text-xs text-dim">{tier.key}</code>
            <Input
              className="flex-1"
              value={sayLabel(tier.label)}
              onChange={(event) => setTier(index, { label: event.target.value })}
              aria-label={`${tier.key} label`}
            />
            <NumberField
              className="w-20"
              hideStepper
              min={0}
              max={100}
              step={1}
              value={tier.min}
              onValueChange={(min) => setTier(index, { min: min ?? 0 })}
              aria-label={`${tier.key} threshold`}
            />
          </li>
        ))}
      </ul>
    </FieldGroup>
  )
}
