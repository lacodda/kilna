import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, Plus } from 'lucide-react'
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
import {
  Menu,
  MenuCheckboxIndicator,
  MenuCheckboxItem,
  MenuPopup,
  MenuTrigger,
} from '@/components/ui/menu'
import { MarkAvatars } from '@/components/MarkAvatars'
import type { Mark, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { markIconOf } from '@/lib/markIcon'
import { say as sayLabel, useProfile } from '@/lib/useProfile'

/**
 * The work's own words, and the flags raised on it.
 *
 * Two lists side by side rather than one: a tag says what the work *is* and
 * stays with it, a mark says something about this week and comes off. A tag
 * is whatever the author types, a chip with a cross that takes it away; a
 * mark comes from the profile's short list and is drawn as the catalogue
 * draws it since v0.90.1 - its icon on a round tile in its hue, its name on
 * hover (`MarkAvatars`).
 *
 * Only the marks that are raised are drawn. Every mark the profile knows stood
 * here as an empty outline until v0.80, three switches before the first word
 * of what the work is. The raised ones are the way into the profile's list -
 * each ticked or not, a press raises or lowers one - and "+ Mark" stands in
 * for them while none is raised.
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

      {/* Only where the profile has marks: a menu of nothing is a button
          that lies about what it holds. */}
      {marks.length > 0 && (
        <Menu>
          <MenuTrigger
            render={
              raised.length === 0 ? (
                // Named in full for a reader: "Mark" alone does not say that
                // it raises one.
                <Button
                  size="xs"
                  title={t('work.addMark')}
                  aria-label={t('work.addMark')}
                  className={ADD}
                />
              ) : (
                // The tiles themselves, named by every mark they show: inside
                // a button its tiles are drawn, not read, so the button's
                // name is what a reader hears.
                <Button
                  variant="icon"
                  size="xs"
                  aria-label={t('work.marksRaised', {
                    marks: raised.map((mark) => sayLabel(mark.label)).join(', '),
                  })}
                  className="rounded-full px-1"
                />
              )
            }
          >
            {raised.length === 0 ? (
              <>
                <Plus aria-hidden />
                {t('work.mark')}
              </>
            ) : (
              <MarkAvatars marks={raised} />
            )}
          </MenuTrigger>
          <MenuPopup align="start">
            {marks.map((mark) => {
              const Icon = markIconOf(mark)
              return (
                <MenuCheckboxItem
                  key={mark.key}
                  checked={work.marks.includes(mark.key)}
                  // Sent over the work as it stands: a mark the profile no
                  // longer defines stays on it, untouched.
                  onCheckedChange={(next) =>
                    patch.mutate({
                      marks: next
                        ? [...work.marks, mark.key]
                        : work.marks.filter((key) => key !== mark.key),
                    })
                  }
                >
                  <Icon aria-hidden className={MARK_INK[mark.colour ?? 'plain']} />
                  <span className="flex-1">{sayLabel(mark.label)}</span>
                  <MenuCheckboxIndicator>
                    <Check aria-hidden className="size-3.5" />
                  </MenuCheckboxIndicator>
                </MenuCheckboxItem>
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

/** A mark's icon colour in the menu that raises and lowers it, by the palette
    role its profile named - the colour its tile wears once raised. Plain and
    accent take the item's own ink. */
const MARK_INK: Record<string, string> = {
  plain: '',
  accent: '',
  good: 'text-good',
  warn: 'text-warn',
  bad: 'text-bad',
  info: 'text-info',
}
