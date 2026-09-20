import { useTranslation } from 'react-i18next'
import { CircleCheck, TriangleAlert } from 'lucide-react'
import type { Complaint, Storyboard } from '@/lib/storyboard'
import { formatSeconds } from '@/lib/timecode'
import { Panel, SectionLabel } from '@/components/ui/panel'

/*
 * What the board still owes, above the board.
 *
 * The harvest of a storyboard: the counts a person would otherwise make by
 * scrolling, and the list of what is missing. It sits above the table rather
 * than in a dialog because it is not a verdict to be dismissed — it is the
 * queue the next hour of work comes out of, and a queue you have to reopen to
 * see is a queue you stop looking at.
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
  const { tally, complaints } = board

  // An empty board has nothing to harvest. It is not "done" and it is not
  // "wrong" — it is a board before the work, and the tab already says so.
  if (tally.scenes === 0) return null

  return (
    <Panel className="flex flex-col gap-3 p-4">
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <SectionLabel>{t('scenes.check.title')}</SectionLabel>
        {/* The counts, as a sentence rather than a row of tiles: five numbers
            about one board read as a sentence and tile up into a dashboard
            that says less. */}
        <p className="text-sm text-dim">
          {t('scenes.check.tally', {
            scenes: tally.scenes,
            written: tally.written,
            framed: tally.framed,
            filmed: tally.filmed,
          })}
        </p>
      </div>

      {complaints.length === 0 ? (
        <p className="flex items-center gap-2 text-sm text-good">
          <CircleCheck aria-hidden className="size-4 shrink-0" />
          {t('scenes.check.nothingMissing')}
        </p>
      ) : (
        <ul className="flex flex-col gap-1">
          {complaints.map((complaint, index) => (
            <Line
              // A complaint has no id of its own — it is a fact about a board,
              // computed fresh on every render. Its place in the list is what
              // identifies it, and the list is rebuilt whole each time.
              key={`${complaint.kind}:${complaint.run?.sceneId ?? complaint.scene?.id ?? index}`}
              complaint={complaint}
              onGo={onGo}
            />
          ))}
        </ul>
      )}
    </Panel>
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
      // The same padding the button below carries, so a line about the whole
      // board stands in the same column as the lines about scenes rather
      // than four pixels to their left — measured on a real board.
      <li className="flex items-start gap-2 px-1 py-0.5 text-sm text-dim">
        <TriangleAlert aria-hidden className="mt-0.5 size-3.5 shrink-0 text-warn" />
        <span>{text}</span>
      </li>
    )
  }

  return (
    <li>
      <button
        type="button"
        className="flex w-full items-start gap-2 rounded-inner px-1 py-0.5 text-left text-sm text-dim hover:bg-soft hover:text-text"
        onClick={() => onGo(target)}
      >
        <TriangleAlert aria-hidden className="mt-0.5 size-3.5 shrink-0 text-warn" />
        <span>{text}</span>
      </button>
    </li>
  )
}
