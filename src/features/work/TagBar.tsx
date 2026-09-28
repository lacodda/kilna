import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import {
  Combobox,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import type { Mark, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { badgeVariantOf, markIconOf } from '@/lib/markIcon'
import { say as sayLabel, useProfile } from '@/lib/useProfile'

/**
 * The work's own words, and the flags raised on it.
 *
 * Two lists side by side rather than one: a tag says what the work *is* and
 * stays with it, a mark says something about this week and comes off. They look
 * alike on purpose — both are chips with a cross that takes them away — but a
 * mark comes from the profile's short list and a tag is whatever the author
 * types.
 *
 * Only the marks that are raised are drawn. Every mark the profile knows stood
 * here as an empty outline until v0.80, three switches before the first word
 * of what the work is; raising one is now the menu behind "+ Mark", beside
 * "+ Tag".
 *
 * Neither derives anything. The status beside them already answers "where is
 * this in the process", and a second thing that quietly moved a work would be
 * a second answer to a question that has one.
 *
 * Drawn into the header's row of chips rather than a row of its own, so it
 * returns the chips alone.
 */
export function TagBar({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()
  const [adding, setAdding] = useState(false)
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState('')
  const patch = useAppMutation({
    mutationFn: (changes: { tags?: string[]; marks?: string[] }) => updateWork(work.id, changes),
    failure: 'toast.workSaveFailed',
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.tagsEdited'),
        refresh: [keys.workTags, keys.catalogue],
      })
    },
  })

  // What the workspace already says, so the second winter song is tagged from
  // the list rather than retyped into a near-miss.
  const known = useQuery({ ...queries.workTags(), staleTime: 30_000, enabled: adding })

  const marks: Mark[] = profile.config.marks ?? []
  // A mark the profile no longer defines is not drawn - the catalogue's rule
  // too - but it stays on the work: raising or lowering another one sends the
  // whole set back with it untouched.
  const raised = marks.filter((mark) => work.marks.includes(mark.key))
  const lowered = marks.filter((mark) => !work.marks.includes(mark.key))

  const addTag = (tag: string) => {
    const value = tag.trim()
    if (value === '') return
    setDraft('')
    setOpen(false)
    setAdding(false)
    // Sent as typed; the backend trims, drops blanks and folds duplicates, so
    // the rule lives in one place rather than in every box that adds a tag.
    patch.mutate({ tags: [...work.tags, value] })
  }

  const suggestions = (known.data ?? [])
    .map(([tag]) => tag)
    .filter(
      (tag) =>
        !work.tags.some((existing) => existing.toLowerCase() === tag.toLowerCase()) &&
        tag.toLowerCase().includes(draft.trim().toLowerCase()),
    )
    .slice(0, 8)

  return (
    <>
      {raised.map((mark) => {
        const Icon = markIconOf(mark)
        const label = sayLabel(mark.label)
        return (
          // The mark wears the colour its profile gave it, so a warning
          // still reads as one.
          <Chip
            key={mark.key}
            variant={badgeVariantOf(mark.colour ?? 'plain')}
            removeLabel={t('work.markOff', { mark: label })}
            onRemove={() => patch.mutate({ marks: work.marks.filter((key) => key !== mark.key) })}
          >
            <Icon aria-hidden className="size-3" />
            {label}
          </Chip>
        )
      })}

      {work.tags.map((tag) => (
        <Chip
          key={tag}
          removeLabel={t('work.removeTag', { tag })}
          onRemove={() => patch.mutate({ tags: work.tags.filter((kept) => kept !== tag) })}
        >
          {tag}
        </Chip>
      ))}

      {adding ? (
        // A Combobox rather than an input with a list under it. The hand-made
        // version had no way to close: an outside click and Escape both need a
        // dismiss layer around the popup, and writing one per dropdown is how
        // an app ends up with three that behave differently. Base UI's brings
        // the layer, the portal and the arrow keys with it.
        <Combobox
          items={suggestions}
          // Filtered here, against the tags already on the work as well as the
          // query, so the list never offers a word that is already a chip.
          filter={null}
          value={draft}
          onValueChange={(next) => setDraft(typeof next === 'string' ? next : '')}
          open={open}
          onOpenChange={(next) => {
            setOpen(next)
            // Closing is the end of the errand, not a pause in it: leaving the
            // box behind would put a second, empty tag field on the card.
            if (!next) {
              setDraft('')
              setAdding(false)
            }
          }}
        >
          <ComboboxInput
            autoFocus
            placeholder={t('work.tagPlaceholder')}
            aria-label={t('work.addTag')}
            onKeyDown={(event) => {
              // A tag is whatever the author types, so Enter takes the typed
              // word. Only when nothing is highlighted: with a highlighted row
              // Enter belongs to the list.
              if (event.key === 'Enter' && !event.defaultPrevented) addTag(draft)
            }}
            className="h-auto w-40 rounded-full border-accent px-2 py-0.5 text-xs"
          />

          <ComboboxPopup className="w-48 p-1">
            <ComboboxList>
              {(tag: string) => (
                <ComboboxItem
                  key={tag}
                  value={tag}
                  onClick={() => addTag(tag)}
                  className="rounded-md px-2 py-1 text-xs"
                >
                  {tag}
                </ComboboxItem>
              )}
            </ComboboxList>
            <ComboboxEmpty className="px-2 py-1 text-xs">{t('work.tagNoMatch')}</ComboboxEmpty>
          </ComboboxPopup>
        </Combobox>
      ) : (
        <Button
          size="xs"
          onClick={() => {
            setAdding(true)
            setOpen(true)
          }}
          title={t('work.addTag')}
          className={ADD}
        >
          <Plus aria-hidden />
          {t('work.addTag')}
        </Button>
      )}

      {/* Only while there is a mark left to raise: a menu of nothing is a
          button that lies about what it holds. */}
      {lowered.length > 0 && (
        <Menu>
          {/* Named in full for a reader: "Mark" alone, beside chips that are
              marks, does not say that it raises one. */}
          <MenuTrigger
            render={
              <Button
                size="xs"
                title={t('work.addMark')}
                aria-label={t('work.addMark')}
                className={ADD}
              />
            }
          >
            <Plus aria-hidden />
            {t('work.mark')}
          </MenuTrigger>
          <MenuPopup align="start">
            {lowered.map((mark) => {
              const Icon = markIconOf(mark)
              return (
                <MenuItem
                  key={mark.key}
                  onClick={() => patch.mutate({ marks: [...work.marks, mark.key] })}
                >
                  <Icon aria-hidden className={MARK_INK[mark.colour ?? 'plain']} />
                  {sayLabel(mark.label)}
                </MenuItem>
              )
            })}
          </MenuPopup>
        </Menu>
      )}
    </>
  )
}

/**
 * The clothes of an adding button in a row of chips: the chip's height and
 * round ends, and a broken border - the mockup's dashed chip, which stands
 * for something not there yet rather than one more value.
 */
const ADD = 'h-auto rounded-full border-dashed border-line-2 py-0.5 pr-2.5 pl-2'

/** A mark's icon colour in the menu that raises it, by the palette role its
    profile named - the colour it wears once raised. Plain and accent take the
    item's own ink. */
const MARK_INK: Record<string, string> = {
  plain: '',
  accent: '',
  good: 'text-good',
  warn: 'text-warn',
  bad: 'text-bad',
  info: 'text-info',
}
