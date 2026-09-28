import type { Slot } from '@/lib/dashboard'
import { coverImageFor } from '@/lib/cover'
import { formatDay } from '@/lib/format'
import { useCovers } from '@/lib/useCovers'
import { allOf, labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'

/**
 * One of the nearest slots: the cover, the work, what goes out and when.
 *
 * Opens on the work's releases, where the slot it stands for is kept - the
 * card is about a date, not about the work in general.
 */
export function SlotCard({
  slot,
  onSelect,
}: {
  slot: Slot
  onSelect: (workId: string, tab?: string) => void
}) {
  const covers = useCovers()
  const profile = useProfile()
  const { release } = slot

  return (
    // A card, not a row: a cover over two lines, which no row's slots hold.
    // So the quiet button with no size of its own, laid out as a column; its
    // border and hover are the card's.
    <Button
      variant="ghost"
      size={null}
      onClick={() => onSelect(release.work_id, 'releases')}
      className="flex-col items-stretch gap-0 overflow-hidden rounded-lg bg-raise text-left"
    >
      <span
        aria-hidden
        className="block h-16.5"
        style={{ background: coverImageFor(release.work_id, covers.get(release.work_id)) }}
      />
      <span className="block px-3 pt-2 pb-2.5">
        <span className="block truncate text-sm font-semibold text-text">{release.work_title}</span>
        <span className="block truncate font-mono text-xs text-faint">
          {labelOf(allOf(profile.config, 'release_kinds'), release.kind)} ·{' '}
          {formatDay(release.scheduled_at as string)}
        </span>
      </span>
    </Button>
  )
}
