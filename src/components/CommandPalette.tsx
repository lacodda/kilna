import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { search, type Hit, type HitKind } from '@/lib/api'
import { coverFor } from '@/lib/cover'
import { keys } from '@/lib/query'
import { cn } from '@/lib/utils'
import { ComboboxGroupLabel } from '@/components/ui/combobox'
import {
  CommandPalette as Palette,
  CommandPaletteEmpty,
  CommandPaletteGroup,
  CommandPaletteInput,
  CommandPaletteItem,
  CommandPaletteList,
  CommandPalettePopup,
  CommandPaletteRow,
  commandPaletteItemVariants,
} from '@/components/ui/command-palette'

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

/** One group of hits, in the shape Base UI reads a grouped list in. */
interface Group {
  kind: HitKind
  items: Hit[]
}

/**
 * One box that finds anything.
 *
 * The palette is for recognising something you already know exists — a song
 * whose title you half remember, a line you wrote last month. So it shows a few
 * hits per kind rather than everything, and every hit opens the work it belongs
 * to on the tab where that hit lives.
 */
export function CommandPalette({ open, onOpenChange }: Props) {
  // Mounted only while open, so every visit starts from an empty box: the
  // palette is somewhere you pass through, not somewhere you return to with
  // your last query still in it.
  return open ? <Contents onOpenChange={onOpenChange} /> : null
}

function Contents({ onOpenChange }: { onOpenChange: (open: boolean) => void }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [query, setQuery] = useState('')

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

  // Hits arrive grouped by kind already; this fixes the order they are shown
  // in, which is also the order the arrow keys walk. The walking itself is the
  // palette component's — it reads this same array.
  const groups = useMemo<Group[]>(() => {
    const data = hits.data ?? []
    return GROUPS.map((kind) => ({
      kind,
      items: data.filter((hit) => hit.kind === kind),
    })).filter((group) => group.items.length > 0)
  }, [hits.data])

  const openHit = (hit: Hit) => {
    navigate(`/works/${hit.work_id}/${TAB_FOR[hit.kind]}`)
    onOpenChange(false)
  }

  // What is shown while there is nothing to show: a prompt, then the wait, then
  // the miss. One line, because `Empty` must stay mounted for its announcement
  // to fire rather than be swapped in and out.
  const nothing =
    settled.trim() === ''
      ? t('search.hint')
      : hits.isPending
        ? t('search.searching')
        : t('search.nothing', { query: settled })

  return (
    <Palette
      items={groups}
      // The searching is the server's. Base UI would otherwise filter these
      // hits a second time against the same query, by title alone — and drop
      // every hit that matched on a body it cannot see.
      filter={null}
      open
      onOpenChange={(next) => {
        if (!next) onOpenChange(false)
      }}
      onValueChange={(hit: Hit | null) => {
        if (hit !== null) openHit(hit)
      }}
      // A hit is an object, and two fetches never return the same one twice;
      // without this the highlight would compare by reference and never match.
      isItemEqualToValue={(a: Hit, b: Hit) =>
        a.kind === b.kind && a.work_id === b.work_id && a.rank === b.rank
      }
    >
      <CommandPalettePopup aria-label={t('search.title')}>
        <CommandPaletteInput
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t('search.placeholder')}
          aria-label={t('search.title')}
          hint={['Escape']}
        />

        <div className="min-h-0 flex-1 overflow-y-auto py-1.5">
          <CommandPaletteEmpty className="px-4 py-6 text-center text-xs text-faint">
            {nothing}
          </CommandPaletteEmpty>

          <CommandPaletteList>
            {(group: Group) => (
              <CommandPaletteGroup key={group.kind} items={group.items}>
                <ComboboxGroupLabel className="px-3.5 pt-2 pb-1 tracking-[0.08em]">
                  {t(`search.group.${group.kind}`)}
                </ComboboxGroupLabel>

                {/* Mapped by hand rather than through a second List: a List
                    is the `role="listbox"`, and there is one of those per
                    palette. Each row's place in the walk order is resolved by
                    the component from `value` and `isItemEqualToValue`. */}
                {group.items.map((hit) => (
                  <CommandPaletteItem
                    key={`${hit.kind}-${hit.work_id}-${hit.rank}`}
                    value={hit}
                    className={cn(
                      commandPaletteItemVariants(),
                      'mx-1.5 pr-3 data-[highlighted]:bg-accent-soft',
                    )}
                  >
                    <CommandPaletteRow
                      icon={
                        <span
                          aria-hidden
                          className="size-6 shrink-0 rounded-md"
                          style={{ background: coverFor(hit.work_id) }}
                        />
                      }
                      hint={hit.detail}
                    >
                      <b className="block truncate text-[13px] font-medium">{hit.title}</b>
                      {/* Which work it came from matters most for a hit that
                          is a line of text rather than a title. */}
                      {hit.kind !== 'work' && (
                        <span className="block truncate text-[11px] text-faint">
                          {hit.work_title}
                        </span>
                      )}
                    </CommandPaletteRow>
                  </CommandPaletteItem>
                ))}
              </CommandPaletteGroup>
            )}
          </CommandPaletteList>
        </div>

        <div className="flex gap-3.5 border-t border-line px-4 py-2.5 font-mono text-[11px] text-faint">
          <span>{t('search.navigate')}</span>
          <span>{t('search.openHit')}</span>
          <span>{t('search.close')}</span>
        </div>
      </CommandPalettePopup>
    </Palette>
  )
}
