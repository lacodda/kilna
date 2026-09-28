import {
  useRef,
  useState,
  type HTMLAttributes,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import type { Cut } from '@/lib/api/types'
import { dragged, type Grip, type Stretch } from '@/lib/cuts'
import { formatSeconds } from '@/lib/timecode'
import { cn } from '@/lib/utils'
import { Track, type TrackSegment } from '@/components/ui/track'
import { place } from '@/components/ui/track-segments'

interface Props {
  /** The stretches on this donor, where they are to be drawn. */
  cuts: Cut[]
  /** The donor's length in seconds: what the track is drawn against. */
  duration: number
  /** What the whole track is, for a reader who cannot see it. */
  label: string
  /** What a stretch is called on hover, when its own label does not say. */
  nameOf?: (cut: Cut) => string | null
  /**
   * Where a stretch is while it is dragged, and once more with `done` when it
   * is let go - where it was taken from, when the drag was cancelled. Absent
   * on a track that is only read.
   */
  onMove?: (id: string, stretch: Stretch, done: boolean) => void
  disabled?: boolean
}

/** What a stretch's body and each of its ends answer the pointer with. */
type Handlers = Pick<
  HTMLAttributes<HTMLElement>,
  'onPointerDown' | 'onPointerMove' | 'onPointerUp' | 'onPointerCancel'
>

/** A drag in progress: which stretch, by what, from where. */
interface Hold {
  id: string
  grip: Grip
  /** Where the pointer went down, in client pixels. */
  x: number
  /** How wide the track was then: pixels per donor length. */
  width: number
  from: Stretch
}

/**
 * One donor drawn as its own length, with the stretches taken out of it.
 *
 * The bar is dowel's Track; what this adds is a hand on each stretch. The
 * body of a stretch moves it whole, each of its edges moves that end - the
 * accent lines the mockup draws at a stretch's ends are those handles. A
 * person deciding whether to take another twelve seconds is asking "where,
 * relative to what I already took", and dragging the stretch there answers it
 * where it is asked. The keyboard finishes what the eye placed: the seconds
 * under the track are the exact values, and the track is the pointer's way to
 * the same thing, which is why it is hidden from a screen reader rather than
 * made a second set of controls.
 *
 * Nothing is written while the pointer moves. The stretch follows it through
 * `onMove`, and one change is made when it is let go: one write, one line of
 * history, one undo.
 */
export function CutTrack({ cuts, duration, label, nameOf, onMove, disabled = false }: Props) {
  const surface = useRef<HTMLDivElement>(null)
  const hold = useRef<Hold | null>(null)
  // Only for the cursor: which stretch is in the hand.
  const [holding, setHolding] = useState<string | null>(null)
  const movable = onMove !== undefined && !disabled

  const segments: TrackSegment[] = cuts.map((cut) => {
    const span = `${formatSeconds(cut.starts_at)} – ${formatSeconds(cut.ends_at)}`
    const name = nameOf?.(cut) ?? cut.label
    return {
      key: cut.id,
      start: cut.starts_at,
      end: cut.ends_at,
      // The quieter fill, so the accent edges read as the stretch's ends.
      tone: 'past',
      label: name === null || name === '' ? span : `${name} · ${span}`,
    }
  })
  // The same geometry the Track lays its segments out by, so the handles sit
  // exactly over what it draws.
  const placed = place(segments, { from: 0, to: duration })

  /** Where the held stretch would land at this pointer. */
  const landing = (event: ReactPointerEvent<HTMLElement>): Stretch | null => {
    const current = hold.current
    if (current === null) return null
    const delta = ((event.clientX - current.x) / current.width) * duration
    return dragged(current.from, current.grip, delta, duration)
  }

  const letGo = (event: ReactPointerEvent<HTMLElement>) => {
    const current = hold.current
    if (current === null) return
    const next = event.type === 'pointercancel' ? current.from : (landing(event) ?? current.from)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
    hold.current = null
    setHolding(null)
    onMove?.(current.id, next, true)
  }

  // Rebuilt every render on purpose, as the reorder grip's are: they close
  // over the stretch and the callback, and memoising them would mean listing
  // both and getting one wrong.
  const grips = (cut: Cut, grip: Grip): Handlers => ({
    onPointerDown: (event) => {
      if (!movable || event.button !== 0) return
      const box = surface.current?.getBoundingClientRect()
      if (box === undefined || box.width <= 0) return
      // Stopped here, so taking an edge does not also take the body it sits in.
      event.preventDefault()
      event.stopPropagation()
      event.currentTarget.setPointerCapture(event.pointerId)
      hold.current = {
        id: cut.id,
        grip,
        x: event.clientX,
        width: box.width,
        from: { starts_at: cut.starts_at, ends_at: cut.ends_at },
      }
      setHolding(cut.id)
    },
    onPointerMove: (event) => {
      const current = hold.current
      const next = landing(event)
      if (current !== null && next !== null) onMove?.(current.id, next, false)
    },
    onPointerUp: letGo,
    onPointerCancel: letGo,
  })

  return (
    <div className="relative">
      <Track
        segments={segments}
        from={0}
        to={duration}
        label={label}
        // The mockup's `.track`: a 30px lane sunk into the panel, which is
        // the height at which a stretch is something to take hold of rather
        // than a line to read.
        className="h-7.5 rounded-md border border-line bg-bg"
      />
      {/* Over the Track's own segments, inside its border, at their places. */}
      <div ref={surface} aria-hidden className="absolute inset-px">
        {placed.map((geometry, index) => {
          const cut = cuts[index]!
          return (
            <span
              key={cut.id}
              title={segments[index]!.label}
              style={{ left: `${geometry.left}%`, width: `${geometry.width}%` }}
              className={cn(
                'absolute inset-y-0',
                movable && 'cursor-grab touch-none',
                holding === cut.id && 'cursor-grabbing',
              )}
              {...(movable ? grips(cut, 'whole') : {})}
            >
              <Edge side="start" movable={movable} {...(movable ? grips(cut, 'start') : {})} />
              <Edge side="end" movable={movable} {...(movable ? grips(cut, 'end') : {})} />
            </span>
          )
        })}
      </div>
    </div>
  )
}

/**
 * One end of a stretch: a two-pixel accent line, in a hit area wide enough to
 * find with a pointer.
 */
function Edge({
  side,
  movable,
  ...handlers
}: {
  side: 'start' | 'end'
  movable: boolean
} & Handlers) {
  return (
    <span
      className={cn(
        'absolute inset-y-0 w-2',
        side === 'start' ? '-left-1' : '-right-1',
        movable && 'cursor-ew-resize touch-none',
      )}
      {...handlers}
    >
      <span
        className={cn(
          'absolute inset-y-0 w-0.5 bg-accent',
          side === 'start' ? 'left-1' : 'right-1',
        )}
      />
    </span>
  )
}
