import { describe, expect, it } from 'vitest'
import type { Run, RunEvent } from './api'
import { inOrder, view, withEvent } from './runs'

const run = (events: RunEvent[], overrides: Partial<Run> = {}): Run => ({
  id: 'r1',
  chat_id: 'c1',
  prompt: 'Rewrite the second verse',
  state: 'running',
  events,
  started_at: '2026-08-24T10:00:00Z',
  ...overrides,
})

describe('view', () => {
  it('reads a run that is still going as working, with what it has said so far', () => {
    const drawn = view(
      run([
        { kind: 'started', session_id: 's' },
        { kind: 'tool', name: 'Read', detail: 'notes.md' },
        { kind: 'text', body: 'Here is one.' },
      ]),
    )

    expect(drawn.working).toBe(true)
    expect(drawn.steps).toEqual(['Read · notes.md'])
    expect(drawn.body).toBe('Here is one.')
    expect(drawn.failure).toBeNull()
  })

  it('joins the blocks of an answer in the order they arrived', () => {
    const drawn = view(
      run([
        { kind: 'text', body: 'First.' },
        { kind: 'tool', name: 'Grep', detail: 'chorus' },
        { kind: 'text', body: 'Second.' },
      ]),
    )

    expect(drawn.body).toBe('First.\n\nSecond.')
  })

  it('does not repeat the answer when the result restates the blocks', () => {
    const drawn = view(
      run(
        [
          { kind: 'text', body: 'A verse.' },
          { kind: 'finished', body: 'A verse.', cost_usd: 0.1 },
        ],
        { state: 'done' },
      ),
    )

    expect(drawn.body).toBe('A verse.')
    expect(drawn.cost).toBe(0.1)
    expect(drawn.working).toBe(false)
  })

  it('falls back to the result body when nothing was streamed', () => {
    // A short answer can arrive as a result and nothing else.
    const drawn = view(run([{ kind: 'finished', body: 'Yes.' }], { state: 'done' }))

    expect(drawn.body).toBe('Yes.')
  })

  it('names a tool that had no argument worth showing', () => {
    const drawn = view(run([{ kind: 'tool', name: 'Ponder', detail: '' }]))

    expect(drawn.steps).toEqual(['Ponder'])
  })

  it('shows the failure a run reported', () => {
    const drawn = view(
      run([{ kind: 'failed', message: 'Not logged in' }], { state: 'failed' }),
    )

    expect(drawn.failure).toBe('Not logged in')
    expect(drawn.working).toBe(false)
  })

  it('reports a stopped run as stopped, not as a failure with an English word', () => {
    const drawn = view(
      run([{ kind: 'text', body: 'half a th' }, { kind: 'stopped' }], {
        state: 'cancelled',
      }),
    )

    expect(drawn.cancelled).toBe(true)
    expect(drawn.failure).toBeNull()
    expect(drawn.working).toBe(false)
    expect(drawn.body).toBe('half a th')
  })

  it('explains a run the app abandoned, which never got to say anything', () => {
    const drawn = view(
      run([{ kind: 'text', body: 'half a th' }], {
        state: 'broken',
        detail: 'kilna closed while this run was going',
      }),
    )

    expect(drawn.failure).toBe('kilna closed while this run was going')
    expect(drawn.working).toBe(false)
  })

  it('leaves an abandoned run with nothing to say without an empty complaint', () => {
    const drawn = view(run([], { state: 'broken' }))

    expect(drawn.failure).toBeNull()
  })
})

describe('withEvent', () => {
  it('appends without disturbing the run it was given', () => {
    const before = run([{ kind: 'text', body: 'one' }])

    const after = withEvent(before, { kind: 'text', body: 'two' })

    expect(before.events).toHaveLength(1)
    expect(view(after).body).toBe('one\n\ntwo')
  })

  it('ends the run when the answer finishes', () => {
    const after = withEvent(run([]), { kind: 'finished', body: 'done' })

    expect(after.state).toBe('done')
    expect(view(after).working).toBe(false)
  })

  it('ends the run when it fails', () => {
    const after = withEvent(run([]), { kind: 'failed', message: 'stopped' })

    expect(after.state).toBe('failed')
    expect(view(after).failure).toBe('stopped')
  })

  it('ends the run when it is stopped by hand', () => {
    const after = withEvent(run([]), { kind: 'stopped' })

    expect(after.state).toBe('cancelled')
    expect(view(after).cancelled).toBe(true)
    expect(view(after).working).toBe(false)
  })

  it('leaves a run going for everything else', () => {
    for (const event of [
      { kind: 'started', session_id: 's' },
      { kind: 'text', body: 'x' },
      { kind: 'tool', name: 'Read', detail: 'x' },
    ] satisfies RunEvent[]) {
      expect(withEvent(run([]), event).state).toBe('running')
    }
  })

  it('replaying stored events gives the same view as receiving them live', () => {
    const events: RunEvent[] = [
      { kind: 'started', session_id: 's' },
      { kind: 'tool', name: 'Read', detail: 'notes.md' },
      { kind: 'text', body: 'A verse.' },
      { kind: 'finished', body: 'A verse.', cost_usd: 0.2 },
    ]

    const live = events.reduce(withEvent, run([]))
    const replayed = run(events, { state: 'done' })

    expect(view(live)).toEqual(view(replayed))
  })
})

describe('inOrder', () => {
  it('reads oldest first, the way a conversation goes', () => {
    const older = run([], { id: 'a', started_at: '2026-08-24T09:00:00Z' })
    const newer = run([], { id: 'b', started_at: '2026-08-24T10:00:00Z' })

    expect(inOrder([newer, older]).map((r) => r.id)).toEqual(['a', 'b'])
  })

  it('keeps a stable order for runs started in the same second', () => {
    const first = run([], { id: 'a' })
    const second = run([], { id: 'b' })

    expect(inOrder([second, first]).map((r) => r.id)).toEqual(['a', 'b'])
  })

  it('leaves the list it was given alone', () => {
    const runs = [run([], { id: 'b' }), run([], { id: 'a' })]

    inOrder(runs)

    expect(runs.map((r) => r.id)).toEqual(['b', 'a'])
  })
})
