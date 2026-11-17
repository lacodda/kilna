import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import type { Work } from '@/lib/api/types'
import { Providers } from '@/app/Providers'
import { StageChecks } from '@/components/StageChecks'
import { StageDial } from '@/components/StageDial'
import { StageStops } from '@/components/StageStops'
import { StageWidget } from '@/features/work/tabs/overview/StageWidget'
import { createQueryClient } from '@/lib/query/client'
import { stagesOf } from '@/lib/stages'
import { ProfileContext } from '@/lib/useProfile'
import { mockBackend } from '@/test/backend'
import { answersFor, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The five stops (FR-03, v0.119): how a dial is drawn at each, and the row of
 * five that sets a work's stage and narrows a list by it - with no word and
 * no number beside a dial anywhere.
 */

const WORDS = [
  '0% — idea / untouched',
  '25% — started / first drafts',
  '50% — halfway / working draft',
  '75% — almost ready / polishing',
  '100% — final / ready to ship',
]

/**
 * One of an SVG shape's stroke attributes. Named in two halves: the token
 * check reads every quoted word that begins like a colour class as one.
 */
function stroke(shape: Element, part: string): string | null {
  return shape.getAttribute(['stroke', part].join('-'))
}

/** A colour written in hex, as the browser reports one it was given. */
function asRgb(hex: string): string {
  const probe = document.createElement('span')
  probe.style.color = hex
  return probe.style.color
}

function mounted(children: React.ReactNode) {
  const workspace = studio()
  const backend = mockBackend({
    ...answersFor(workspace),
    update_work: ({ id, patch }) => ({
      ...workspace.works.find((work) => work.id === id)!,
      ...(patch as Partial<Work>),
    }),
  })
  render(
    <Providers client={createQueryClient()}>
      <ProfileContext.Provider value={workspace.profile}>
        <MemoryRouter>{children}</MemoryRouter>
      </ProfileContext.Provider>
    </Providers>,
  )
  return { workspace, backend }
}

describe('a dial', () => {
  it('draws each of the five stops its own way, in its own colour', () => {
    const stops = stagesOf(studio().profile.config)
    expect(stops.map((stop) => stop.percent)).toEqual([0, 25, 50, 75, 100])

    render(
      <>
        {stops.map((stop) => (
          <StageDial key={stop.key} stage={stop} />
        ))}
      </>,
    )
    const dials = screen.getAllByRole('img')
    expect(dials.map((dial) => dial.getAttribute('aria-label'))).toEqual(WORDS)
    // What it says when it is pointed at is the same sentence.
    expect(dials.map((dial) => dial.querySelector('title')?.textContent)).toEqual(WORDS)
    // The exact colour each stop names, through `color` - every stroke is
    // `currentColor`. The five are the profile's, and all different.
    const named = stops.map((one) => one.hex)
    expect(named.every((hex) => hex !== undefined && /^#[0-9A-F]{6}$/.test(hex))).toBe(true)
    expect(new Set(named).size).toBe(5)
    expect(dials.map((dial) => dial.style.color)).toEqual(named.map((hex) => asRgb(hex!)))
    for (const dial of dials) {
      for (const circle of dial.querySelectorAll('circle')) {
        expect(circle.getAttribute('stroke')).toBe('currentColor')
        expect(circle.getAttribute('fill')).toBe('none')
      }
    }

    const circles = dials.map((dial) => [...dial.querySelectorAll('circle')])
    // At 0: an empty ring and nothing in it - no dot in the middle.
    expect(circles[0]).toHaveLength(1)
    expect(stroke(circles[0]![0]!, 'opacity')).toBe('1')
    // At 100: the whole ring, closed.
    expect(circles[4]).toHaveLength(1)
    expect(stroke(circles[4]![0]!, 'opacity')).toBe('1')
    // Between: an arc of a quarter, a half, three quarters over a dim track.
    const circumference = 2 * Math.PI * 5
    for (const [at, share] of [
      [1, 0.25],
      [2, 0.5],
      [3, 0.75],
    ] as const) {
      const [track, arc] = circles[at]!
      expect(stroke(track!, 'opacity')).toBe('0.22')
      expect(Number(stroke(arc!, 'dasharray')!.split(' ')[0])).toBeCloseTo(circumference * share)
      expect(stroke(arc!, 'linecap')).toBe('round')
    }
  })
})

describe('the row of five stops', () => {
  it('sets one stop: five radios named by their words, with nothing written beside them', () => {
    const onPick = vi.fn()
    mounted(<StageStops label="Stage" pick="one" on={[25]} onPick={onPick} />)

    const row = screen.getByRole('radiogroup', { name: 'Stage' })
    const radios = within(row).getAllByRole('radio')
    expect(radios.map((radio) => radio.getAttribute('aria-label'))).toEqual(WORDS)
    expect(radios.map((radio) => radio.getAttribute('aria-checked'))).toEqual([
      'false',
      'true',
      'false',
      'false',
      'false',
    ])
    // No word and no percentage stands in the row: what is read there is
    // only the dials' own titles, which are not drawn.
    for (const radio of radios) {
      const drawn = [...radio.childNodes].filter((node) => node.nodeType === Node.TEXT_NODE)
      expect(drawn).toHaveLength(0)
    }
    expect(within(row).queryByRole('button', { name: /clear|reset/i })).toBeNull()

    fireEvent.click(radios[3]!)
    expect(onPick).toHaveBeenLastCalledWith(75)

    // The arrows move along the scale, Home and End go to its ends.
    fireEvent.keyDown(row, { key: 'ArrowRight' })
    expect(onPick).toHaveBeenLastCalledWith(50)
    fireEvent.keyDown(row, { key: 'ArrowLeft' })
    expect(onPick).toHaveBeenLastCalledWith(0)
    fireEvent.keyDown(row, { key: 'End' })
    expect(onPick).toHaveBeenLastCalledWith(100)
    fireEvent.keyDown(row, { key: 'Home' })
    expect(onPick).toHaveBeenLastCalledWith(0)
  })

  it('ticks any of them in a filter, reading an older tick as the stop it lands on', () => {
    const onChange = vi.fn()
    // -1 was "no stage", 83 a stop of the seven: the first stop and 75.
    mounted(<StageChecks chosen={[-1, 83]} onChange={onChange} />)

    const row = screen.getByRole('group', { name: en.stage.label })
    const dials = within(row).getAllByRole('button')
    expect(dials).toHaveLength(5)
    expect(dials.map((dial) => dial.getAttribute('aria-pressed'))).toEqual([
      'true',
      'false',
      'false',
      'true',
      'false',
    ])

    fireEvent.click(dials[2]!)
    expect(onChange).toHaveBeenLastCalledWith([0, 50, 75])
    fireEvent.click(dials[0]!)
    expect(onChange).toHaveBeenLastCalledWith([75])
  })
})

describe("the overview's stage widget", () => {
  it('shows the five dials and writes the one pressed', async () => {
    const workspace = studio()
    const song = workspace.works[0]!
    const { backend } = mounted(<StageWidget work={{ ...song, stage: 50 }} />)

    const row = await screen.findByRole('radiogroup', { name: en.stage.label })
    expect(within(row).getByRole('radio', { checked: true })).toHaveAccessibleName(WORDS[2])
    // Neither the stop's word nor its number is written out: with the
    // dials' own titles taken away - said on hover, not drawn - the row
    // holds no text at all.
    const drawn = row.cloneNode(true) as HTMLElement
    for (const title of drawn.querySelectorAll('title')) title.remove()
    expect(drawn.textContent).toBe('')

    fireEvent.click(within(row).getByRole('radio', { name: WORDS[4] }))
    await waitFor(() =>
      expect(backend.argsOf('update_work')).toEqual([{ id: song.id, patch: { stage: 100 } }]),
    )
  })
})
