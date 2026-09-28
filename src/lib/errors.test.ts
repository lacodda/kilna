import { describe, expect, it } from 'vitest'
import i18n from '@/i18n'
import { humanError, sayReason } from '@/lib/errors'

describe('humanError', () => {
  it('answers a second click on a running task with "wait", not with a diagnosis', () => {
    // The refusal used to travel as `assistant`, and the window told the person
    // to check that Claude Code was installed - while it was busy answering.
    const said = humanError({ kind: 'alreadyRunning', message: 'this is already running' })
    expect(said).toBe(i18n.t('error.alreadyRunning'))
    expect(said).not.toBe(i18n.t('error.assistant'))
  })

  it('has its own words for every kind the backend refuses with', () => {
    for (const kind of ['busy', 'frozen', 'notRestorable', 'layoutStale', 'internal']) {
      expect(humanError({ kind, message: 'backend words' })).toBe(i18n.t(`error.${kind}`))
    }
  })

  it('never says [object Object]', () => {
    expect(humanError({ other: true })).not.toContain('[object Object]')
    expect(humanError({ kind: 'other', message: 'a real sentence' })).toBe('a real sentence')
  })

  it('says a refusal from its code, in the language of the window', async () => {
    const refusal = {
      kind: 'refused',
      message: 'The profile has no status “nope”.',
      code: 'work.unknownStatus',
      params: { status: 'nope' },
    }
    await i18n.changeLanguage('ru')
    try {
      const said = humanError(refusal)
      expect(said).toBe(i18n.t('refusal.work.unknownStatus', { status: 'nope' }))
      expect(said).not.toBe(refusal.message)
      expect(said).toContain('nope')
    } finally {
      await i18n.changeLanguage('en')
    }
  })

  it('says every problem of a refused proposal, each in its own words', () => {
    const said = humanError({
      kind: 'refused',
      message: 'english',
      code: 'proposal.invalid',
      params: {
        problems: [
          { key: 'refusal.proposal.newWorkNeedsTitle', params: {} },
          { key: 'refusal.work.unknownStatus', params: { status: 'nope' } },
        ],
      },
    })
    expect(said).toContain(i18n.t('refusal.proposal.newWorkNeedsTitle'))
    expect(said).toContain(i18n.t('refusal.work.unknownStatus', { status: 'nope' }))
    expect(said).not.toContain('[object Object]')
  })

  it('keeps the backend English for a code this build has no sentence for', () => {
    const said = humanError({
      kind: 'refused',
      message: 'A sentence from a newer backend.',
      code: 'nowhere.atAll',
      params: {},
    })
    expect(said).toBe('A sentence from a newer backend.')
  })
})

describe('sayReason', () => {
  it('says why a batch passed an item over', () => {
    expect(sayReason({ key: 'skip.alreadyThere', params: {} })).toBe(i18n.t('skip.alreadyThere'))
    expect(sayReason({ key: 'error.notFound', params: {} })).toBe(i18n.t('error.notFound'))
  })
})
