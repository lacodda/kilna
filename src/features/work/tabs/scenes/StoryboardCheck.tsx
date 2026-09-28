import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CircleCheck, TriangleAlert } from 'lucide-react'
import type { Complaint, Storyboard } from '@/lib/storyboard'
import { formatSeconds } from '@/lib/timecode'
import { cn } from '@/lib/utils'
import { chipVariants } from '@/components/ui/chip'
import { RowButton } from '@/components/ui/list-row'
import { Popover, PopoverPopup, PopoverTitle, PopoverTrigger } from '@/components/ui/popover'
import { Scroll } from '@/components/frame'

/*
 * What the board still owes, as a chip at the foot of the board.
 *
 * The harvest of a storyboard: the counts a person would otherwise make by
 * scrolling, and the list of what is missing. It stood above the board as a
 * panel until v0.81 and took the height of its list away from the board even
 * when the list said nothing was missing (wish 1944); now it is the mockup's
 * chip - how many things are missing, in the colour of a warning - and the
 * list opens from it. The number stays in sight, so the queue is not
 * forgotten; the queue itself is one click away rather than always open.
 *
 * Every line is a way back into the board: clicking one opens the scene it is
 * about. A report that names scene 34 and leaves you to find scene 34 is a
 * report that costs more than it saves on a board of fifty.
 */

export interface StoryboardCheckProps {
  board: Storyboard
  /** Take me to this scene: the row opens and the board scrolls to it. */
  onGo: (sceneId: string) => void
}

export function StoryboardCheck({ board, onGo }: StoryboardCheckProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const { tally, complaints } = board

  // An empty board has nothing to harvest. It is not "done" and it is not
  // "wrong" — it is a board before the work, and the tab already says so.
  if (tally.scenes === 0) return null

  const missing = complaints.length

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        className={cn(
          chipVariants({ variant: missing === 0 ? 'good' : 'warn' }),
          'cursor-pointer target-min',
          'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent',
        )}
      >
        {missing === 0 ? t('scenes.check.chipDone') : t('scenes.check.chip', { number: missing })}
      </PopoverTrigger>

      <PopoverPopup
        size="lg"
        side="top"
        align="end"
        arrow={false}
        className="flex flex-col gap-2 p-2"
      >
        <PopoverTitle className="px-1.5 pt-1">
          <span className="caption">{t('scenes.check.title')}</span>
        </PopoverTitle>
        {/* The counts, as a sentence rather than a row of tiles: five numbers
            about one board read as a sentence and tile up into a dashboard
            that says less. */}
        <p className="px-1.5 text-sm text-dim">
          {t('scenes.check.tally', {
            scenes: tally.scenes,
            written: tally.written,
            framed: tally.framed,
            filmed: tally.filmed,
          })}
        </p>

        {missing === 0 ? (
          <p className="flex items-center gap-2 px-1.5 pb-1 text-sm text-good">
            <CircleCheck aria-hidden className="size-4 shrink-0" />
            {t('scenes.check.nothingMissing')}
          </p>
        ) : (
          // Held to a height of its own: a board of fifty half-drawn scenes
          // can say a lot, and the popup must not run off the window.
          <div className="flex max-h-80 flex-col">
            <Scroll label={t('scenes.check.title')}>
              <ul className="flex flex-col gap-0.5">
                {complaints.map((complaint, index) => (
                  <Line
                    // A complaint has no id of its own — it is a fact about a
                    // board, computed fresh on every render. Its place in the
                    // list is what identifies it, and the list is rebuilt
                    // whole each time.
                    key={`${complaint.kind}:${complaint.run?.sceneId ?? complaint.scene?.id ?? index}`}
                    complaint={complaint}
                    onGo={(sceneId) => {
                      setOpen(false)
                      onGo(sceneId)
                    }}
                  />
                ))}
              </ul>
            </Scroll>
          </div>
        )}
      </PopoverPopup>
    </Popover>
  )
}

/** One thing missing, as a line you can click to go and fix it. */
function Line({ complaint, onGo }: { complaint: Complaint; onGo: (sceneId: string) => void }) {
  const { t } = useTranslation()
  const { kind, scene, seconds, count, run } = complaint

  // A run of neighbours saying the same thing reads as one sentence about
  // the stretch — "scenes 3 to 42 are waiting for a picture" — rather than
  // forty copies of one sentence about a scene.
  const text = run
    ? t(`scenes.check.run.${kind}`, { from: run.from, to: run.to, count: count ?? 0 })
    : t(`scenes.check.${kind}`, {
        number: scene?.position ?? 0,
        count: count ?? 0,
        // Seconds read as a timecode everywhere else on this board, so a gap
        // of ninety seconds says 1:30 rather than 90 — one board, one way of
        // writing time.
        seconds: formatSeconds(seconds ?? null),
      })

  // Where the line sends you: the scene, or the first of a run — which is
  // where the work resumes.
  const target = run?.sceneId ?? scene?.id

  // A complaint about the whole timing has nowhere to go; it is a line, not
  // a button, because a button that does nothing when pressed is worse than
  // plain text.
  if (target === undefined) {
    return (
      // The padding and gap a RowButton carries, so a line about the whole
      // board stands in the same column as the lines about scenes rather
      // than to their left — the drift was measured on a real board once.
      <li className="flex items-center gap-2.5 px-2.5 py-1.5 text-sm text-dim">
        <TriangleAlert aria-hidden className="size-3.5 shrink-0 text-warn" />
        <span>{text}</span>
      </li>
    )
  }

  return (
    <li>
      {/* The row truncates its words to one line; the title keeps a long
          sentence readable in a narrow window. */}
      <RowButton
        start={<TriangleAlert aria-hidden className="size-3.5 text-warn" />}
        title={text}
        onClick={() => onGo(target)}
      >
        {text}
      </RowButton>
    </li>
  )
}
