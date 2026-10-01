import { useTranslation } from 'react-i18next'
import type { CoverDetail } from '@/lib/api/types'
import { Switch } from '@/components/ui/switch'

interface Props {
  details: CoverDetail[]
  disabled?: boolean
  /** Switch one on or off - for this picture; the channel keeps its default. */
  onToggle: (id: string, on: boolean) => void
}

/**
 * The channel's signature details as switches (v0.88): one per detail of the
 * card «Channel» that belongs in this picture, in the card's order.
 *
 * A switch starts where the channel puts it and the picture remembers its
 * own; a detail added to the card appears here on every picture, and one
 * taken off leaves the pictures that used it with the prompt they were
 * copied with. What each one puts into the prompt is its title - hover it.
 */
export function DetailSwitches({ details, disabled, onToggle }: Props) {
  const { t } = useTranslation()
  if (details.length === 0) {
    return <p className="text-xs text-faint">{t('cover.details.none')}</p>
  }
  return (
    <ul className="flex flex-col gap-1.5">
      {details.map((detail) => (
        <li key={detail.id} title={detail.template} className="flex items-center gap-2">
          <Switch
            checked={detail.on}
            disabled={disabled}
            onCheckedChange={(on) => onToggle(detail.id, on)}
          >
            {detail.name}
          </Switch>
          {detail.on !== detail.default_on && (
            <span className="text-2xs text-faint">{t('cover.details.ownChoice')}</span>
          )}
        </li>
      ))}
    </ul>
  )
}
