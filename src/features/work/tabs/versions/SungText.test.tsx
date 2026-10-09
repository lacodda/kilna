import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import type { NewVersion, StressNote, TextCheck, Version } from '@/lib/api/types'
import { mockBackend, type Backend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio, type Studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * A sung text in the Versions tab (ADR 0053).
 *
 * The lyric's role is sung, so the backend is asked about its stresses and
 * the window answers with them: a strip of the words the singer may get
 * wrong, each with its answers, written over exactly that word; the gesture
 * that puts a stress on a vowel; and a copy without the singer's marks. The
 * check is the backend's, so here it is a stand-in that finds a homograph
 * and a missing ё wherever they are in the text it is sent.
 */

const LYRIC = '[Verse]\nСтарый замок у воды\nеще горит'

/** What the backend would say of `text`: "замок" reads two ways, "еще"
 *  is written without its ё. Offsets follow the text, as the real check's do. */
function checkOf(text: string, sung: boolean): TextCheck {
  const empty: TextCheck = {
    repeats: [],
    terms: [],
    marks: [],
    stress: [],
    accents: [],
    phrases: [],
    unknown: [],
  }
  if (!sung) return empty
  const stress: StressNote[] = []
  const castle = text.indexOf('замок')
  if (castle >= 0) {
    stress.push({
      kind: 'homograph',
      word: 'замок',
      start: castle,
      end: castle + 5,
      options: ['зАмок', 'замОк'],
    })
  }
  const yet = text.indexOf('еще')
  if (yet >= 0) {
    stress.push({ kind: 'yo', word: 'еще', start: yet, end: yet + 3, options: ['ещё'] })
  }
  return {
    ...empty,
    stress,
    marks: stress.map((note, index) => ({ start: note.start, end: note.end, stress: index })),
  }
}

let place: Studio
let backend: Backend

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  place = studio()
  place.versions.find((version) => version.id === IDS.lyrics)!.body = LYRIC
  backend = mockBackend(answersFor(place))
  backend.answer('check_text', ({ text, sung }) => checkOf(text as string, sung === true))
  // A change to a read version mints the next revision, the way the backend
  // does: into the studio, so the panel can open it.
  backend.answer('create_version', ({ workId, version }) => {
    const asked = version as NewVersion
    const made: Version = {
      trial_id: null,
      id: `v-made-${place.versions.length}`,
      work_id: workId as string,
      role: asked.role,
      revision: 3,
      label: null,
      body: asked.body,
      meta: {},
      parent_version_id: asked.parent_version_id ?? null,
      created_at: NOW,
    }
    place.versions.push(made)
    return made
  })
})

afterEach(() => {
  vi.useRealTimers()
})

