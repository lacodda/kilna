import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Providers } from '@/app/Providers'
import { NewWorkDialog } from '@/components/NewWorkDialog'
import { createQueryClient } from '@/lib/query/client'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'

/*
 * The new work's dialog is named for the kind it makes, in words (FR-19
 * 19.4): a kind's label is a word per language, and handed to the sentence
 * as it was it read "New: [object Object]".
 */

describe('the dialog of a new work', () => {
  it('is named for the kind in words', async () => {
    const workspace = studio()
    mockBackend(answersFor(workspace))
    render(
      <Providers client={createQueryClient()}>
        <ProfileContext.Provider value={workspace.profile}>
          <NewWorkDialog kind="song" onClose={() => {}} onCreated={() => {}} />
        </ProfileContext.Provider>
      </Providers>,
    )

    const heading = await screen.findByRole('heading', { name: /^New: / })
    expect(heading).toHaveTextContent('New: Song')
    expect(heading.textContent).not.toContain('[object Object]')
  })
})
