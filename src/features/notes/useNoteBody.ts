import { useCallback, useEffect, useRef, useState } from 'react'
import { updateNote } from '@/lib/api/notes'
import type { Note } from '@/lib/api/types'
import { say } from '@/lib/toast'
import type { SaveStatus } from '@/components/SaveState'

/** How long after the last keystroke the body is written. */
const SETTLE_MS = 600

/**
 * A note's body that saves itself a moment after the typing pauses.
 *
 * Simpler than a version's editor on purpose: a note has no revisions and no
 * score to freeze it, so every change goes into the same row. One write at a
 * time, in order, so a keystroke made while the last write is in flight is
 * not overtaken by it.
 */
export function useNoteBody(note: Note, settle: () => void, failure: string) {
  const [text, setTextState] = useState(note.body)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const latest = useRef(note.body)
  const saved = useRef(note.body)
  const chain = useRef<Promise<void>>(Promise.resolve())
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const tick = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  const persist = useCallback(async () => {
    const body = latest.current
    if (body === saved.current) return
    setStatus('saving')
    try {
      await updateNote(note.id, { body })
      saved.current = body
      setStatus('saved')
      clearTimeout(tick.current)
      tick.current = setTimeout(() => setStatus('idle'), 2000)
      settle()
    } catch (cause) {
      setStatus('idle')
      say.failedTo(failure, cause)
    }
  }, [note.id, settle, failure])

  const flush = useCallback(() => {
    clearTimeout(timer.current)
    chain.current = chain.current.then(persist)
    return chain.current
  }, [persist])

  /** Replace the text; `now` writes at once rather than after the pause —
   *  a ticked box is a decision, not a keystroke. */
  const setText = useCallback(
    (next: string, now = false) => {
      latest.current = next
      setTextState(next)
      clearTimeout(timer.current)
      if (now) void flush()
      else timer.current = setTimeout(() => void flush(), SETTLE_MS)
    },
    [flush],
  )

  // Leaving the note writes what is pending.
  useEffect(
    () => () => {
      clearTimeout(tick.current)
      if (latest.current !== saved.current) void flush()
    },
    [flush],
  )

  return { text, setText, status, flush }
}
