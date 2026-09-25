import { useState } from 'react'

/*
 * The two things every dialog of the app does on open, in one place so the
 * dialogs built on dowel's parts directly do them the same way as `AppDialog`.
 */

/** What a dialog focuses when it opens: a field to type into first, anything
 * else that can be focused in the body after that. */
const FIELD =
  'input:not([type=hidden]):not(:disabled), textarea:not(:disabled), select:not(:disabled), [contenteditable="true"]'
const FOCUSABLE = 'button:not(:disabled), [href], [tabindex]:not([tabindex="-1"]), ' + FIELD

/** The element a dialog should open on - its first field, else the first
 * thing in its body that takes focus - or `null` to let the caller fall back.
 * Not the close cross: it is the first tabbable in the popup, which is what a
 * dialog focuses by default, and a form that opens on its close button makes
 * the person reach for the mouse before typing a letter. */
export function firstFocus(body: ParentNode | null): HTMLElement | null {
  if (body === null) return null
  return body.querySelector<HTMLElement>(FIELD) ?? body.querySelector<HTMLElement>(FOCUSABLE)
}

/**
 * Whether anything has been typed into a dialog since it opened, and the
 * handler that notices - put `onInput` on the popup and pass `typed` to the
 * root's `disablePointerDismissal`.
 *
 * A click beside a dialog closed it and threw away what was in it. `Escape`
 * and Cancel still close it, because those are asked for; a stray click is
 * not. The `input` event is what every text field, textarea and checkbox
 * fires, and it bubbles through React's tree across portals, so nothing
 * inside has to report anything.
 *
 * `openedOn` is what identifies one opening - `open`, or the thing the dialog
 * was opened for. When it changes the flag starts over, in the render rather
 * than in an effect, so the first frame of a reopened dialog is already clean.
 */
export function useTypedSinceOpen(openedOn: unknown): { typed: boolean; onInput: () => void } {
  const [typed, setTyped] = useState(false)
  const [seen, setSeen] = useState(openedOn)
  if (!Object.is(openedOn, seen)) {
    setSeen(openedOn)
    setTyped(false)
  }
  return {
    typed,
    onInput: () => {
      if (!typed) setTyped(true)
    },
  }
}
