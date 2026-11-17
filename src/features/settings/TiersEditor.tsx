import { useTranslation } from 'react-i18next'
import type { Tier } from '@/lib/api/types'
import { NumberField } from '@/components/ui/number-field'
import { KeyedList } from '@/features/settings/KeyedList'
import { LabelInput } from '@/features/settings/LabelInput'

/**
 * A kind's tiers: what each is called and the total it takes to reach it.
 * Added, taken out and put in order since v0.112; a tier's key stands, as a
 * score already given names its tier by it.
 */
export function TiersEditor({
  tiers,
  onChange,
  problems,
}: {
  tiers: Tier[]
  onChange: (tiers: Tier[]) => void
  problems?: (key: string) => readonly string[]
}) {
  const { t } = useTranslation()

  return (
    <KeyedList
      label={t('editor.tiers')}
      help={t('editor.tiersHint')}
      entries={tiers}
      onChange={onChange}
      make={(key) => ({ key, label: key, min: 0 })}
      addLabel={t('editor.addTier')}
      problems={problems}
    >
      {(tier, set) => (
        <>
          <LabelInput
            className="flex-1"
            label={tier.label}
            onChange={(label) => set({ label: label ?? '' })}
            aria-label={`${tier.key} label`}
          />
          <NumberField
            className="w-20"
            hideStepper
            min={0}
            max={100}
            step={1}
            value={tier.min}
            onValueChange={(min) => set({ min: min ?? 0 })}
            aria-label={`${tier.key} threshold`}
          />
        </>
      )}
    </KeyedList>
  )
}
