import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { Trial, TrialBoard, TrialCard } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The board of trials on an experiment's card (v0.95, ADR 0061): what the
 * window asks the backend for when a person sweeps the field, judges a
 * trial, varies it, or takes a kept one into a song. What lands on the board
 * is the backend's; these hold the window to what it sends.
 */

const LAB = IDS.lab

let backend: Backend
let workspace: Studio

function trial(id: string, over: Partial<Trial> = {}): Trial {
  return {
    id,
    profile_id: IDS.profile,
    work_id: LAB,
    series: 'A sweep of the field',
    position: 1,
    parent_id: null,
    angle: `angle ${id}`,
    body: `amen break, fuzz bass, ${id}`,
    bricks: [],
    reference: '',
    outcome: '',
    verdict: null,
    source_version_id: null,
    run_first: false,
    created_at: '2026-10-01T10:00:00Z',
    updated_at: '2026-10-01T10:00:00Z',
    ...over,
  }
}

function card(one: Trial, over: Partial<TrialCard> = {}): TrialCard {
  return { trial: one, takes: [], harvest: [], lost_anchors: [], source: null, ...over }
}

function board(cards: TrialCard[], over: Partial<TrialBoard> = {}): TrialBoard {
  return {
    work_id: LAB,
    anchors: ['fuzz bass'],
    harvest_role: 'style',
    harvest_kinds: ['song'],
    composition: 'sound',
    series: [...new Set(cards.map((one) => one.trial.series))],
    trials: cards,
    ...over,
  }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  workspace = studio()
  backend = mockBackend({
    ...answersFor(workspace),
    start_lab_task: ({ id, action }) => ({
      chatId: 'chat-trials',
      runId: 'run-trials',
      taskKey: `${action as string}:lab:${id as string}:-:vary:A sweep of the field`,
      title: 'Propose trials',
    }),
    judge_trial: ({ id, verdict }) => ({ ...trial(id as string), verdict }),
    vary_trial: ({ id, angle }) =>
      trial('child', { parent_id: id as string, angle: angle as string }),
    harvest_trial: ({ id, workId }) => ({
      id: 'v-harvest',
      work_id: workId,
      role: 'style',
      revision: 2,
      label: null,
      body: 'amen break',
      meta: {},
      parent_version_id: null,
      trial_id: id,
      created_at: NOW,
    }),
    update_trial: ({ id }) => trial(id as string),
  })
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

async function openBoard() {
  const { client } = renderApp(`/works/${LAB}/trials`)
  await screen.findByRole('button', { name: new RegExp(en.trials.new) })
  await settled(client)
}

describe('the board of trials', () => {
  it('sweeps the field from an empty board', async () => {
    await openBoard()
    expect(screen.getByText(en.trials.emptyBoard)).toBeInTheDocument()

    fireEvent.click(screen.getAllByRole('button', { name: en.trials.sweepGo })[0]!)
    await waitFor(() => expect(backend.argsOf('start_lab_task')).toHaveLength(1))
    expect(backend.argsOf('start_lab_task')[0]).toEqual({
      id: LAB,
      action: 'propose-trials',
      request: { count: 6, series: en.trials.series.sweep, around: null, fix: false },
    })
  })

  it('lists the series as a tree, and judges the open trial', async () => {
    backend.answer('trial_board', () =>
      board([
        card(trial('core', { position: 1 })),
        card(trial('slower', { position: 2, parent_id: 'core' }), {
          lost_anchors: ['fuzz bass'],
        }),
      ]),
    )
    await openBoard()

    const list = screen.getByRole('region', { name: 'A sweep of the field' })
    expect(within(list).getByRole('button', { name: /angle core/ })).toBeInTheDocument()
    fireEvent.click(within(list).getByRole('button', { name: /angle slower/ }))

    // The open one has lost an anchor, and says which.
    expect(await screen.findByText(en.trials.lost)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('radio', { name: en.trials.verdict.keep }))
    await waitFor(() =>
      expect(backend.argsOf('judge_trial')).toEqual([{ id: 'slower', verdict: 'keep' }]),
    )
  })

  it('varies the open trial with what is to move', async () => {
    backend.answer('trial_board', () => board([card(trial('core'))]))
    await openBoard()

    fireEvent.click(screen.getByRole('button', { name: en.trials.more }))
    fireEvent.click(await screen.findByRole('menuitem', { name: en.trials.vary }))
    const dialog = await screen.findByRole('dialog', { name: en.trials.varyTitle })
    fireEvent.change(within(dialog).getByRole('textbox', { name: en.trials.angle }), {
      target: { value: 'denser' },
    })
    fireEvent.click(within(dialog).getByRole('button', { name: en.trials.vary }))

    await waitFor(() =>
      expect(backend.argsOf('vary_trial')).toEqual([{ id: 'core', angle: 'denser', series: null }]),
    )
  })

  it('takes a kept trial into a song, beside its current style', async () => {
    backend.answer('trial_board', () => board([card(trial('kept', { verdict: 'keep' }))]))
    await openBoard()

    fireEvent.click(await screen.findByRole('button', { name: en.trials.intoWork }))
    const picker = await screen.findByRole('dialog')
    fireEvent.click(await within(picker).findByText('Paper Lanterns'))

    await waitFor(() =>
      expect(backend.argsOf('harvest_trial')).toEqual([
        { id: 'kept', workId: IDS.song, label: null, makeCurrent: false },
      ]),
    )
  })

  it('offers no harvest for a trial not kept', async () => {
    backend.answer('trial_board', () => board([card(trial('open'))]))
    await openBoard()
    expect(screen.queryByRole('button', { name: en.trials.intoWork })).not.toBeInTheDocument()
  })
})
