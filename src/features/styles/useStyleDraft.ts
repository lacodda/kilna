import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { updateStyleBrick } from '@/lib/api/styles'
import type { StyleBrick } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { adopt, formOf, patchOf, type StyleForm } from '@/lib/styleDraft'
import { say } from '@/lib/toast'
import type { SaveStatus } from '@/components/ui/save-state'

/** How long after the last keystroke the style is written. */
const SETTLE_MS = 600

interface Options {
  /** Whether the form's name is one the backend would refuse; its type and
   * name are held back while it is (see `patchOf`). */
  holdKey: (form: StyleForm) => boolean
  /** What to say when a write fails. */
  failure: string
}

/**
 * A style open in its editor, writing itself a moment after the typing pauses.
 *
 * The notes' body does the same for one field; a style has five, and a
 * stored side that changes under the editor - the assistant keeps a
 * description it wrote, an undo puts a name back - so what arrives is taken
 * in by the rules of `lib/styleDraft` rather than by replacing the form. One
 * write at a time, in order, so a keystroke made while the last write is in
 * flight is not overtaken by it.
 *
 * A picked type or status is written at once (`now`): it is a decision, not
 * a keystroke, and a pause before it would only be a window for losing it.
 */
export function useStyleDraft(brick: StyleBrick, { holdKey, failure }: Options) {
  const client = useQueryClient()
  const [form, setForm] = useState(() => formOf(brick))
  const [status, setStatus] = useState<SaveStatus>('idle')

  // Refs rather than state for everything the write chain reads: a write
  // started on one render must see the keystrokes of the renders after it.
  const latest = useRef(form)
  /** The stored brick as last seen, in the form's spelling. */
  const base = useRef(form)
  /** The newest moment taken in, so an answer that left before a write and
   * arrived after it does not put the old words back. */
  const seen = useRef(brick.updated_at)
  const chain = useRef<Promise<void>>(Promise.resolve())
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const tick = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const hold = useRef(holdKey)
  /** Set once the style is deleted: nothing pending may be written after it. */
  const gone = useRef(false)

  useEffect(() => {
    hold.current = holdKey
  }, [holdKey])

  const takeIn = useCallback((incoming: StyleBrick, wrote: StyleForm = base.current) => {
    if (incoming.updated_at < seen.current) return
    seen.current = incoming.updated_at
    const next = adopt(latest.current, wrote, formOf(incoming))
    base.current = next.base
    if (JSON.stringify(next.form) !== JSON.stringify(latest.current)) {
      latest.current = next.form
      setForm(next.form)
    }
  }, [])

  // The stored brick changed: a write of this editor's coming back, or a
  // change made somewhere else. The query shares structure, so this runs only
  // when something in the brick is actually different.
  useEffect(() => {
    takeIn(brick)
  }, [brick, takeIn])

  const persist = useCallback(async () => {
    if (gone.current) return
    const snapshot = latest.current
    const patch = patchOf(snapshot, base.current, { keyHeld: hold.current(snapshot) })
    if (patch === null) return
    setStatus('saving')
    try {
      const written = await updateStyleBrick(brick.id, patch)
      // What was written is the base for the fields it carried, spelled as
      // the box spelled them: a field typed back to what it was while the
      // write was out is then an edit of its own, not the answer's to undo.
      const wrote: StyleForm = { ...base.current }
      for (const field of Object.keys(patch) as (keyof StyleForm)[]) {
        Object.assign(wrote, { [field]: snapshot[field] })
      }
      takeIn(written, wrote)
      setStatus('saved')
      clearTimeout(tick.current)
      tick.current = setTimeout(() => setStatus('idle'), 2000)
      // The lists and the counts, not the references: a written field moves
      // no picture, and every card on the screen holds a query for its own.
      for (const key of [keys.styleBricks, keys.styleCounts, keys.journal]) {
        void client.invalidateQueries({ queryKey: key })
      }
    } catch (cause) {
      // The form keeps what was typed and the base what is stored, so the
      // next keystroke tries again.
      setStatus('idle')
      say.failedTo(failure, cause)
    }
  }, [brick.id, client, failure, takeIn])

  const flush = useCallback(() => {
    clearTimeout(timer.current)
    chain.current = chain.current.then(persist)
    return chain.current
  }, [persist])

  /** Change some fields; `now` writes at once rather than after the pause. */
  const edit = useCallback(
    (change: Partial<StyleForm>, now = false) => {
      const next = { ...latest.current, ...change }
      latest.current = next
      setForm(next)
      clearTimeout(timer.current)
      if (now) void flush()
      else timer.current = setTimeout(() => void flush(), SETTLE_MS)
    },
    [flush],
  )

  /** The style is going: drop what is pending rather than write it to a
   * brick that will not be there. Flush first when it should be kept. */
  const forget = useCallback(() => {
    gone.current = true
    clearTimeout(timer.current)
  }, [])

  // Leaving the style - closing it, opening another - writes what is pending.
  useEffect(
    () => () => {
      clearTimeout(tick.current)
      void flush()
    },
    [flush],
  )

  return { form, edit, status, flush, forget }
}
