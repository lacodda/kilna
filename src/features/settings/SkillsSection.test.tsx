import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { EntryStanding, ProfileConfig, PromptTemplate } from '@/lib/api/types'
import { actionsOfScope, isButton } from '@/lib/actions'
import { answers, copyKey, grouped, laidOver, NO_FILTER, skillSet } from '@/lib/skills'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * Settings → Skills (FR-17): every action, found by its words and narrowed,
 * edited whole as part of the profile's draft - a button or a method an
 * agent reads, reset to what ships, copied, previewed as the assistant would
 * be handed it.
 */

let backend: Backend
let workspace: Studio
let standing: EntryStanding[]

const critique: PromptTemplate = {
  key: 'critique',
  label: { en: 'Critique the lyrics', ru: 'Разбор текста' },
  category: 'critique',
  template: 'Critique {title}',
  method: 'Read slowly. Name what does not sing.',
  when: { en: 'After a draft, before polishing', ru: 'После черновика' },
}
const workflow: PromptTemplate = {
  key: 'song-workflow',
  label: 'Song workflow',
  category: 'workflow',
  template: '',
  show_in_ui: false,
  method: 'Go through the song in this order.',
}
const own: PromptTemplate = { key: 'mine', label: 'My own', template: 'Do {title}' }

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  workspace.profile.config.work_kinds = workspace.profile.config.work_kinds.slice(0, 1)
  workspace.profile.config.prompts = [critique, workflow, own]
  standing = [
    {
      key: 'critique',
      standing: 'changed',
      standard: { ...critique, template: 'Critique {title} as it stands' },
    },
    { key: 'song-workflow', standing: 'standard', standard: workflow },
  ]
  backend = mockBackend({
    ...answersFor(workspace),
    check_profile_config: () => [],
    shipped_standing: () => standing,
    shipped_offers: () => [],
    preview_skill: ({ action }) => ({
      method: (action as PromptTemplate).method ?? null,
      answer: 'End your reply with a fenced json block',
      placeholders: ['title'],
    }),
    update_profile_config: ({ config }) => {
      workspace.profile = { ...workspace.profile, config: config as ProfileConfig }
      return workspace.profile
    },
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function open() {
  const view = renderApp('/settings/skills')
  await settled(view.client)
  // The filter outlives the section, as it does in the window: each test
  // starts from the whole list.
  const search = await screen.findByRole('searchbox', { name: en.skills.search })
  fireEvent.change(search, { target: { value: '' } })
  fireEvent.click(screen.getByRole('radio', { name: en.skills.mode.all }))
  await screen.findByRole('heading', { name: en.skills.category.critique })
  return view
}

const row = (name: RegExp) => screen.getByRole('button', { name })

/** Open a skill's form, whichever way the last visit left it. */
function expand(name: RegExp) {
  const found = row(name)
  if (found.getAttribute('aria-expanded') !== 'true') fireEvent.click(found)
}

async function save() {
  const bar = await screen.findByRole('toolbar', { name: en.settings.unsaved })
  fireEvent.click(within(bar).getByRole('button', { name: en.editor.save }))
  await waitFor(() => expect(backend.argsOf('update_profile_config').length).toBeGreaterThan(0))
  const sent = backend.argsOf('update_profile_config').at(-1) as { config: ProfileConfig }
  return sent.config.prompts
}

describe('settings → skills', () => {
  it('lists every skill by its group, as a button or a method, and where it stands', async () => {
    await open()
    const critiqueRow = row(/Critique the lyrics/)
    expect(within(critiqueRow).getByText(en.skills.mode.button)).toBeVisible()
    expect(within(critiqueRow).getByText(en.skills.origin.changed)).toBeVisible()
    expect(within(critiqueRow).getByText('After a draft, before polishing')).toBeVisible()
    const workflowRow = row(/Song workflow/)
    expect(within(workflowRow).getByText(en.skills.mode.playbook)).toBeVisible()
    expect(within(row(/My own/)).getByText(en.skills.origin.own)).toBeVisible()
    expect(screen.getByRole('heading', { name: en.skills.category.workflow })).toBeVisible()
    expect(screen.getByRole('heading', { name: en.skills.category.none })).toBeVisible()
  })

  it('finds a skill by a word of its method, and narrows to the methods alone', async () => {
    await open()
    fireEvent.change(screen.getByRole('searchbox', { name: en.skills.search }), {
      target: { value: 'in this order' },
    })
    expect(row(/Song workflow/)).toBeVisible()
    expect(screen.queryByRole('button', { name: /Critique the lyrics/ })).toBeNull()

    fireEvent.change(screen.getByRole('searchbox', { name: en.skills.search }), {
      target: { value: '' },
    })
    fireEvent.click(screen.getByRole('radio', { name: en.skills.mode.playbook }))
    expect(row(/Song workflow/)).toBeVisible()
    expect(screen.queryByRole('button', { name: /My own/ })).toBeNull()
  })

  it('turns a button into a method an agent reads, keeping its message', async () => {
    await open()
    expand(/Critique the lyrics/)
    fireEvent.click(screen.getByRole('switch', { name: en.skills.showInUi }))
    expect(screen.getByText(en.skills.templateHidden)).toBeVisible()
    const prompts = await save()
    const saved = prompts.find((one) => one.key === 'critique')!
    expect(saved.show_in_ui).toBe(false)
    expect(saved.template).toBe('Critique {title}')
  })

  it('resets a changed skill to what ships, and copies one under a new key', async () => {
    await open()
    expand(/Critique the lyrics/)
    fireEvent.click(screen.getByRole('button', { name: en.skills.reset }))
    fireEvent.click(screen.getByRole('button', { name: en.skills.duplicate }))
    const prompts = await save()
    expect(prompts.map((one) => one.key)).toEqual([
      'critique',
      'critique-copy',
      'song-workflow',
      'mine',
    ])
    expect(prompts[0]!.template).toBe('Critique {title} as it stands')
  })

  it('shows what the assistant would be handed', async () => {
    await open()
    expand(/Song workflow/)
    fireEvent.click(screen.getByRole('button', { name: en.skills.preview }))
    const preview = await screen.findByRole('region', { name: en.skills.previewTitle })
    expect(within(preview).getByText('Go through the song in this order.')).toBeVisible()
    expect(within(preview).getByText('End your reply with a fenced json block')).toBeVisible()
    expect(backend.argsOf('preview_skill')[0]).toMatchObject({ kind: 'song' })
  })
})

describe('reading skills', () => {
  it('never offers a method alone or a hidden skill as a button', () => {
    expect(isButton(critique)).toBe(true)
    expect(isButton(workflow)).toBe(false)
    expect(isButton({ ...critique, show_in_ui: false })).toBe(false)
    expect(actionsOfScope([critique, workflow, own], 'work').map((one) => one.key)).toEqual([
      'critique',
      'mine',
    ])
  })

  it('filters by words in either language, kind, mode and origin', () => {
    expect(answers(critique, { ...NO_FILTER, query: 'разбор' }, 'changed')).toBe(true)
    expect(answers(critique, { ...NO_FILTER, query: 'polishing' }, 'changed')).toBe(true)
    expect(answers(critique, { ...NO_FILTER, origin: 'standard' }, 'changed')).toBe(false)
    expect(answers({ ...critique, kinds: ['video'] }, { ...NO_FILTER, kind: 'song' }, 'own')).toBe(
      false,
    )
    expect(answers(critique, { ...NO_FILTER, kind: 'song' }, 'own')).toBe(true)
    expect(answers(workflow, { ...NO_FILTER, mode: 'button' }, 'own')).toBe(false)
  })

  it('groups in order, copies under a free key, and lays a set over the profile', () => {
    expect(grouped([own, workflow, critique]).map((group) => group.category)).toEqual([
      'critique',
      'workflow',
      null,
    ])
    expect(copyKey([critique, { ...critique, key: 'critique-copy' }], 'critique')).toBe(
      'critique-copy-2',
    )
    const merged = laidOver([critique, own], [{ ...own, template: 'New' }, workflow])
    expect(merged.actions.map((one) => one.key)).toEqual(['critique', 'mine', 'song-workflow'])
    expect(merged.actions[1]!.template).toBe('New')
    expect([merged.replaced, merged.added]).toEqual([1, 1])
    expect(JSON.parse(skillSet([own]))).toEqual({ kilna: 'skills', version: 1, skills: [own] })
  })
})
