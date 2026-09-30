import { describe, expect, it } from 'vitest'
import type { PromptTemplate } from '@/lib/api/types'
import {
  actionsOfScope,
  cardAction,
  commentAction,
  gatherAction,
  scopeOf,
  selectionAction,
} from '@/lib/actions'

const action = (key: string, over: Partial<PromptTemplate> = {}): PromptTemplate => ({
  key,
  label: key,
  template: 'x',
  ...over,
})

describe('actions by scope', () => {
  const all = [
    action('critique'),
    action('scene-prompts', { scope: 'scene' }),
    action('describe-style', { scope: 'style' }),
    action('read-comment', { scope: 'comment', produces: 'comment' }),
    action('reply-to-comment', { scope: 'comment', produces: 'reply' }),
    action('later', { scope: 'something-new' }),
  ]

  it('reads an unknown scope as a work action, the way the backend does', () => {
    expect(scopeOf(action('x', { scope: 'something-new' }))).toBe('work')
    expect(scopeOf(action('x'))).toBe('work')
  })

  it('keeps comment and style actions off a work', () => {
    expect(actionsOfScope(all, 'work').map((one) => one.key)).toEqual(['critique', 'later'])
  })

  it('keeps actions about a card or a selection off a work, and finds them by what they produce', () => {
    const canon = [
      action('gather-canon', { produces: 'canon' }),
      action('to-canon', { scope: 'selection', produces: 'canon' }),
      action('gather-card', { scope: 'canon', produces: 'canon' }),
      action('describe-card', { scope: 'canon', produces: 'card-prompt' }),
    ]
    expect(actionsOfScope(canon, 'work').map((one) => one.key)).toEqual(['gather-canon'])
    expect(selectionAction(canon)?.key).toBe('to-canon')
    expect(gatherAction(canon)?.key).toBe('gather-canon')
    expect(cardAction(canon, 'canon')?.key).toBe('gather-card')
    expect(cardAction(canon, 'card-prompt')?.key).toBe('describe-card')
  })

  it('finds the comment action by what it produces', () => {
    expect(commentAction(all, 'reply')?.key).toBe('reply-to-comment')
    expect(commentAction(all, 'comment')?.key).toBe('read-comment')
    expect(commentAction([action('critique')], 'reply')).toBeUndefined()
  })
})
