import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import type { RunEmission } from '@/lib/api/types'
import { endingOf, isIdle, sayEndings, type Ending } from '@/lib/endings'

const t = i18n.t.bind(i18n)

function emission(kind: 'finished' | 'failed' | 'stopped' | 'text', task?: string): RunEmission {
  const event =
    kind === 'finished'
      ? { kind, body: 'ok' }
      : kind === 'failed'
        ? { kind, message: 'no' }
        : kind === 'text'
          ? { kind, body: 'half' }
          : { kind }
  return { run_id: 'r', chat_id: 'c', task, event } as RunEmission
}

const ending = (title: string | null, outcome: Ending['outcome'], task = true): Ending => ({
  chatId: 'c',
  title,
  outcome,
  task,
})

describe('what ended', () => {
  it('is an answer or a failure; a stopped run and a block of text end nothing', () => {
    expect(endingOf(emission('finished', 'critique:w1'), 'Tide')).toEqual({
      chatId: 'c',
      title: 'Tide',
      outcome: 'done',
      task: true,
    })
    expect(endingOf(emission('failed'), null)?.outcome).toBe('ended')
    expect(endingOf(emission('failed'), null)?.task).toBe(false)
    expect(endingOf(emission('stopped', 'critique:w1'), 'Tide')).toBeNull()
    expect(endingOf(emission('text'), 'Tide')).toBeNull()
  })

  it('is said at once only when nothing is going or waiting', () => {
    expect(isIdle({ running: 0, queued: 0 })).toBe(true)
    expect(isIdle({ running: 1, queued: 0 })).toBe(false)
    expect(isIdle({ running: 0, queued: 2 })).toBe(false)
    // Not knowing is not idle: hold rather than say early.
    expect(isIdle(undefined)).toBe(false)
  })
})

describe('the words for what was held', () => {
  it('says one ending the way its toast does', () => {
    expect(sayEndings([ending('Critique · Tide', 'done')], t).title).toBe(
      'Finished: Critique · Tide',
    )
    expect(sayEndings([ending('Critique · Tide', 'ended')], t).title).toBe(
      'Stopped without an answer: Critique · Tide',
    )
    // A question typed before the window was closed is an answer, not a task.
    expect(sayEndings([ending('Tighten the chorus', 'done', false)], t).title).toBe(
      'Answered: Tighten the chorus',
    )
    expect(sayEndings([ending(null, 'done', false)], t).title).toBe('The assistant answered.')
  })

  it('counts several, names three and says what did not come back', () => {
    const said = sayEndings(
      [
        ending('One', 'done'),
        ending('Two', 'done'),
        ending('Three', 'ended'),
        ending('Four', 'done'),
        ending('Five', 'done'),
      ],
      t,
    )
    expect(said.title).toBe('4 answers are ready')
    expect(said.body).toBe('One, Two, Three and 2 more\n1 stopped without an answer')
  })

  it('heads with the failures when nothing came back', () => {
    const said = sayEndings([ending('One', 'ended'), ending(null, 'ended')], t)
    expect(said.title).toBe('2 stopped without an answer')
    expect(said.body).toBe('One and 1 more')
  })

  it('speaks Russian with Russian plurals', async () => {
    await i18n.changeLanguage('ru')
    const said = sayEndings([ending('А', 'done'), ending('Б', 'done'), ending('В', 'done')], t)
    expect(said.title).toBe('Готовы 3 ответа')
    expect(said.body).toBe('А, Б, В')
  })
})
