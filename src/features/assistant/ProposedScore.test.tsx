import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import type { ScoreProposal } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { ProposedScore } from '@/features/assistant/ProposedScore'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, IDS, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A proposed score that also reads what the work feels like (FR-01, FR-02):
 * the vibe is shown beside the marks, said to be kept only where the author
 * has said none.
 */

function mounted(proposal: ScoreProposal) {
  const workspace = studio()
  mockBackend(answersFor(workspace))
  render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <ProposedScore
          workId={IDS.song}
          messageId="m1"
          proposal={proposal}
          applied={null}
          dismissed={false}
        />
      </ProfileContext.Provider>
    </Providers>,
  )
}

describe('a proposed score', () => {
  it('says the vibe it read, and that it does not stand over the author', () => {
    mounted({
      kind: 'score',
      axes: { hook: 7 },
      note: 'the chorus carries it',
      vibe: { mood: 'melancholy', complexity: 'complex' },
    } as ScoreProposal)

    expect(screen.getAllByText('Melancholy · Complex').length).toBeGreaterThan(0)
    expect(screen.getByRole('img', { name: 'Melancholy · Complex' })).toBeInTheDocument()
    expect(screen.getByText(en.assistant.vibeIfNone)).toBeVisible()
  })

  it('names the words of a vibe the profile does not have (v0.169)', () => {
    mounted({
      kind: 'score',
      axes: { hook: 7 },
      vibe: { mood: 'melancholy' },
      vibe_dropped: ['sunny', 'baroque'],
    } as ScoreProposal)

    expect(screen.getByText(/left out: sunny, baroque/)).toBeVisible()
  })

  it('says nothing of a vibe when the answer read none', () => {
    mounted({ kind: 'score', axes: { hook: 7 } } as ScoreProposal)
    expect(screen.queryByText(en.assistant.vibeIfNone)).toBeNull()
  })
})
