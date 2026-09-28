import { useTranslation } from 'react-i18next'
import { CalendarRange } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api/types'
import { countByKind, type KindFilter } from '@/lib/calendarFilter'
import { KindGlyph } from '@/lib/releaseIcon'
import { allOf, say, useProfile } from '@/lib/useProfile'
import { Chip, ChipGroup } from '@/components/ui/chip'

interface Props {
  /** Everything with a date, unfiltered: the counts are of the whole calendar. */
  slots: readonly ScheduledRelease[]
  value: KindFilter
  onChange: (kind: KindFilter) => void
}

/**
 * Which kinds of release the month is showing.
 *
 * Chips carry their words, not just a glyph. The catalogue learned this the
 * hard way at v0.19: with icons alone nobody could tell which filter was on.
 * The glyph is here because it is the same one the chips carry, so the row
 * doubles as the legend the calendar otherwise lacks.
 */
export function KindFilterBar({ slots, value, onChange }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const counts = countByKind(slots)

  // Only kinds the calendar actually holds. A profile states every kind the
  // craft can ship; a filter for a kind with nothing behind it is a button
  // that can only ever empty the screen.
  const kinds = allOf(profile.config, 'release_kinds').filter((kind) => counts.has(kind.key))

  // One kind is not a choice. The row would say "all" beside the only thing
  // there is, which reads as a broken filter rather than a simple calendar.
  if (kinds.length < 2) return null

  return (
    <ChipGroup
      aria-label={t('releases.kind')}
      value={[value ?? ALL]}
      // One kind at a time. Pressing the chip that is already on lets go of
      // it, which is back to all of them, the way the catalogue's gap chips
      // do: no separate control for going back.
      onValueChange={([next]) => onChange(next === undefined || next === ALL ? null : next)}
    >
      <Chip value={ALL} count={slots.length}>
        <CalendarRange aria-hidden className="size-3.5 shrink-0" />
        {t('calendar.allKinds')}
      </Chip>
      {kinds.map((kind) => (
        <Chip key={kind.key} value={kind.key} count={counts.get(kind.key) ?? 0}>
          <KindGlyph icon={kind.icon} className="size-3.5 shrink-0" />
          {say(kind.label)}
        </Chip>
      ))}
    </ChipGroup>
  )
}

/** The value of the chip that stands for every kind at once. A symbol rather
 * than a word, so it is not a kind a profile would name; and not empty,
 * because a Toggle reads an empty value as none and makes up an id instead. */
const ALL = '*'
