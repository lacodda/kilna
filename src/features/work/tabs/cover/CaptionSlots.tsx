import { useTranslation } from 'react-i18next'
import type { CoverSlot } from '@/lib/api/types'
import { DraftText } from '@/features/cover/Section'

interface Props {
  slots: CoverSlot[]
  disabled?: boolean
  /** The cover's own lines for a slot; none hands it back to the channel. */
  onChange: (slot: string, lines: string[]) => void
}

/**
 * The captions the chosen lettering and dressing ask for, one box per slot.
 *
 * A slot is filled from the cover first and the channel's card second, and a
 * slot with nothing in either drops out of the prompt with the phrase around
 * it - so the box shows the channel's lines as its placeholder, and says so
 * when there are none. One line per caption: `{micro}` reads them all,
 * `{micro.0}` the first.
 */
export function CaptionSlots({ slots, disabled, onChange }: Props) {
  const { t } = useTranslation()
  if (slots.length === 0) return null
  return (
    <ul aria-label={t('cover.dressing.captions')} className="flex flex-col gap-2">
      {slots.map((slot) => (
        <li key={slot.name} className="flex flex-col gap-1">
          <span className="flex items-baseline gap-2">
            <span className="rounded-sm bg-info-soft px-1.5 font-mono text-xs text-info">
              {`{${slot.name}}`}
            </span>
            <span className="text-2xs text-faint">
              {slot.own.length > 0
                ? t('cover.dressing.own')
                : slot.channel.length > 0
                  ? t('cover.dressing.fromChannel')
                  : t('cover.dressing.drops')}
            </span>
          </span>
          <DraftText
            label={t('cover.dressing.slot', { slot: slot.name })}
            value={slot.own.join('\n')}
            rows={1}
            mono
            disabled={disabled}
            placeholder={slot.channel.join('\n')}
            onCommit={(text) =>
              onChange(
                slot.name,
                text
                  .split('\n')
                  .map((line) => line.trim())
                  .filter((line) => line !== ''),
              )
            }
          />
        </li>
      ))}
    </ul>
  )
}
