import { describe, expect, it } from 'vitest'
import { paragraphsOf } from '@/features/comments/NewCommentDialog'

/*
 * Comments typed in a batch (v0.162): each paragraph is one, a blank line
 * between them, however many spaces that line holds.
 */

describe('several comments typed at once', () => {
  it('splits the text at blank lines and drops what is empty', () => {
    expect(paragraphsOf('great song\n\n  \nwhat is the bridge about?\nreally\n\n\n')).toEqual([
      'great song',
      'what is the bridge about?\nreally',
    ])
    expect(paragraphsOf('   ')).toEqual([])
  })
})
