import { useCallback, useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { createVersion, updateVersionBody, type Version } from '@/lib/api'
import { begun, continues, touched, type Session } from '@/lib/editing'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'

/** How long after the last keystroke the text is written. */
const SETTLE_MS = 600

export type EditingStatus = 'idle' | 'saving' | 'saved'

interface Options {
  workId: string
  role: string
  /** The version whose text is on screen, once loaded. */
  open: Version | null | undefined
  /** Called with the id of a version the session minted, so the panel can open it. */
  onMinted: (id: string) => void
  /** What to say when a write fails. */
  failure: string
}

/**
 * Text that saves itself into the version of the sitting.
 *
 * The first change to an open version mints the next revision from it and
 * makes that the open one; every change after goes into that revision, a
 * moment after the typing pauses. The rule for which changes share a version
 * is `lib/editing`; this is the plumbing that applies it — one write at a
 * time, in order, so a keystroke that arrives while the version is still
 * being minted lands in the minted version rather than in a second one.
 *
 * A scored version refuses to change (the backend answers `frozen`: a score
 * is a snapshot of the text it read). The refusal is not shown — the change
 * is what the person meant, so it goes into the next revision instead, which
 * is exactly what a first change to that version would have done.
 */
export function useBodyEditing({ workId, role, open, onMinted, failure }: Options) {
  const client = useQueryClient()

  const [text, setTextState] = useState(open?.body ?? '')
  const [status, setStatus] = useState<EditingStatus>('idle')

  // Refs rather than state for everything the write chain reads: a chain
  // started on one render must see the keystrokes of the renders after it.
  const latest = useRef(text)
  // The version the text on screen belongs to, and the body it last had on
  // disk. `target` is set the moment a version is minted, before the query
  // for it resolves, so the open version changing to it does not reset the
  // text to what was on disk a keystroke ago.
  const target = useRef<string | null>(open?.id ?? null)
  const saved = useRef(open?.body ?? '')
  const session = useRef<Session | null>(null)
  const chain = useRef<Promise<void>>(Promise.resolve())
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const tick = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  // Another version was opened: show its text. The one the session minted
  // arrives here too, already matching `target`, and is left alone.
  useEffect(() => {
    if (open == null || open.id === target.current) return
    target.current = open.id
    saved.current = open.body
    latest.current = open.body
    setTextState(open.body)
  }, [open])

  const settle = useCallback(
    (ids: string[]) => {
      for (const id of ids) void client.invalidateQueries({ queryKey: keys.version(id) })
      for (const key of [keys.versions(workId), keys.work(workId), keys.works, keys.journal]) {
        void client.invalidateQueries({ queryKey: key })
      }
    },
    [client, workId],
  )

  const showSaved = useCallback(() => {
    setStatus('saved')
    clearTimeout(tick.current)
    tick.current = setTimeout(() => setStatus('idle'), 2000)
  }, [])

  const mint = useCallback(
    async (parentId: string, body: string) => {
      const created = await createVersion(workId, {
        role,
        body,
        label: null,
        make_current: true,
        parent_version_id: parentId,
      })
      target.current = created.id
      saved.current = body
      session.current = begun(created.id, role, Date.now())
      onMinted(created.id)
      return created.id
    },
    [workId, role, onMinted],
  )

  const persist = useCallback(async () => {
    const body = latest.current
    const id = target.current
    if (id === null || body === saved.current) return

    setStatus('saving')
    try {
      const now = Date.now()
      let written = id
      if (continues(session.current, role, id, now)) {
        try {
          await updateVersionBody(id, body)
          saved.current = body
          session.current = touched(session.current!, now)
        } catch (cause) {
          if (!isFrozen(cause)) throw cause
          written = await mint(id, body)
        }
      } else {
        written = await mint(id, body)
      }
      showSaved()
      settle([id, written])
    } catch (cause) {
      setStatus('idle')
      say.failedTo(failure, cause)
    }
  }, [role, mint, settle, showSaved, failure])

  /** Write now rather than after the pause. */
  const flush = useCallback(() => {
    clearTimeout(timer.current)
    chain.current = chain.current.then(persist)
    return chain.current
  }, [persist])

  const setText = useCallback(
    (next: string) => {
      latest.current = next
      setTextState(next)
      clearTimeout(timer.current)
      timer.current = setTimeout(() => void flush(), SETTLE_MS)
    },
    [flush],
  )

  // Leaving the text writes what is pending and ends the sitting.
  useEffect(
    () => () => {
      clearTimeout(tick.current)
      if (latest.current !== saved.current) void flush()
    },
    [flush],
  )

  return { text, setText, status, flush }
}

function isFrozen(cause: unknown): boolean {
  return (
    typeof cause === 'object' &&
    cause !== null &&
    (cause as { kind?: unknown }).kind === 'frozen'
  )
}
