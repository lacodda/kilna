import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import * as Primitive from '@radix-ui/react-dialog'
import { useQuery } from '@tanstack/react-query'
import { Search } from 'lucide-react'
import { search, type Hit, type HitKind } from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { keys } from '@/lib/query'
import { cn } from '@/lib/utils'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Which tab a hit should open the work on. */
const TAB_FOR: Record<HitKind, string> = {
  work: 'overview',
  version: 'versions',
  note: 'notes',
  message: 'assistant',
}

/** The order the groups appear in, coarsest first. */
const GROUPS: HitKind[] = ['work', 'version', 'note', 'message']

/**
 * One box that finds anything.
 *
 * The palette is for recognising something you already know exists — a song
 * whose title you half remember, a line you wrote last month. So it shows a few
 * hits per kind rather than everything, and every hit opens the work it belongs
 * to on the tab where that hit lives.
 */
export function CommandPalette({ open, onOpenChange }: Props) {
  return (
    <Primitive.Root open={open} onOpenChange={onOpenChange}>
      <Primitive.Portal>
        <Primitive.Overlay className="fixed inset-0 z-70 bg-black/55 backdrop-blur-[2px]" />
        {/* Mounted only while open, so every visit starts from an empty box:
            the palette is somewhere you pass through, not somewhere you return
            to with your last query still in it. */}
        <Contents onOpenChange={onOpenChange} />
      </Primitive.Portal>
    </Primitive.Root>
  )
}

function Contents({ onOpenChange }: { onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const listRef = useRef<HTMLDivElement>(null)

  // Typing is faster than SQLite is slow, but not by much once a workspace has
  // a few hundred bodies in it — so the query trails the keystrokes.
  const [settled, setSettled] = useState('')
  useEffect(() => {
    const timer = setTimeout(() => setSettled(query), 140)
    return () => clearTimeout(timer)
  }, [query])

  const hits = useQuery({
    queryKey: keys.search(settled),
    queryFn: () => search(settled),
    enabled: settled.trim() !== '',
  })

  // Hits arrive grouped by kind already; this fixes the order the arrow keys
  // walk, so the flat position of a row is known before it is drawn rather
  // than counted up while drawing.
  const groups = useMemo(() => {
    const data = hits.data ?? []
    return GROUPS.map((kind) => ({ kind, hits: data.filter((hit) => hit.kind === kind) })).filter(
      (group) => group.hits.length > 0,
    )
  }, [hits.data])

  const found = useMemo(() => groups.flatMap((group) => group.hits), [groups])

  // A new set of hits invalidates whatever was highlighted. Adjusting during
  // render rather than in an effect avoids the frame where the highlight sits
  // on a row that is no longer there.
  const [highlightedFor, setHighlightedFor] = useState(found)
  if (highlightedFor !== found) {
    setHighlightedFor(found)
    setActive(0)
  }

  const openHit = (hit: Hit) => {
    navigate(`/works/${hit.work_id}/${TAB_FOR[hit.kind]}`)
    onOpenChange(false)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (found.length === 0) return
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        setActive((current) => (current + 1) % found.length)
        break
      case 'ArrowUp':
        event.preventDefault()
        setActive((current) => (current - 1 + found.length) % found.length)
        break
      case 'Enter': {
        event.preventDefault()
        const hit = found[active]
        if (hit !== undefined) openHit(hit)
        break
      }
      default:
        break
    }
  }

  // Keep the highlighted row in view when the arrows walk past the fold.
  useEffect(() => {
    listRef.current?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
  }, [active])

  return (
    <Primitive.Content
          onKeyDown={onKeyDown}
          className="fixed left-1/2 top-[14vh] z-80 flex max-h-[70vh] w-[34rem] max-w-[92vw] -translate-x-1/2 flex-col overflow-hidden rounded-2xl border border-line-2 bg-raise shadow-raise"
        >
          <Primitive.Title className="sr-only">{t('search.title')}</Primitive.Title>
          <Primitive.Description className="sr-only">{t('search.hint')}</Primitive.Description>

          <div className="flex items-center gap-2.5 border-b border-line px-4 py-3">
            <Search aria-hidden className="size-4 shrink-0 text-faint" />
            <input
              autoFocus
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('search.placeholder')}
              aria-label={t('search.title')}
              className="w-full bg-transparent text-sm text-text outline-none placeholder:text-faint"
            />
          </div>

          <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto py-1.5">
            {settled.trim() === '' && (
              <p className="px-4 py-6 text-center text-xs text-faint">{t('search.hint')}</p>
            )}

            {settled.trim() !== '' && hits.isPending && (
              <p className="px-4 py-6 text-center text-xs text-faint">{t('search.searching')}</p>
            )}

            {settled.trim() !== '' && !hits.isPending && found.length === 0 && (
              <p className="px-4 py-6 text-center text-xs text-faint">
                {t('search.nothing', { query: settled })}
              </p>
            )}

            {groups.map((group) => {
              // Where this group's first row sits in the flat order the arrows
              // walk through.
              const offset = found.indexOf(group.hits[0]!)

              return (
                <div key={group.kind}>
                  <div className="px-3.5 pt-2 pb-1 text-[10.5px] font-medium uppercase tracking-[0.08em] text-faint">
                    {t(`search.group.${group.kind}`)}
                  </div>
                  {group.hits.map((hit, indexInGroup) => {
                    const position = offset + indexInGroup
                    return (
                      <button
                        key={`${hit.kind}-${hit.work_id}-${hit.rank}`}
                        type="button"
                        data-active={position === active}
                        onMouseEnter={() => setActive(position)}
                        onClick={() => openHit(hit)}
                        className={cn(
                          'mx-1.5 flex w-[calc(100%-0.75rem)] items-center gap-2.5 rounded-[9px] px-3 py-2 text-left',
                          position === active && 'bg-accent-soft',
                        )}
                      >
                        <span
                          aria-hidden
                          className="size-6 shrink-0 rounded-md"
                          style={{ background: coverFor(hit.work_id) }}
                        />
                        <span className="min-w-0 flex-1">
                          <b className="block truncate text-[13px] font-medium">{hit.title}</b>
                          {/* Which work it came from matters most for a hit that
                              is a line of text rather than a title. */}
                          {hit.kind !== 'work' && (
                            <span className="block truncate text-[11px] text-faint">
                              {hit.work_title}
                            </span>
                          )}
                        </span>
                        <small className="shrink-0 text-[11px] text-faint">{hit.detail}</small>
                      </button>
                    )
                  })}
                </div>
              )
            })}
          </div>

          <div className="flex gap-3.5 border-t border-line px-4 py-2.5 font-mono text-[11px] text-faint">
            <span>{t('search.navigate')}</span>
            <span>{t('search.openHit')}</span>
            <span>{t('search.close')}</span>
          </div>
    </Primitive.Content>
  )
}
