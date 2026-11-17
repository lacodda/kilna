import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import type { ShownVibe, Vibe } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { axesOf, resolveVibe, VIBE_LINES, vibeWords, withLine, type VibeLine } from '@/lib/vibe'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Popover, PopoverPopup, PopoverTrigger } from '@/components/ui/popover'
import { VibeIcon } from '@/components/VibeIcon'

/**
 * The mark of what a work feels like, and the three lines that set it
 * (FR-02): its mood, its character, how complex it is.
 *
 * One, two or all three may be said; a line left unsaid stands at the
 * profile's neutral word while the mark is drawn, and the last word taken
 * away takes the mark with it. Pressing a word that is on takes it off -
 * there is no separate "none" on a line.
 *
 * A work with no vibe of its own shows the one of what it was made from - a
 * clip is its song on another surface - and says whose it is. Saying one
 * here makes it the work's own; clearing it gives the source's back.
 */
export function VibePicker({
  workId,
  own,
  shown,
  compact = false,
  className,
}: {
  workId: string
  /** What was said for this work itself. */
  own: Vibe | null | undefined
  /** What it shows: its own, or the one it takes from what it was made from. */
  shown: ShownVibe | null | undefined
  /** The trigger's size; a catalogue's row draws it tighter than a card. */
  compact?: boolean
  className?: string
}) {
  const { t, i18n } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [open, setOpen] = useState(false)
  const axes = axesOf(profile.config)

  const save = useAppMutation({
    mutationFn: (vibe: Vibe | null) => updateWork(workId, { vibe }),
    failure: 'toast.workSaveFailed',
    onSuccess: (updated) => {
      client.setQueryData(keys.work(workId), updated)
      announceEdited({
        client,
        message: t('vibe.saved'),
        refresh: [...refresh.work, keys.calendar, keys.releases],
      })
    },
  })

  // A profile with no lines has nothing to pick from and nothing to draw.
  if (axes === undefined) return null

  const language = i18n.resolvedLanguage ?? 'en'
  const taken = own === undefined || own === null ? (shown?.from ?? undefined) : undefined
  const resolved = resolveVibe(axes, own ?? shown?.vibe)
  const words =
    resolved === null
      ? t('vibe.set')
      : taken === undefined
        ? vibeWords(resolved, language)
        : t('vibe.takenFrom', { vibe: vibeWords(resolved, language), title: taken })

  const lineName = (line: VibeLine) => {
    const named = axes.labels?.[line]
    return named === undefined || named === null ? t(`vibe.line.${line}`) : sayLabel(named)
  }
  const options = {
    mood: axes.moods,
    character: axes.characters,
    complexity: axes.complexities,
  } as const

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={words}
        title={words}
        disabled={save.isPending}
        className={cn(
          'cursor-pointer rounded-md transition-colors hover:bg-soft',
          compact ? 'p-0.5' : 'flex size-7 items-center justify-center',
          // Nothing said yet: a quiet way in, not a mark.
          resolved === null && 'text-faint opacity-50 hover:opacity-100 focus-visible:opacity-100',
          className,
        )}
      >
        {resolved === null ? (
          <Plus aria-hidden className={compact ? 'size-3.5' : 'size-4'} />
        ) : (
          <VibeIcon
            vibe={resolved}
            words={words}
            size={compact ? 16 : 20}
            // Taken from elsewhere, it is drawn quieter than a work's own.
            className={cn(taken !== undefined && 'opacity-60')}
          />
        )}
      </PopoverTrigger>

      <PopoverPopup arrow={false} className="flex w-80 flex-col gap-3 p-3">
        <div className="flex min-w-0 items-center gap-3">
          <span className="flex size-12 shrink-0 items-center justify-center rounded-md bg-bg">
            {resolved === null ? (
              <Plus aria-hidden className="size-5 text-faint" />
            ) : (
              <VibeIcon vibe={resolved} words={words} size={32} />
            )}
          </span>
          <p className="min-w-0 text-sm">
            {resolved === null ? <span className="text-dim">{t('vibe.none')}</span> : words}
          </p>
        </div>

        {VIBE_LINES.map((line) => (
          <div key={line} className="flex flex-col gap-1">
            <span className="text-xs text-dim">{lineName(line)}</span>
            <ChipGroup
              aria-label={lineName(line)}
              value={own?.[line] === undefined || own[line] === null ? [] : [own[line]]}
              // Pressing the word that is on takes it off.
              onValueChange={([next]) => save.mutate(withLine(own, line, next))}
              className="flex-wrap"
            >
              {options[line].map((option) => (
                <Chip
                  key={option.key}
                  value={option.key}
                  title={
                    option.hint === undefined || option.hint === null
                      ? undefined
                      : sayLabel(option.hint)
                  }
                >
                  {sayLabel(option.label)}
                </Chip>
              ))}
            </ChipGroup>
          </div>
        ))}

        <Button
          size="sm"
          className="self-start"
          disabled={own === undefined || own === null || save.isPending}
          onClick={() => save.mutate(null)}
        >
          {shown?.from === undefined || shown.from === null || own === undefined || own === null
            ? t('vibe.clear')
            : t('vibe.takeFromSource')}
        </Button>
      </PopoverPopup>
    </Popover>
  )
}
