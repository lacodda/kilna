import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, screen, within } from '@testing-library/react'
import { mockBackend } from '@/test/backend'
import { renderApp, settled } from '@/test/render'
import { answersFor, IDS, NOW, studio } from '@/test/workspace'
import en from '@/i18n/locales/en.json'

/*
 * The Versions tab reads first and writes when asked.
 *
 * Until v0.80 the whole text was a button into a full-screen editor, so a
 * press meant to select a line threw the text into a box instead; the new
 * version's form was appended under the text and turned the column into a
 * page; and the role chips left out the roles written about the text. These
 * hold what replaced that: the text is read in place and can be selected,
 * writing starts from the pencil or a key, the form takes the text's place,
 * and every role is a chip with its count - zero included.
 */

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(NOW))
  mockBackend(answersFor(studio()))
})

afterEach(() => {
  vi.useRealTimers()
})

/** The song's Versions tab, with its second lyric open and read - by a
 *  link, the way a score row opens the draft it judged. */
async function openVersions() {
  const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.lyrics}`)
  await settled(client)
  const main = await screen.findByRole('main')
  // The section header of the open revision, as the reading text draws it.
  const header = await within(main).findByText('[Chorus]')
  return { main, header }
}

const editBox = () => screen.queryByRole('textbox', { name: en.versions.edit })

describe('the Versions tab', () => {
  it('shows every role as a chip with its count, the empty ones too', async () => {
    const { main } = await openVersions()
    const lanes = within(main).getByRole('group', { name: en.versions.lanes })
    const chip = (name: RegExp) => within(lanes).getByRole('button', { name })

    expect(chip(/^Lyrics/)).toHaveTextContent('2')
    expect(chip(/^Lyrics/)).toHaveAttribute('aria-pressed', 'true')
    expect(chip(/^Style prompt/)).toHaveTextContent('1')
    // Commentary is a lane of its own now, and says it holds nothing yet.
    expect(chip(/^Review/)).toHaveTextContent('0')
    expect(chip(/^Critique/)).toHaveTextContent('0')
  })

  it('draws the section headers of the text in the accent', async () => {
    const { header } = await openVersions()
    expect(header).toHaveClass('text-accent-2')
    expect(screen.getByText('Paper, paper, carry the light')).not.toHaveClass('text-accent-2')
  })

  it('stays reading when the text is clicked, so a line can be selected', async () => {
    const { header } = await openVersions()
    fireEvent.mouseDown(header)
    fireEvent.click(header)
    expect(editBox()).toBeNull()
    expect(header.closest('.selectable'), 'a text that cannot be copied').not.toBeNull()
  })

  it('writes when the pencil is pressed, and reads again on Escape', async () => {
    const { main } = await openVersions()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.edit }))

    const box = await screen.findByRole('textbox', { name: en.versions.edit })
    expect(box).toHaveValue(
      '[Verse]\nLanterns on the water\nfloating out of reach\n\n[Chorus]\nPaper, paper, carry the light',
    )

    fireEvent.keyDown(box, { key: 'Escape', code: 'Escape' })
    expect(editBox()).toBeNull()
    expect(screen.getByText('[Chorus]')).toBeInTheDocument()
  })

  it('writes when E is pressed on the text, on any keyboard layout', async () => {
    const { header } = await openVersions()
    // The key's place, not its letter: on a Cyrillic layout it types `у`.
    fireEvent.keyDown(header, { key: 'у', code: 'KeyE' })
    expect(await screen.findByRole('textbox', { name: en.versions.edit })).toBeInTheDocument()
  })

  it('puts the text over the whole window, and back on Escape', async () => {
    const { main } = await openVersions()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.expand }))

    // On the stage, outside the card, and only there.
    const staged = await screen.findByText('[Chorus]')
    expect(main).not.toContainElement(staged)
    expect(screen.getByText(en.versions.stageHint)).toBeInTheDocument()

    fireEvent.keyDown(window, { key: 'Escape', code: 'Escape' })
    expect(screen.queryByText(en.versions.stageHint)).toBeNull()
    expect(within(main).getByText('[Chorus]')).toBeInTheDocument()
  })

  it('puts the form for a new version in the place of the text, and back', async () => {
    const { main } = await openVersions()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.new }))

    const form = await within(main).findByRole('textbox', { name: en.versions.draftPlaceholder })
    expect(form).toBeInTheDocument()
    // Not under the text: the text is not on screen while the form is.
    expect(within(main).queryByText('Paper, paper, carry the light')).toBeNull()

    fireEvent.click(within(main).getByRole('button', { name: en.versions.cancel }))
    expect(await within(main).findByText('[Chorus]')).toBeInTheDocument()
    expect(within(main).queryByRole('textbox', { name: en.versions.draftPlaceholder })).toBeNull()
  })

  it('starts a version from nothing at the skeleton its role ships, not as a kept draft', async () => {
    const { main } = await openVersions()
    fireEvent.click(within(main).getByRole('button', { name: en.versions.new }))

    const form = await within(main).findByRole('textbox', { name: en.versions.draftPlaceholder })
    expect((form as HTMLTextAreaElement).value.startsWith('[Intro]\n\n[Verse 1]')).toBe(true)
    expect(within(main).queryByText(en.versions.kept)).toBeNull()
  })

  it('draws the tree beside the list where a version was written from another', async () => {
    const { main } = await openVersions()
    const list = within(main).getByRole('listbox', { name: en.versions.title })
    // The second lyric was written from the first: a dot per row, joined.
    const cells = list.querySelectorAll('[data-graph-column]')
    expect(cells).toHaveLength(2)
    expect(cells[0]).toHaveAttribute('title', 'Written from v1')
  })

  it('steps to the version the open one was written from, and back to it', async () => {
    const { main } = await openVersions()
    const open = within(main).getByRole('option', { name: /Second pass/ })
    open.focus()
    fireEvent.keyDown(open, { key: 'ArrowLeft' })
    // The first lyric has no part headers.
    expect(
      await within(main).findByRole('option', { name: /^v1/, selected: true }),
    ).toBeInTheDocument()
    expect(within(main).queryByText('[Chorus]')).toBeNull()

    fireEvent.keyDown(within(main).getByRole('option', { name: /^v1/ }), {
      key: 'ArrowRight',
    })
    expect(await within(main).findByText('[Chorus]')).toBeInTheDocument()
  })

  it('says the words, the lines and how far a row moved from the one it came from', async () => {
    const { main } = await openVersions()
    const row = within(main).getByRole('option', { name: /Second pass/ })
    // Headers are not words; four lines came in since the first lyric.
    expect(await within(row).findByText(/13 words · 3 lines/)).toBeInTheDocument()
    expect(within(row).getByText('+4')).toBeInTheDocument()
    expect(within(row).getByText('−0')).toBeInTheDocument()
  })

  it('marks what changed since the version it came from, and puts that one beside it on a press', async () => {
    const { main, header } = await openVersions()
    // The chorus came in with the second pass: a bar in its margin.
    expect(header.closest('[data-line]')?.className).toContain('before:bg-good')
    expect(
      screen.getByText('Lanterns on the water').closest('[data-line]')?.className ?? '',
    ).not.toContain('before:bg-good')

    const since = within(main).getByRole('button', { name: /since v1/ })
    fireEvent.click(since)
    // The first lyric stands beside the text, and the margin steps aside
    // for the comparison's own marks.
    expect(
      await within(main).findAllByRole('button', { name: en.versions.stopComparing }),
    ).not.toHaveLength(0)
    expect(within(main).queryByRole('button', { name: /since v1/ })).toBeNull()
  })

  it('keeps the lane a link opened when a version in it is clicked', async () => {
    // A link to a style prompt opens the style lane. Clicking a row there fell
    // back to the first lane and its newest text, because only a pick by hand
    // told the panel which lane it was in.
    const { client } = renderApp(`/works/${IDS.song}/versions?version=${IDS.style}`)
    await settled(client)
    const main = await screen.findByRole('main')
    const prompt = 'warm synth pop, slow build, airy female vocal'
    await within(main).findByText(prompt)
    const lanes = within(main).getByRole('group', { name: en.versions.lanes })
    expect(within(lanes).getByRole('button', { name: /^Style prompt/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    )

    fireEvent.click(within(main).getByRole('option', { name: /^v1/ }))
    expect(await within(main).findByText(prompt)).toBeInTheDocument()
    expect(within(lanes).getByRole('button', { name: /^Style prompt/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('offers the invitation, not a form, in a role of commentary with nothing in it', async () => {
    const { main } = await openVersions()
    const lanes = within(main).getByRole('group', { name: en.versions.lanes })
    fireEvent.click(within(lanes).getByRole('button', { name: /^Review/ }))

    expect(await within(main).findAllByText(en.versions.noCommentary)).not.toHaveLength(0)
    expect(within(main).queryByRole('textbox', { name: en.versions.draftPlaceholder })).toBeNull()
    expect(within(main).queryByRole('button', { name: en.versions.new })).toBeNull()
  })
})
