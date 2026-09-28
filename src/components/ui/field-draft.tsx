import { useState, type KeyboardEvent } from 'react'

/*
 * FieldDraft.
 *
 * The rules between a value stored somewhere and the text someone is typing
 * over it, as a pure step and a hook to spread on an input.
 *
 * A field that writes to a database, a file or a server holds two things at
 * once - what is stored and what is in the box - and every product that wrote
 * one by hand broke a different rule between them. kilna broke all of them,
 * one screen at a time, which is why they are written down here:
 *
 * - **Leaving writes only a change.** The Overview wrote on every blur, so
 *   tabbing through twelve fields was twelve operations in the history and
 *   twelve toasts.
 * - **While nobody is typing, the box follows the stored value.** The fields
 *   were uncontrolled, so a value a plugin wrote kept showing the old one -
 *   and the next blur wrote the old one back over it.
 * - **While someone is typing, their words win until they leave.** A refetch
 *   in the middle of a sentence does not take it away from them.
 * - **Escape puts back what is stored and writes nothing.**
 * - **Enter on something unreadable keeps the words and says so.** "1:75" is
 *   a typo one key from right; throwing it away teaches nothing. Leaving the
 *   field with it is different: the box goes back to what is stored, because
 *   a box showing a value that was not saved is a second truth.
 *
 * `stepDraft` is the whole of it, pure, so it is tested without a DOM; the
 * hook only wires it to an input. The donor is kilna's `lib/fieldDraft.ts`.
 * InlineField is built on this, and a product's own field can use it directly:
 * spread `useFieldDraft(...)` onto an Input or a Textarea.
 */

export interface DraftState {
  /** What the box shows. */
  draft: string
  /** What is stored, as last seen, in the same spelling the box uses. */
  stored: string
  /** Whether someone is in the field. */
  editing: boolean
  /** Whether the last attempt to commit was refused as unreadable. */
  invalid: boolean
}

export type DraftEvent =
  | { type: 'stored'; value: string }
  | { type: 'focus' }
  | { type: 'type'; text: string }
  | { type: 'refuse' }
  | { type: 'escape' }
  | { type: 'leave' }

export function startDraft(stored: string): DraftState {
  return { draft: stored, stored, editing: false, invalid: false }
}

/** The next state, and the text to write when the event commits one. */
export function stepDraft(state: DraftState, event: DraftEvent): { state: DraftState; commit: string | null } {
  switch (event.type) {
    case 'stored':
      return {
        // Mid-sentence, the new stored value is noted but the box is left
        // alone; otherwise the box takes it at once.
        state: state.editing
          ? { ...state, stored: event.value }
          : { draft: event.value, stored: event.value, editing: false, invalid: false },
        commit: null,
      }
    case 'focus':
      return { state: { ...state, editing: true }, commit: null }
    case 'type':
      return { state: { ...state, draft: event.text, editing: true, invalid: false }, commit: null }
    case 'refuse':
      return { state: { ...state, invalid: true }, commit: null }
    case 'escape':
      return { state: { ...state, draft: state.stored, editing: false, invalid: false }, commit: null }
    case 'leave':
      return {
        state: { ...state, editing: false, invalid: false },
        commit: state.draft !== state.stored ? state.draft : null,
      }
  }
}

type Editable = HTMLInputElement | HTMLTextAreaElement

export interface FieldDraftOptions {
  /** Keep Enter for new lines, for a Textarea. Otherwise Enter leaves the
   * field, which writes it. */
  multiline?: boolean
  /** Whether the typed text can be written at all. Unreadable text is never
   * handed to `onCommit`: on Enter the field keeps it and is marked invalid,
   * on leaving it goes back to what is stored. */
  readable?: (text: string) => boolean
}

/**
 * An input's props for a stored value - `value`, `aria-invalid` and the
 * handlers, nothing else - to spread onto an Input or a Textarea.
 *
 * `value` is the stored value already spelled as text. `onCommit` receives
 * the typed text when it differs, and may refuse it by returning `false` - a
 * server that said no, a number past the end of the board - and the box goes
 * back to what is stored.
 */
export function useFieldDraft(
  value: string,
  onCommit: (text: string) => boolean | void,
  { multiline = false, readable }: FieldDraftOptions = {},
) {
  const [state, setState] = useState(() => startDraft(value))

  // The stored value moved underneath - a refetch, a plugin, an undo. Taken in
  // during render, the way a controlled value is, so the first paint after it
  // already shows it.
  if (value !== state.stored) {
    setState(stepDraft(state, { type: 'stored', value }).state)
  }

  const send = (event: DraftEvent) => {
    const next = stepDraft(state, event)
    if (next.commit === null) {
      setState(next.state)
      return
    }
    const refused = (readable !== undefined && !readable(next.commit)) || onCommit(next.commit) === false
    setState(refused ? stepDraft(next.state, { type: 'escape' }).state : next.state)
  }

  return {
    value: state.draft,
    'aria-invalid': state.invalid || undefined,
    onFocus: () => send({ type: 'focus' }),
    onChange: (event: { target: { value: string } }) => send({ type: 'type', text: event.target.value }),
    onBlur: () => send({ type: 'leave' }),
    onKeyDown: (event: KeyboardEvent<Editable>) => {
      if (event.key === 'Escape') {
        // Taken here, so a screen-level Escape - closing a panel, a dialog -
        // does not also fire while the field is only putting its text back.
        event.stopPropagation()
        send({ type: 'escape' })
        return
      }
      // Not while an input method is composing: Enter there picks the
      // candidate, and leaving the field would commit half a word.
      if (event.key !== 'Enter' || multiline || event.nativeEvent.isComposing) return
      event.preventDefault()
      if (readable !== undefined && state.draft !== state.stored && !readable(state.draft)) {
        send({ type: 'refuse' })
        return
      }
      event.currentTarget.blur()
    },
  }
}
