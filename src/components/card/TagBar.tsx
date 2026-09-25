import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, X } from 'lucide-react'
import {
  Combobox,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
} from '@/components/ui/combobox'
import { updateWork, workTags, type Mark, type Work } from '@/lib/api'
import { announceEdited } from '@/lib/edited'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
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
  const patch = useMutation({
    mutationFn: (changes: { tags?: string[]; marks?: string[] }) =>
      updateWork(work.id, changes),
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      announceEdited({
        client,
        message: t('toast.tagsEdited'),
        refresh: [keys.workTags, keys.catalogue],
      })
    },
    onError: (cause) => say.failedTo(t('toast.workSaveFailed'), cause),
  })

  // What the workspace already says, so the second winter song is tagged from
  // the list rather than retyped into a near-miss.
  const known = useQuery({
    queryKey: keys.workTags,
    queryFn: workTags,
    staleTime: 30_000,
    enabled: adding,
  })

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
      {marks.map((mark) => {
        const on = raised.has(mark.key)
        const Icon = markIconOf(mark)
        return (
          <button
            key={mark.key}
            type="button"
            title={t(on ? 'work.markOff' : 'work.markOn', { mark: mark.label })}
            onClick={() =>
              patch.mutate({
                marks: on
                  ? work.marks.filter((key) => key !== mark.key)
                  : [...work.marks, mark.key],
              })
            }
            className={cn(
              'inline-flex cursor-pointer items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] transition-colors',
              // Off is an outline: the row of what could be raised is always
              // there, so raising one is a click and not a hunt through a menu.
              on
                ? cn('font-medium', MARK_ON[mark.colour ?? 'plain'])
                : 'border-line text-faint hover:text-dim',
            )}
          >
            <Icon aria-hidden className="size-3" />
            {sayLabel(mark.label)}
          </button>
        )
      })}

      {marks.length > 0 && work.tags.length > 0 && (
        <span aria-hidden className="mx-0.5 h-3.5 w-px bg-line" />
      )}

      {work.tags.map((tag) => (
        <span
          key={tag}
          className="group inline-flex items-center gap-1 rounded-full bg-soft px-2 py-0.5 text-[11px] text-dim"
        >
          {tag}
          <button
            type="button"
            title={t('work.removeTag', { tag })}
            onClick={() => patch.mutate({ tags: work.tags.filter((kept) => kept !== tag) })}
            className="cursor-pointer text-faint transition-colors hover:text-bad"
          >
            <X aria-hidden className="size-3" />
          </button>
        </span>
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
            className="h-auto w-40 rounded-full border-accent px-2 py-0.5 text-[11px]"
          />

          <ComboboxPopup className="w-48 p-1">
            <ComboboxList>
              {(tag: string) => (
                <ComboboxItem
                  key={tag}
                  value={tag}
                  onClick={() => addTag(tag)}
                  className="rounded-md px-2 py-1 text-[11px]"
                >
                  {tag}
                </ComboboxItem>
              )}
            </ComboboxList>
            <ComboboxEmpty className="px-2 py-1 text-[11px]">
              {t('work.tagNoMatch')}
            </ComboboxEmpty>
          </ComboboxPopup>
        </Combobox>
      ) : (
        <button
          type="button"
          onClick={() => {
            setAdding(true)
            setOpen(true)
          }}
          title={t('work.addTag')}
          className="inline-flex cursor-pointer items-center gap-0.5 rounded-full border border-dashed border-line px-2 py-0.5 text-[11px] text-faint transition-colors hover:border-line-2 hover:text-dim"
        >
          <Plus aria-hidden className="size-3" />
          {t('work.addTag')}
        </button>
      )}
    </div>
  )
}

/** A raised mark's colours, by the palette role its profile named. */
const MARK_ON: Record<string, string> = {
  plain: 'border-line-2 bg-soft text-text',
  accent: 'border-transparent bg-accent-soft text-accent',
  good: 'border-transparent bg-good-soft text-good',
  warn: 'border-transparent bg-warn-soft text-warn',
  bad: 'border-transparent bg-bad-soft text-bad',
  info: 'border-transparent bg-info-soft text-info',
}
