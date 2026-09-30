import type { PromptTemplate } from '@/lib/api/types'

/** What a profile action is about, as the backend reads `scope`. */
export type ActionScope = 'work' | 'scene' | 'style' | 'comment' | 'canon' | 'selection'

/**
 * The scope of an action. Anything the backend reads as a work action — no
 * scope, `work`, or a word this build does not know — is `work` here too,
 * so the two ends cannot disagree about where a button belongs.
 */
export function scopeOf(action: Pick<PromptTemplate, 'scope'>): ActionScope {
  switch (action.scope?.trim()) {
    case 'scene':
      return 'scene'
    case 'style':
      return 'style'
    case 'comment':
      return 'comment'
    case 'canon':
      return 'canon'
    case 'selection':
      return 'selection'
    default:
      return 'work'
  }
}

/** The actions of one scope, in the profile's order. Every list of buttons
 *  asks through here, so an action about a style or a comment never turns up
 *  on a work's card — a button there would send a prompt about nothing. */
export function actionsOfScope(actions: PromptTemplate[], scope: ActionScope): PromptTemplate[] {
  return actions.filter((action) => scopeOf(action) === scope)
}

/** The action about a comment that produces this, if the profile has one. */
export function commentAction(
  actions: PromptTemplate[],
  produces: 'comment' | 'reply',
): PromptTemplate | undefined {
  return actionsOfScope(actions, 'comment').find((action) => action.produces?.trim() === produces)
}

/** The action about a card of the canon that produces this, if the profile
 *  has one: `canon` gathers the card's facts, `card-prompt` describes it. */
export function cardAction(
  actions: PromptTemplate[],
  produces: 'canon' | 'card-prompt',
): PromptTemplate | undefined {
  return actionsOfScope(actions, 'canon').find((action) => action.produces?.trim() === produces)
}

/** The action that reads selected lines into the canon, if the profile has one. */
export function selectionAction(actions: PromptTemplate[]): PromptTemplate | undefined {
  return actionsOfScope(actions, 'selection').find((action) => action.produces?.trim() === 'canon')
}

/** A work action that gathers the canon from the work's text, if the profile has one. */
export function gatherAction(actions: PromptTemplate[]): PromptTemplate | undefined {
  return actionsOfScope(actions, 'work').find((action) => action.produces?.trim() === 'canon')
}
