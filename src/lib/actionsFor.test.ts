import { describe, expect, it } from 'vitest'
import { actionsFor } from '@/components/assistant/ActionBar'
import type { PromptTemplate } from '@/lib/api'

function action(over: Partial<PromptTemplate>): PromptTemplate {
  return { key: 'k', label: 'Label', template: 'Do it.', ...over }
}

describe('actionsFor', () => {
  it('offers an action with no kinds on every kind', () => {
    const actions = [action({ key: 'score' })]

    expect(actionsFor(actions, 'video', 'work').map((a) => a.key)).toEqual(['score'])
    expect(actionsFor(actions, 'song', 'work').map((a) => a.key)).toEqual(['score'])
  })

  it('keeps an action off a kind it does not name', () => {
    const actions = [action({ key: 'plot', kinds: ['video'] })]

    expect(actionsFor(actions, 'song', 'work')).toEqual([])
  })

  it('sorts scene actions and work actions into their own bars', () => {
    const actions = [action({ key: 'plot' }), action({ key: 'prompts', scope: 'scene' })]

    expect(actionsFor(actions, 'video', 'work').map((a) => a.key)).toEqual(['plot'])
    expect(actionsFor(actions, 'video', 'scene').map((a) => a.key)).toEqual(['prompts'])
  })

  /* A style action is about a brick of the dictionary, not about a work. Left
     to the `scene ? scene : work` fallback it landed on every card, where it
     would have sent a prompt about nothing. */
  it('keeps a style action off both bars', () => {
    const actions = [action({ key: 'describe-style', scope: 'style' })]

    expect(actionsFor(actions, 'video', 'work')).toEqual([])
    expect(actionsFor(actions, 'video', 'scene')).toEqual([])
  })

  it('offers nothing when the kind is unknown', () => {
    expect(actionsFor([action({ key: 'score' })], undefined, 'work')).toEqual([])
  })
})
