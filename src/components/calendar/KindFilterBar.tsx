import { useTranslation } from 'react-i18next'
import { CalendarRange } from 'lucide-react'
import type { ScheduledRelease } from '@/lib/api'
import { countByKind, type KindFilter } from '@/lib/calendarFilter'
import { KindGlyph } from '@/lib/releaseIcon'
import { useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

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
  const kinds = profile.config.release_kinds.filter((kind) => counts.has(kind.key))

  // One kind is not a choice. The row would say "all" beside the only thing
  // there is, which reads as a broken filter rather than a simple calendar.
  if (kinds.length < 2) return null

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Chip
        active={value === null}
        onClick={() => onChange(null)}
        icon={null}
        label={t('calendar.allKinds')}
        count={slots.length}
      />
      {kinds.map((kind) => (
        <Chip
          key={kind.key}
          active={value === kind.key}
          // Clicking the chip that is already on turns it off, the way the
          // catalogue's gap chips do: one kind at a time, and no separate
          // control for going back to all of them.
          onClick={() => onChange(value === kind.key ? null : kind.key)}
          icon={kind.icon}
          label={kind.label}
          count={counts.get(kind.key) ?? 0}
        />
      ))}
    </div>
  )
}

interface ChipProps {
  active: boolean
  onClick: () => void
  /** Glyph name, or null for the chip that stands for every kind at once. */
  icon: string | null | undefined
  label: string
  count: number
}

function Chip({ active, onClick, icon, label, count }: ChipProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      // `aria-pressed` rather than a bare button: a toggle that only looks
      // pressed tells a screen reader nothing about what the grid is showing.
      aria-pressed={active}
      className={cn(
        'flex cursor-pointer items-center gap-1.5 rounded-[9px] px-2.5 py-1 text-xs transition-colors',
        active ? 'bg-accent-soft text-accent-2' : 'text-dim hover:bg-soft',
      )}
    >
      {icon === null ? (
        <CalendarRange aria-hidden className="size-3.5 shrink-0" />
      ) : (
        <KindGlyph icon={icon} className="size-3.5 shrink-0" />
      )}
      {label}
      <span className="font-mono tabular-nums text-faint">{count}</span>
    </button>
  )
}
