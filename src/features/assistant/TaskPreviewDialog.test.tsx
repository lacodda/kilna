import { describe, expect, it } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import type { PromptTemplate } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'
import { createQueryClient } from '@/lib/query/client'
import { mockBackend } from '@/test/backend'
import en from '@/i18n/locales/en.json'

/*
 * What a run may read beyond its prompt, said before it starts (v0.108): the
 * same paths its brief will name, and none for a run that reads the web.
 */

const ACTION = { key: 'critique', label: 'Critique', template: '{body}' } as PromptTemplate

function mount(action: PromptTemplate, sight: string[]) {
  const backend = mockBackend({
    preview_task: () => ({ prompt: 'The prompt.', key: 'critique:w1', title: 'Critique' }),
    run_sight: () => sight,
  })
  render(
    <Providers client={createQueryClient()}>
      <TaskPreviewDialog
        open
        onOpenChange={() => {}}
        target={{ on: 'work', workId: 'w1' }}
        action={action}
        onStarted={() => {}}
      />
    </Providers>,
  )
  return backend
}

describe('the preview of a task', () => {
  it('names what the run may also read, when the setting opens anything', async () => {
    mount(ACTION, ['C:/media/songs/tide', 'C:/workspace/media/a.png'])

    expect(await screen.findByText('C:/media/songs/tide')).toBeVisible()
    expect(screen.getByText('C:/workspace/media/a.png')).toBeVisible()
    expect(screen.getByText(en.assistant.sightTitle)).toBeVisible()
  })

  it('says nothing of it when the setting opens nothing', async () => {
    const backend = mount(ACTION, [])

    expect(await screen.findByText('The prompt.')).toBeVisible()
    await waitFor(() => expect(backend.argsOf('run_sight')).toHaveLength(1))
    expect(screen.queryByText(en.assistant.sightTitle)).toBeNull()
  })

  it('does not ask for a run that reads the web: it has no file tools', async () => {
    const backend = mount({ ...ACTION, web: true }, ['C:/media/songs/tide'])

    expect(await screen.findByText('The prompt.')).toBeVisible()
    expect(backend.argsOf('run_sight')).toEqual([])
    expect(screen.queryByText(en.assistant.sightTitle)).toBeNull()
  })
})
