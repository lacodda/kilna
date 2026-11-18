import { describe, expect, it } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import type { TriggerFile } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { SkillTrigger } from '@/features/settings/SkillTrigger'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import en from '@/i18n/locales/en.json'

/*
 * The thin trigger an agent is given (FR-17): shown whole, installed in the
 * agent's own place, and never written over a file there without a yes.
 */

function files(exists: boolean): TriggerFile[] {
  return [
    {
      client: 'claude-code',
      text: '---\nname: kilna\n---\n\n1. Call `workspace`.',
      path: 'C:/Users/someone/.claude/skills/kilna/SKILL.md',
      exists,
      file_name: 'SKILL.md',
    },
    {
      client: 'cursor',
      text: '---\ndescription: kilna\n---',
      path: null,
      exists: false,
      file_name: 'kilna.mdc',
    },
  ]
}

function mounted(exists: boolean) {
  const backend = mockBackend({
    skill_triggers: () => files(exists),
    install_skill_trigger: () => 'C:/Users/someone/.claude/skills/kilna/SKILL.md',
  })
  render(
    <Providers client={createQueryClient()}>
      <SkillTrigger />
    </Providers>,
  )
  return backend
}

describe('a trigger for an agent', () => {
  it('is shown whole and installed in its place', async () => {
    const backend = mounted(false)
    const text = await screen.findByLabelText(en.agents.trigger.text)
    expect(text).toHaveTextContent('Call `workspace`.')
    expect(screen.getByText(/Installed at C:\/Users\/someone/)).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: en.agents.trigger.install }))
    await waitFor(() =>
      expect(backend.argsOf('install_skill_trigger')).toEqual([
        { client: 'claude-code', path: null, replace: false },
      ]),
    )
  })

  it('asks before writing over a file already there', async () => {
    const backend = mounted(true)
    fireEvent.click(await screen.findByRole('button', { name: en.agents.trigger.install }))
    const asked = await screen.findByRole('alertdialog')
    expect(backend.argsOf('install_skill_trigger')).toEqual([])
    fireEvent.click(within(asked).getByRole('button', { name: en.agents.trigger.replace }))
    await waitFor(() =>
      expect(backend.argsOf('install_skill_trigger')).toEqual([
        { client: 'claude-code', path: null, replace: true },
      ]),
    )
  })

  it('says where a project-read agent keeps it', async () => {
    mounted(false)
    fireEvent.click(await screen.findByRole('radio', { name: 'Cursor' }))
    expect(screen.getByText(/\.cursor\/rules\/kilna\.mdc/)).toBeVisible()
    expect(screen.getByRole('button', { name: en.agents.trigger.saveAs })).toBeVisible()
  })
})
