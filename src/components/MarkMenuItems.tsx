import { useTranslation } from 'react-i18next'
import { Check, Minus } from 'lucide-react'
import type { Mark } from '@/lib/api/types'
import { markIconOf } from '@/lib/markIcon'
import { say } from '@/lib/useProfile'
import { MenuCheckboxIndicator, MenuCheckboxItem } from '@/components/ui/menu'

/** Whether a mark is raised: on, off, or on some of a batch. */
export type MarkState = boolean | 'some'

/**
 * The profile's marks as a menu's ticks (v0.139, FR-12 12.1): the card's
 * menu, the catalogue cell's and the bulk bar's are one list, so a mark reads
 * the same wherever it is raised. A mark on some of a batch shows a dash, and
 * pressing it raises it on all of them - the next press takes it off.
 */
export function MarkMenuItems({
  marks,
  stateOf,
  onToggle,
}: {
  marks: readonly Mark[]
  stateOf: (key: string) => MarkState
  onToggle: (key: string, raised: boolean) => void
}) {
  const { t } = useTranslation()
  return marks.map((mark) => {
    const Icon = markIconOf(mark)
    const state = stateOf(mark.key)
    return (
      <MenuCheckboxItem
        key={mark.key}
        checked={state === true}
        onCheckedChange={(next) => onToggle(mark.key, state === 'some' ? true : next)}
      >
        <Icon aria-hidden className={MARK_INK[mark.colour ?? 'plain']} />
        <span className="flex-1">{say(mark.label)}</span>
        {state === 'some' ? (
          <span title={t('catalogue.bulk.markSome')} aria-label={t('catalogue.bulk.markSome')}>
            <Minus aria-hidden className="size-3.5 text-faint" />
          </span>
        ) : (
          <MenuCheckboxIndicator>
            <Check aria-hidden className="size-3.5" />
          </MenuCheckboxIndicator>
        )}
      </MenuCheckboxItem>
    )
  })
}

/** A mark's icon colour in the menu that raises and lowers it, by the palette
    role its profile named - the colour its tile wears once raised. Plain and
    accent take the item's own ink. */
const MARK_INK: Record<string, string> = {
  plain: '',
  accent: '',
  good: 'text-good',
  warn: 'text-warn',
  bad: 'text-bad',
  info: 'text-info',
}
