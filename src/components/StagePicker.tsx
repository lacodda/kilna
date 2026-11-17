import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { StageDial, stageWords } from '@/components/StageDial'
import { StageStops } from '@/components/StageStops'
import { stageAt } from '@/lib/stages'
import { useProfile } from '@/lib/useProfile'
import { useStage } from '@/lib/useStage'
import { cn } from '@/lib/utils'

/**
 * The dial, and the five stops that set it.
 *
 * A row of stops rather than a free slider: the stops are what an author
 * thinks in, and a continuous slider would invite them to agonise over 63
 * versus 67, which is a question about the control rather than about the
 * work. Five dials in a row and nothing else (FR-03): no word, no number and
 * no "Clear" - a work always stands somewhere, and the way back to the start
 * is the first stop.
 */
export function StagePicker({
  workId,
  percent,
  /** The button's size; the catalogue row draws it tighter than the card. */
  compact = false,
  className,
}: {
  workId: string
  percent: number
  compact?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [open, setOpen] = useState(false)
  const stage = useStage(workId)
  const current = stageAt(profile.config, percent)
  const words = stageWords(t, current)

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={words}
        title={words}
        disabled={stage.isPending}
        className={cn(
          'cursor-pointer rounded-md transition-colors hover:bg-soft',
          compact ? 'p-0.5' : 'flex size-7 items-center justify-center',
          className,
        )}
      >
        <StageDial stage={current} size={compact ? 14 : 16} />
      </PopoverTrigger>

      <PopoverPopup arrow={false} className="w-auto p-1.5">
        <StageStops
          label={t('stage.label')}
          pick="one"
          on={[current.percent]}
          onPick={(next) => {
            stage.mutate(next)
            // Choosing is the whole errand; the panel closes rather than
            // waiting for a second, dismissing click.
            setOpen(false)
          }}
        />
      </PopoverPopup>
    </Popover>
  )
}
