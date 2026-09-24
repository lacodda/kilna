import { useState, type KeyboardEvent } from 'react'

/*
 * A field that is typed into and written somewhere else - the value stored in
 * the database, the draft in the box, and the rules between them.
 *
 * The rules, because each one was broken somewhere before this existed:
 * - Leaving a field writes it only when it changed. The Overview wrote on
 *   every blur, so tabbing through twelve fields was twelve operations in the
 *   log and twelve toasts.
 * - While nobody is typing, the box follows the stored value. The Overview
 *   fields were uncontrolled, so a value a plugin wrote kept showing the old
 *   one, and the next blur wrote the old one back over it.
 * - While someone is typing, their words win until they leave.
 * - Escape puts back what is stored and writes nothing.
 *
 * `step` is the whole of it, pure, so it is tested without a DOM; the hook
 * only wires it to an input.
 */

export interface DraftState {
  /** What the box shows. */
  draft: string
  /** What is stored, as last seen. */
  stored: string
  /** Whether someone is in the field. */
  editing: boolean
}

export type DraftEvent =
  | { type: 'stored'; value: string }
  | { type: 'focus' }
  | { type: 'type'; text: string }
  | { type: 'escape' }
  | { type: 'leave' }

export function start(stored: string): DraftState {
  return { draft: stored, stored, editing: false }
}

/** The next state, and the text to write when the event commits one. */
export function step(state: DraftState, event: DraftEvent): { state: DraftState; commit: string | null } {
  switch (event.type) {
    case 'stored':
      return {
        state: state.editing
          ? { ...state, stored: event.value }
          : { draft: event.value, stored: event.value, editing: false },
        commit: null,
      }
    case 'focus':
      return { state: { ...state, editing: true }, commit: null }
    case 'type':
      return { state: { ...state, draft: event.text, editing: true }, commit: null }
    case 'escape':
      return { state: { ...state, draft: state.stored, editing: false }, commit: null }
    case 'leave':
      return {
        state: { ...state, editing: false },
        commit: state.draft !== state.stored ? state.draft : null,
      }
  }
}

type Element = HTMLInputElement | HTMLTextAreaElement

/**
 * An input's props for a stored value: `value`, the handlers, and nothing
 * else - spread them onto an `Input` or a `Textarea`. `multiline` keeps Enter
 * for new lines; otherwise Enter leaves the field, which writes it.
 *
 * `onCommit` may refuse what was typed by returning `false` - a timecode that
 * does not parse, a number past the end of the board - and the box goes back
 * to what is stored: a field left showing a value that was not saved is a
 * second truth about the thing it names.
 */
export function useFieldDraft(
  value: string,
  onCommit: (text: string) => boolean | void,
  { multiline = false }: { multiline?: boolean } = {},
) {
  const [state, setState] = useState(() => start(value))

  // The stored value moved underneath - a refetch, a plugin, an undo. Taken in
  // during render, the way a controlled value is, so the first paint after it
  // already shows it.
  if (value !== state.stored) {
    setState(step(state, { type: 'stored', value }).state)
  }

  const send = (event: DraftEvent) => {
    const next = step(state, event)
    const refused = next.commit !== null && onCommit(next.commit) === false
    setState(refused ? step(next.state, { type: 'escape' }).state : next.state)
  }

  return {
    value: state.draft,
    onFocus: () => send({ type: 'focus' }),
    onChange: (event: { target: { value: string } }) => send({ type: 'type', text: event.target.value }),
    onBlur: () => send({ type: 'leave' }),
    onKeyDown: (event: KeyboardEvent<Element>) => {
      if (event.key === 'Escape') {
        // Taken here, so a screen-level Escape (closing a stage, a dialog)
        // does not also fire while the field is only putting its text back.
        event.stopPropagation()
        send({ type: 'escape' })
      } else if (event.key === 'Enter' && !multiline) {
        event.currentTarget.blur()
      }
    },
  }
}
