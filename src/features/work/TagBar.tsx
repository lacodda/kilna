import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import {
  Combobox,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'
import type { Mark, Work } from '@/lib/api/types'
import { updateWork } from '@/lib/api/works'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { markIconOf } from '@/lib/markIcon'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'

/**
 * The work's own words, and the flags raised on it.
 *
 * Two lists side by side rather than one: a tag says what the work *is* and
 * stays with it, a mark says something about this week and comes off. They look
 * alike on purpose — both are chips you click — but a mark comes from the
 * profile's short list and a tag is whatever the author types.
 *
 * Neither derives anything. The status above already answers "where is this in
 * the process", and a second thing that quietly moved a work would be a second
 * answer to a question that has one.
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
  const raised = new Set(work.marks)

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
    <div className="mt-2 flex flex-wrap items-center gap-1.5">
      {/* Off is an outline: the row of what could be raised is always there,
          so raising one is a click and not a hunt through a menu. One group,
          because the marks are one value - the set raised on the work - and
          a press adds or takes away one of them, keeping any the profile no
          longer lists. */}
      {marks.length > 0 && (
        <ChipGroup
          multiple
          aria-label={t('work.marks')}
          value={work.marks}
          onValueChange={(next) => patch.mutate({ marks: next })}
        >
          {marks.map((mark) => {
            const on = raised.has(mark.key)
            const Icon = markIconOf(mark)
            return (
              <Chip
                key={mark.key}
                value={mark.key}
                title={t(on ? 'work.markOff' : 'work.markOn', { mark: mark.label })}
              >
                {/* A raised chip wears the set's "on"; the icon keeps the
                    colour the profile gave the mark, so a warning still
                    reads as one. */}
                <Icon
                  aria-hidden
                  className={cn('size-3', on && MARK_INK[mark.colour ?? 'plain'])}
                />
                {sayLabel(mark.label)}
              </Chip>
            )
          })}
        </ChipGroup>
      )}

      {marks.length > 0 && work.tags.length > 0 && (
        <span aria-hidden className="mx-0.5 h-3.5 w-px bg-line" />
      )}

      {work.tags.map((tag) => (
        <Chip
          key={tag}
          variant="soft"
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
        >
          <Plus aria-hidden />
          {t('work.addTag')}
        </Button>
      )}
    </div>
  )
}

/** A raised mark's icon colour, by the palette role its profile named. Plain
    and accent take the chip's own ink. */
const MARK_INK: Record<string, string> = {
  plain: '',
  accent: '',
  good: 'text-good',
  warn: 'text-warn',
  bad: 'text-bad',
  info: 'text-info',
}
