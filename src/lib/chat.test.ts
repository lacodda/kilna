import { describe, expect, it } from 'vitest'
import type { Message, Run } from '@/lib/api'
import { chatLabel, conversation } from '@/lib/chat'

let counter = 0
const message = (
  role: 'user' | 'assistant',
  body: string,
  at: string,
  meta: Record<string, unknown> = {},
): Message => ({
  id: `m${String(++counter)}`,
  chat_id: 'c1',
  role,
  body,
  meta,
  created_at: at,
})

const run = (id: string, prompt: string, at: string, partial: Partial<Run> = {}): Run => ({
  id,
  chat_id: 'c1',
  prompt,
  state: 'done',
  events: [],
  started_at: at,
  ...partial,
})

describe('conversation', () => {
  it('pairs both sides of a tagged exchange with their run', () => {
    const items = conversation(
      [
        message('user', 'shorten the verse', 't1', { run_id: 'r1' }),
        message('assistant', 'done, here it is', 't2', { run_id: 'r1', cost_usd: 0.12 }),
      ],
      [run('r1', 'shorten the verse', 't1', { events: [{ kind: 'tool', name: 'Read', detail: 'x' }] })],
    )

    expect(items).toHaveLength(1)
    expect(items[0]?.prompt).toBe('shorten the verse')
    expect(items[0]?.run?.steps).toEqual(['Read · x'])
    expect(items[0]?.answer?.body).toBe('done, here it is')
    expect(items[0]?.answer?.cost).toBe(0.12)
  })

  it('pairs the untagged past by text, oldest first, one to one', () => {
    // The same question asked twice, stored before messages carried run ids:
    // each message must claim its own run, in order, with no duplicates.
    const items = conversation(
      [
        message('user', 'again', 't1'),
        message('assistant', 'first answer', 't2'),
        message('user', 'again', 't3'),
        message('assistant', 'second answer', 't4'),
      ],
      [run('r1', 'again', 't1'), run('r2', 'again', 't3')],
    )

    expect(items).toHaveLength(2)
    expect(items[0]?.answer?.body).toBe('first answer')
    expect(items[1]?.answer?.body).toBe('second answer')
    expect(items[0]?.run).not.toBeNull()
    expect(items[1]?.run).not.toBeNull()
  })

  it('keeps a plain transcript readable with no runs at all', () => {
    const items = conversation(
      [message('user', 'hello', 't1'), message('assistant', 'hi', 't2')],
      [],
    )

    expect(items).toHaveLength(1)
    expect(items[0]?.run).toBeNull()
    expect(items[0]?.answer?.body).toBe('hi')
  })

  it('lets an answer without a recorded question stand alone', () => {
    const items = conversation([message('assistant', 'orphan', 't1')], [])

    expect(items).toHaveLength(1)
    expect(items[0]?.prompt).toBeNull()
    expect(items[0]?.answer?.body).toBe('orphan')
  })

  it('shows a run whose question never reached the transcript', () => {
    const items = conversation([], [run('r1', 'lost question', 't1')])

    expect(items).toHaveLength(1)
    expect(items[0]?.prompt).toBe('lost question')
    expect(items[0]?.run).not.toBeNull()
  })

  it('keeps a stopped run under its own question, not the next one', () => {
    const items = conversation(
      [
        message('user', 'first', 't1', { run_id: 'r1' }),
        message('user', 'second', 't2', { run_id: 'r2' }),
      ],
      [
        run('r1', 'first', 't1', { state: 'cancelled', events: [{ kind: 'stopped' }] }),
        run('r2', 'second', 't2', { state: 'running' }),
      ],
    )

    expect(items[0]?.run?.cancelled).toBe(true)
    expect(items[1]?.run?.working).toBe(true)
  })

  it('an answer naming its run lands on its own question, not the newest waiting one', () => {
    // Two runs going at once, and the one asked first finishes first: its
    // answer must reach the first question even though the second is also
    // still waiting — which is exactly what guessing by order gets wrong.
    const items = conversation(
      [
        message('user', 'asked first', 't1', { run_id: 'r1' }),
        message('user', 'asked second', 't2', { run_id: 'r2' }),
        message('assistant', 'answer to the first', 't3', { run_id: 'r1' }),
      ],
      [run('r1', 'asked first', 't1'), run('r2', 'asked second', 't2', { state: 'running' })],
    )

    expect(items[0]?.answer?.body).toBe('answer to the first')
    expect(items[1]?.answer).toBeNull()
  })

  it('orders exchanges by time even when a run had to stand alone', () => {
    const items = conversation(
      [message('user', 'late', 't5', { run_id: 'r2' })],
      [run('r1', 'early and lost', 't1'), run('r2', 'late', 't5')],
    )

    expect(items.map((item) => item.prompt)).toEqual(['early and lost', 'late'])
  })
})

describe('a proposal on an answer', () => {
  it('is carried onto the exchange', () => {
    const items = conversation(
      [
        message('user', 'score it', 't1', { run_id: 'r1' }),
        message('assistant', 'Here you go.', 't2', {
          run_id: 'r1',
          proposal: { kind: 'score', axes: { hook: 8 }, note: 'the chorus lands' },
        }),
      ],
      [],
    )

    expect(items[0]?.answer?.proposal?.axes).toEqual({ hook: 8 })
    expect(items[0]?.answer?.proposal?.note).toBe('the chorus lands')
  })

  it('is absent on an ordinary answer', () => {
    const items = conversation(
      [message('user', 'ask', 't1'), message('assistant', 'prose', 't2')],
      [],
    )

    expect(items[0]?.answer?.proposal).toBeNull()
  })

  it('ignores a shape this build does not understand', () => {
    // A newer backend could attach something else entirely; an unreadable
    // proposal must not put a broken apply button on the answer.
    const items = conversation(
      [
        message('user', 'ask', 't1'),
        message('assistant', 'answer', 't2', { proposal: { kind: 'telepathy' } }),
      ],
      [],
    )

    expect(items[0]?.answer?.proposal).toBeNull()
  })
})

describe('chatLabel', () => {
  it('prefers the given name, then the first question, then the fallback', () => {
    const base = { id: 'c', work_id: null, cost_usd: 0, updated_at: 't' }

    expect(chatLabel({ ...base, title: 'Named', first_prompt: 'asked' }, 'New chat')).toBe('Named')
    expect(chatLabel({ ...base, first_prompt: 'asked' }, 'New chat')).toBe('asked')
    expect(chatLabel(base, 'New chat')).toBe('New chat')
  })
})
