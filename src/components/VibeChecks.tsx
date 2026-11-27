import { useTranslation } from 'react-i18next'
import type { VibeAxes } from '@/lib/api/types'
import { vibeTick } from '@/lib/catalogue'
import { say } from '@/lib/useProfile'
import { VIBE_LINES } from '@/lib/vibe'
import { Chip, ChipGroup } from '@/components/ui/chip'

const WORDS = { mood: 'moods', character: 'characters', complexity: 'complexities' } as const

/**
 * The three lines of a vibe as words to tick (v0.169): what a list is
 * narrowed to by what its works feel like. Ticks on one line are any of
 * them, lines are all of them - the catalogue's vibe funnel reads the same
 * ticks by the same rule (`feels`).
 */
export function VibeChecks({
  axes,
  chosen,
  onChange,
}: {
  axes: VibeAxes
  /** The ticks, as `vibeTick` writes them. */
  chosen: readonly string[]
  onChange: (next: string[]) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex w-64 flex-col gap-2 px-1 py-0.5">
      {VIBE_LINES.map((line) => {
        const ticks = axes[WORDS[line]].map((word) => ({
          tick: vibeTick(line, word.key),
          label: say(word.label),
        }))
        const mine = new Set(ticks.map((one) => one.tick))
        return (
          <div key={line} className="flex flex-col gap-1">
            <span className="text-2xs text-faint">{t(`vibe.line.${line}`)}</span>
            <ChipGroup
              multiple
              aria-label={t(`vibe.line.${line}`)}
              value={chosen.filter((tick) => mine.has(tick))}
              onValueChange={(next) =>
                onChange([...chosen.filter((tick) => !mine.has(tick)), ...next])
              }
            >
              {ticks.map((one) => (
                <Chip key={one.tick} value={one.tick}>
                  {one.label}
                </Chip>
              ))}
            </ChipGroup>
          </div>
        )
      })}
    </div>
  )
}
