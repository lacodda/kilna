import { useEffect, useState, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { spinnerVariants } from './spinner'

/*
 * SaveState.
 *
 * The quiet line beside a field that saves itself: "saving…", then a tick that
 * fades. It exists because a form without a Save button has to say what it did
 * anyway - otherwise the reader is left guessing whether their edit survived,
 * and the usual answer to that guess is to press Ctrl+S at a page that has no
 * such thing.
 *
 * Taken from kilna, where it has been in production, with the two things a
 * primitive may not keep: its words (they came from the product's i18n, and a
 * component with a string of its own cannot be translated) and its icons.
 *
 * The rules it already got right, and which are the whole reason not to write
 * this fresh:
 *
 * - **The tick decays.** One that never leaves stops meaning "just now" and
 *   becomes furniture.
 * - **Only a real save earns one.** Mounting next to an idle mutation must not
 *   flash a tick for a save that happened before the reader arrived.
 * - **A failure is not a save.** It is announced elsewhere - a toast, an error
 *   on the field - and saying "saved" underneath that is worse than silence.
 * - **The width is held.** The line sits next to a field; if it grew and shrank
 *   with its own text, the layout would twitch on every keystroke's worth of
 *   saving.
 */

/** What the indicator is showing. `idle` is invisible but still occupies its
 * width. */
export type SaveStatus = 'idle' | 'saving' | 'saved'

/**
 * Turns "is a mutation in flight" into the three states this shows.
 *
 * Only the tick is state: `saving` is read straight off the mutation, and the
 * tick decays on its own.
 *
 * @param isPending whether the save is in flight right now
 * @param isError whether the save that just finished failed
 * @param linger how long the tick stays, in milliseconds
 */
export function useSaveStatus(isPending: boolean, isError = false, linger = 2000): SaveStatus {
  const [justSaved, setJustSaved] = useState(false)
  // The falling edge of a real save is the only thing worth a tick. Without
  // this, mounting beside an idle mutation would show one immediately, for a
  // save the reader was not there for.
  const [wasPending, setWasPending] = useState(isPending)

  // Adjusted during render rather than in an effect: this is state that
  // follows a prop, and `react-hooks/set-state-in-effect` is right to say so.
  if (wasPending !== isPending) {
    setWasPending(isPending)
    setJustSaved(!isPending && !isError)
  }

  useEffect(() => {
    if (!justSaved) return
    const timer = setTimeout(() => setJustSaved(false), linger)
    return () => clearTimeout(timer)
  }, [justSaved, linger])

  if (isPending) return 'saving'
  return justSaved ? 'saved' : 'idle'
}

export interface SaveStateProps {
  /** What to show. Usually from `useSaveStatus`. */
  status: SaveStatus
  /** What to say while saving. The product's word, in the product's language. */
  savingLabel?: ReactNode
  /** What to say once it is saved. */
  savedLabel?: ReactNode
  className?: string
}

/**
 * The indicator itself.
 *
 * `aria-live="polite"` rather than `assertive`: this is a background fact, and
 * interrupting someone mid-sentence to tell them a field saved is precisely
 * the kind of announcement that makes people turn a screen reader's verbosity
 * down.
 */
export function SaveState({ status, savingLabel, savedLabel, className }: SaveStateProps) {
  return (
    <span
      aria-live="polite"
      className={cn(
        'inline-flex min-w-16 items-center gap-1 text-xs text-faint transition-opacity',
        // Kept in the layout when idle, so the row does not reflow around it.
        status === 'idle' && 'opacity-0',
        className,
      )}
    >
      {status === 'saving' && (
        <>
          {/* The ring, not `Spinner`.
            *
            * `Spinner` carries its own `role="status"` and `aria-live`, which
            * is right when it stands alone and wrong inside this: nesting one
            * live region in another gives a screen reader two things to
            * announce for one event. So the visual is reused through the
            * variants and the announcing is left to the span around it. */}
          <span aria-hidden className={cn(spinnerVariants({ size: 'sm', tone: 'current' }), 'border-r-transparent')} />
          {savingLabel}
        </>
      )}
      {status === 'saved' && (
        <>
          {/* The tick is the only place this component names a colour, and it
              names the token for "this went well" rather than a green. */}
          <svg
            viewBox="0 0 16 16"
            aria-hidden="true"
            className="size-3 text-good"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M3 8.5 6.5 12 13 4.5" />
          </svg>
          {savedLabel}
        </>
      )}
    </span>
  )
}
