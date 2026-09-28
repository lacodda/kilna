import { useTranslation } from 'react-i18next'
import { Star } from 'lucide-react'
import type { ScoredWork } from '@/lib/api/types'
import { useStar } from '@/lib/useStar'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

/**
 * The star on a row: on when the work is marked to come back to, and a click
 * either way. It stops the row's own click, which would open the card.
 */
export function RowStar({ row }: { row: ScoredWork }) {
  const { t } = useTranslation()
  const star = useStar(row.work_id)
  const on = row.bookmarked_at !== null
  return (
    <Button
      variant="icon"
      size="icon-xs"
      aria-pressed={on}
      title={t(on ? 'work.unstar' : 'work.star')}
      aria-label={t(on ? 'work.unstar' : 'work.star')}
      disabled={star.isPending}
      onClick={(event) => {
        event.stopPropagation()
        star.mutate(!on)
      }}
    >
      {/* The warn colour is the glyph's rather than the button's: a lit star
          is what says the work is starred, here as on the card, and it stays
          lit whatever the button's hover does. */}
      <Star aria-hidden className={cn('size-3.5', on && 'fill-current text-warn')} />
    </Button>
  )
}
