import { fireEvent, screen, within } from '@testing-library/react'
import en from '@/i18n/locales/en.json'

/**
 * Open a part of the profile editor by the word on its chip: "Song", "Audio",
 * the craft, the fields, the actions. The editor shows one part at a time
 * (v0.112), so a test about a kind's words opens that kind first.
 */
export async function openProfilePart(label: string): Promise<void> {
  const parts = await screen.findByRole('group', { name: en.editor.parts })
  fireEvent.click(within(parts).getByRole('button', { name: label }))
}
