import { useCallback, useEffect, useRef, useState, type ButtonHTMLAttributes } from 'react'
import { cn } from 'dowel-ui'

/*
 * The copy affordance in the corner of a block.
 *
 * Whatever a product shows in a panel - code, a payload, a log, one side of a
 * comparison - somebody eventually wants to take it away, and the button that
 * lets them is written again every time with the same three things missed.
 *
 *   **It confirms only after the clipboard does.** The write can be refused:
 *   it needs a secure context and, in some browsers, a permission. A tick
 *   drawn on click is a lie in exactly the case the reader most needs the
 *   truth.
 *
 *   **It says so as well as showing it.** A tick that appears silently tells a
 *   sighted reader it worked and tells nobody else. The live region is the
 *   part that actually reports.
 *
 *   **It stays reachable without a pointer.** Revealed on hover, which is
 *   right - a permanent button in the corner of every block is clutter - and
 *   on its own that makes it unreachable by keyboard. It is visible whenever
 *   it has focus too, and that pairing is the whole trick.
 *
 * Copyable is the other shape of this, and the two are not interchangeable:
 * that one is a value sitting in a sentence - inline, truncating, showing the
 * text it copies - and this one is a control beside content already on screen,
 * so it carries an icon and no words.
 */

export interface CopyButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'onCopy'> {
  /** What lands on the clipboard. */
  value: string
  /** What the button is called, for a screen reader. Required, and
   * deliberately without a default: a string this component invents is a
   * string the product cannot translate, and it would ship in English to
   * every reader who does not read English. */
  label: string
  /** What is announced after a successful copy. Required for the same
   * reason. */
  copiedLabel: string
  /** Told what happened, for a product that wants its own toast. `false` means
   * the clipboard refused. */
  onCopy?: (ok: boolean) => void
}

export function CopyButton({
  value,
  label,
  copiedLabel,
  onCopy,
  className,
  ...props
}: CopyButtonProps) {
  const [copied, setCopied] = useState(false)
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined)

  // A component that sets state on a timer has to stop when it goes away, or
  // it wakes up in a tree that no longer exists.
  useEffect(() => () => clearTimeout(timer.current), [])

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      onCopy?.(true)
      clearTimeout(timer.current)
      timer.current = setTimeout(() => setCopied(false), 1600)
    } catch {
      onCopy?.(false)
    }
  }, [value, onCopy])

  return (
    <>
      <button
        type="button"
        onClick={copy}
        aria-label={copied ? copiedLabel : label}
        className={cn(
          'shrink-0 rounded-sm p-1 text-faint transition-colors target-min',
          'hover:bg-soft hover:text-text',
          /* `group-hover` rather than a hover of its own: the button is in the
           * corner of a block, and it has to appear when the pointer is
           * anywhere over that block rather than only once it has found the
           * button. The container carries `group`. */
          'opacity-0 group-hover:opacity-100 focus-visible:opacity-100',
          'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
          copied && 'text-good opacity-100',
          className,
        )}
        {...props}
      >
        {copied ? <Tick /> : <Clipboard />}
      </button>

      {/* The drawn tick is invisible to a screen reader; this is the part that
        * reports the copy. Outside the button, because its content changes and
        * a live region inside a labelled control is announced twice. */}
      <span role="status" aria-live="polite" className="sr-only">
        {copied ? copiedLabel : ''}
      </span>
    </>
  )
}

function Clipboard() {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden>
      <rect x="5.5" y="2.5" width="8" height="10" rx="1.5" />
      <path d="M10.5 2.5v-.5a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h.5" />
    </svg>
  )
}

function Tick() {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M3 8.5l3.5 3.5L13 5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}
