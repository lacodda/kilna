import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { humanError } from '@/lib/errors'

describe('humanError', () => {
  it('answers a second click on a running task with "wait", not with a diagnosis', () => {
    // The refusal used to travel as `assistant`, and the window told the person
    // to check that Claude Code was installed - while it was busy answering.
    const said = humanError({ kind: 'alreadyRunning', message: 'this is already running' })
    expect(said).toBe(i18n.t('error.alreadyRunning'))
    expect(said).not.toBe(i18n.t('error.assistant'))
  })

  it('has its own words for every kind the backend refuses with', () => {
    for (const kind of ['busy', 'frozen', 'notRestorable', 'layoutStale']) {
      expect(humanError({ kind, message: 'backend words' })).toBe(i18n.t(`error.${kind}`))
    }
  })

  it('never says [object Object]', () => {
    expect(humanError({ other: true })).not.toContain('[object Object]')
    expect(humanError({ kind: 'other', message: 'a real sentence' })).toBe('a real sentence')
  })
})
