import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Combobox,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'

interface Props {
  /** The tags the note already carries, never offered again. */
  have: string[]
  /** Every tag in use across the notes, most used first. */
  known: string[]
  onAdd: (tag: string) => void
}

/**
 * The small "+ tag" button at the end of a note's tags that turns into a box.
 *
 * The same shape as the work's tag bar, for the reason that one gives: a
 * Combobox brings the dismiss layer, the portal and the arrow keys, and a
 * hand-made list under an input had none of them. Offered from the notes' own
 * vocabulary, not the works' — the two are different words (see `work_tags`).
 */
export function NoteTagAdder({ have, known, onAdd }: Props) {
  const { t } = useTranslation()
  const [adding, setAdding] = useState(false)
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState('')
  const add = (tag: string) => {
    const value = tag.trim().replace(/,+$/, '').trim()
    setDraft('')
    setOpen(false)
    setAdding(false)
    if (value !== '') onAdd(value)
  }

  const suggestions = known
    .filter(
      (tag) =>
        !have.some((mine) => mine.toLowerCase() === tag.toLowerCase()) &&
        tag.toLowerCase().includes(draft.trim().toLowerCase()),
    )
    .slice(0, 8)

  if (!adding) {
    return (
      <Button
        size="xs"
        onClick={() => {
          setAdding(true)
          setOpen(true)
        }}
      >
        <Plus aria-hidden />
        {t('notes.addTag')}
      </Button>
    )
  }

  return (
    <Combobox
      items={suggestions}
      filter={null}
      value={draft}
      onValueChange={(next) => setDraft(typeof next === 'string' ? next : '')}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setDraft('')
          setAdding(false)
        }
      }}
    >
      <ComboboxInput
        autoFocus
        placeholder={t('notes.tagPlaceholder')}
        aria-label={t('notes.addTag')}
        onKeyDown={(event) => {
          // A tag is whatever the author types; with a row highlighted, Enter
          // belongs to the list.
          if (event.key === 'Enter' && !event.defaultPrevented) add(draft)
        }}
        className="h-auto w-36 rounded-full border-accent px-2.5 py-0.5 text-xs"
      />
      <ComboboxPopup className="w-48 p-1">
        <ComboboxList>
          {(tag: string) => (
            <ComboboxItem
              key={tag}
              value={tag}
              onClick={() => add(tag)}
              className="rounded-md px-2 py-1 text-xs"
            >
              {tag}
            </ComboboxItem>
          )}
        </ComboboxList>
        <ComboboxEmpty className="px-2 py-1 text-xs">{t('notes.tagNew')}</ComboboxEmpty>
      </ComboboxPopup>
    </Combobox>
  )
}
