import { useTranslation } from 'react-i18next'
import type { BankDraft } from '@/lib/bank'
import { DatePicker } from '@/components/DatePicker'
import { Input } from '@/components/ui/input'
import { NumberField } from '@/components/ui/number-field'
import { Switch } from '@/components/ui/switch'

interface Props {
  draft: BankDraft
  onChange: (next: BankDraft) => void
}

/**
 * The boxes of an item of a bank (v0.151, FR-20 20.2): its group, what it is
 * made of, how long it rests after a use, when it was last used, and whether
 * it is locked. The works that used it are marked from a work or by a
 * release going out, not typed here; the day of the last use may be, for a
 * use from before the bank was kept.
 */
export function BankFields({ draft, onChange }: Props) {
  const { t } = useTranslation()
  const set = (patch: Partial<BankDraft>) => onChange({ ...draft, ...patch })
  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)] gap-2">
        <Input
          value={draft.group}
          onChange={(event) => set({ group: event.target.value })}
          placeholder={t('canon.bank.groupPlaceholder')}
          aria-label={t('canon.bank.group')}
        />
        <Input
          value={draft.attributes}
          onChange={(event) => set({ attributes: event.target.value })}
          placeholder={t('canon.bank.attributesPlaceholder')}
          aria-label={t('canon.bank.attributes')}
        />
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <NumberField
          className="w-44"
          min={0}
          max={120}
          step={1}
          value={draft.cooldown}
          unit={t('canon.bank.months')}
          aria-label={t('canon.bank.cooldown')}
          onValueChange={(cooldown) => set({ cooldown })}
        />
        <DatePicker
          value={draft.lastUsed}
          onChange={(lastUsed) => set({ lastUsed })}
          placeholder={t('canon.bank.lastUsed')}
          aria-label={t('canon.bank.lastUsed')}
        />
        <Switch checked={draft.locked} onCheckedChange={(locked) => set({ locked })}>
          {t('canon.bank.locked')}
        </Switch>
      </div>
      <Input
        value={draft.note}
        onChange={(event) => set({ note: event.target.value })}
        placeholder={t('canon.bank.notePlaceholder')}
        aria-label={t('canon.bank.note')}
      />
    </div>
  )
}
