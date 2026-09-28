import { useTranslation } from 'react-i18next'
import { Check, ChevronDown } from 'lucide-react'
import type { GroupBy } from '@/lib/catalogue'
import { Button } from '@/components/ui/button'
import {
  Menu,
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuPopup,
  MenuTrigger,
} from '@/components/ui/menu'

interface Props {
  value: GroupBy
  onChange: (next: GroupBy) => void
}

const CHOICES: { value: GroupBy; label: string }[] = [
  { value: 'none', label: 'catalogue.groupNone' },
  { value: 'status', label: 'catalogue.groupStatus' },
  { value: 'tier', label: 'catalogue.groupTier' },
]

/**
 * How the rows are gathered into blocks: a button in the toolbar's row of
 * buttons, where it was a dropdown field among them until v0.79. The button
 * wears the accent while the table is grouped, so a folded catalogue says why
 * it looks the way it does.
 */
export function GroupMenu({ value, onChange }: Props) {
  const { t } = useTranslation()
  const chosen = CHOICES.find((choice) => choice.value === value)

  return (
    <Menu>
      <MenuTrigger render={<Button size="sm" variant={value === 'none' ? 'ghost' : 'soft'} />}>
        {value === 'none' || chosen === undefined ? t('catalogue.groupBy') : t(chosen.label)}
        <ChevronDown aria-hidden />
      </MenuTrigger>

      <MenuPopup align="end">
        {/* One of three: the ticked one is what the table is doing now, and
            ticking another replaces it. */}
        {CHOICES.map((choice) => (
          <MenuCheckboxItem
            key={choice.value}
            checked={choice.value === value}
            // One choice and done, unlike the columns, which are compared.
            closeOnClick
            onCheckedChange={() => onChange(choice.value)}
          >
            <MenuCheckboxIndicator>
              <Check className="size-3.5" aria-hidden />
            </MenuCheckboxIndicator>
            {t(choice.label)}
          </MenuCheckboxItem>
        ))}
      </MenuPopup>
    </Menu>
  )
}