/** The song's Versions tab with its lyric open and read. */
async function openLyric() {
  const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.lyrics}`)
  await settled(client)
  const main = await screen.findByRole('main')
  await within(main).findByText('[Verse]')
  return main
}

/** The body of every revision the panel minted, in order. */
const minted = () =>
  backend.argsOf('create_version').map((args) => (args.version as NewVersion).body)

describe('a sung text', () => {
  it('is checked as sung, and marks the words the singer may get wrong', async () => {
    const main = await openLyric()
    await waitFor(() =>
      expect(backend.argsOf('check_text')).toContainEqual(
        expect.objectContaining({ text: LYRIC, sung: true }),
      ),
    )
    // A missing ё is marked always; a homograph - ten a song, nearly all
    // read right - only while the stresses are shown.
    expect(await within(main).findByText('еще', { selector: 'mark' })).toHaveClass('stress-yo')
    expect(within(main).queryByText('замок', { selector: 'mark' })).toBeNull()

    fireEvent.click(within(main).getByRole('button', { name: en.versions.stress.show }))
    expect(await within(main).findByText('замок', { selector: 'mark' })).toHaveClass(
      'stress-homograph',
    )
  })

  it('folds the homographs into one chip, after the rest', async () => {
    const main = await openLyric()
    const group = await within(main).findByRole('button', {
      name: en.versions.stress.homographs.replace('{{count}}', '1'),
    })
    expect(group).toHaveAttribute('aria-expanded', 'false')
    expect(within(main).getByRole('button', { name: /еще/ })).toBeInTheDocument()
    expect(within(main).queryByRole('button', { name: /замок/ })).toBeNull()

    fireEvent.click(group)
    expect(group).toHaveAttribute('aria-expanded', 'true')
    expect(within(main).getByRole('button', { name: /замок/ })).toBeInTheDocument()
  })

  it('lists a homograph, and the reading chosen is written over that word', async () => {
    const main = await openLyric()
    fireEvent.click(
      await within(main).findByRole('button', {
        name: en.versions.stress.homographs.replace('{{count}}', '1'),
      }),
    )
    const chip = await within(main).findByRole('button', { name: /замок/ })
    fireEvent.click(chip)
    const menu = await screen.findByRole('menu')
    // Both readings, and a way to the word.
    expect(within(menu).getByRole('menuitem', { name: 'зАмок' })).toBeInTheDocument()
    fireEvent.click(within(menu).getByRole('menuitem', { name: 'замОк' }))

    // Read, it is saved the way the editor saves: the next revision, with
    // exactly that word changed and nothing else.
    await waitFor(() => expect(minted()).toEqual([LYRIC.replace('замок', 'замОк')]))
    expect(within(main).queryByRole('textbox', { name: en.versions.edit })).toBeNull()
  })

  it('applies every answer with nothing to choose, and leaves the homograph', async () => {
    const main = await openLyric()
    fireEvent.click(
      await within(main).findByRole('button', {
        name: en.versions.stress.applyAll.replace('{{count}}', '1'),
      }),
    )
    await waitFor(() => expect(minted()).toEqual([LYRIC.replace('еще', 'ещё')]))
  })

  it('puts the stress on the vowel Alt+clicked, and keeps the caret there', async () => {
    const main = await openLyric()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.edit }))
    const box = await screen.findByRole<HTMLTextAreaElement>('textbox', {
      name: en.versions.edit,
    })

    // The click put the caret right after the "о" of "замок".
    const caret = LYRIC.indexOf('замок') + 4
    box.setSelectionRange(caret, caret)
    fireEvent.click(box, { altKey: true })
    expect(box).toHaveValue(LYRIC.replace('замок', 'замОк'))
    expect(box.selectionStart).toBe(caret)

    // Once more on the same vowel takes the mark off.
    fireEvent.click(box, { altKey: true })
    expect(box).toHaveValue(LYRIC)

    // A click without Alt is a click.
    fireEvent.click(box)
    expect(box).toHaveValue(LYRIC)
  })

  it("puts the stress on the vowel just typed with Alt+'", async () => {
    const main = await openLyric()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.edit }))
    const box = await screen.findByRole<HTMLTextAreaElement>('textbox', {
      name: en.versions.edit,
    })

    // After "го" of "горит": the "о" just typed is the one marked. On a
    // Cyrillic layout the key reads "э"; its place is what counts.
    const caret = LYRIC.indexOf('горит') + 2
    box.setSelectionRange(caret, caret)
    fireEvent.keyDown(box, { key: 'э', code: 'Quote', altKey: true })
    expect(box).toHaveValue(LYRIC.replace('горит', 'гОрит'))
  })

  it("copies the text without the singer's marks, as the backend cleans it", async () => {
    const writeText = vi.fn(() => Promise.resolve())
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    backend.answer('clean_text', () => 'cleaned')

    const main = await openLyric()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.stress.copyClean }))

    await waitFor(() => expect(writeText).toHaveBeenCalledWith('cleaned'))
    expect(backend.argsOf('clean_text')).toEqual([{ text: LYRIC }])
    expect(await screen.findByText(en.versions.stress.cleanCopied)).toBeInTheDocument()
  })

  it('is none of this for a role that is not sung', async () => {
    const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.style}`)
    await settled(client)
    const main = await screen.findByRole('main')
    await within(main).findByText('warm synth pop, slow build, airy female vocal')

    expect(backend.argsOf('check_text').every((args) => args.sung === false)).toBe(true)
    expect(within(main).queryByRole('button', { name: en.versions.stress.copyClean })).toBeNull()
    expect(within(main).queryByRole('button', { name: en.versions.stress.show })).toBeNull()
  })
})
