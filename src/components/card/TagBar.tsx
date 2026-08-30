import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, X } from 'lucide-react'
import { updateWork, workTags, type Mark, type Work } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
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
  const [draft, setDraft] = useState('')

  const patch = useMutation({
    mutationFn: (changes: { tags?: string[]; marks?: string[] }) =>
      updateWork(work.id, changes),
    onSuccess: (updated) => {
      client.setQueryData(keys.work(work.id), updated)
      void client.invalidateQueries({ queryKey: keys.workTags })
      void client.invalidateQueries({ queryKey: keys.catalogue })
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
              'cursor-pointer rounded-full border px-2 py-0.5 text-[11px] transition-colors',
              // Off is an outline: the row of what could be raised is always
              // there, so raising one is a click and not a hunt through a menu.
              on ? MARK_ON[mark.colour ?? 'plain'] : 'border-line text-faint hover:text-dim',
            )}
          >
            {mark.label}
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
        <span className="relative">
          <input
            autoFocus
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') addTag(draft)
              if (event.key === 'Escape') {
                setDraft('')
                setAdding(false)
              }
            }}
            // Not on blur: clicking a suggestion blurs the box, and saving on
            // blur would race the click and swallow it.
            placeholder={t('work.tagPlaceholder')}
            aria-label={t('work.addTag')}
            className="w-40 rounded-full border border-accent bg-transparent px-2 py-0.5 text-[11px] outline-none"
          />
          {suggestions.length > 0 && (
            <span className="absolute left-0 top-6 z-30 flex w-48 flex-col rounded-[10px] border border-line bg-raise p-1 shadow-lg">
              {suggestions.map((tag) => (
                <button
                  key={tag}
                  type="button"
                  onMouseDown={(event) => {
                    // Down, not click: the input's blur would otherwise close
                    // the list before the click landed.
                    event.preventDefault()
                    addTag(tag)
                  }}
                  className="cursor-pointer rounded-[7px] px-2 py-1 text-left text-[11px] text-dim transition-colors hover:bg-soft hover:text-text"
                >
                  {tag}
                </button>
              ))}
            </span>
          )}
        </span>
      ) : (
        <button
          type="button"
          onClick={() => setAdding(true)}
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
}
