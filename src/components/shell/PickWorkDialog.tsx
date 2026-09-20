import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { catalogue, type ScoredWork } from '@/lib/api'
import { coverImageFor } from '@/lib/cover'
import { keys } from '@/lib/query'
import { useCovers } from '@/lib/useCovers'
import { cn } from '@/lib/utils'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { stageAt } from '@/lib/stages'
import { ComboboxGroupLabel } from '@/components/ui/combobox'
import { StageDial } from '@/components/StageDial'
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
  /** What the chosen work is for; the dialog closes itself afterwards. */
  onPick: (work: ScoredWork) => void
  /** The day being filled, shown so it is obvious what is being answered. */
  title: string
}

/**
 * Choosing a work that already exists, by name or by eye.
 *
 * The calendar's plus used to open the dialog for MAKING a work, which is
 * almost never what putting something in the calendar means: the song exists,
 * and the question is which of four hundred goes out on Friday. So this asks
 * that question instead — a box to type a name into, the kinds along the top,
 * and the stage against each row so what is nearly ready is visible without
 * opening anything.
 *
 * It is the palette's own component, not a second one that looks like it: the
 * arrows, Enter, Escape, the highlight and the scrolling are all already
 * there. Only the list underneath is different — the catalogue rather than the
 * search index, because a work with no text in it yet still has to be
 * schedulable, and the search index only knows what has been written.
 */
export function PickWorkDialog({ open, onOpenChange, onPick, title }: Props) {
  return open ? <Contents onOpenChange={onOpenChange} onPick={onPick} title={title} /> : null
}

function Contents({ onOpenChange, onPick, title }: Omit<Props, 'open'>) {
  const { t } = useTranslation()
  const profile = useProfile()
  const [query, setQuery] = useState('')
  const [kind, setKind] = useState<string | null>(null)

  const rows = useQuery({ queryKey: keys.catalogue, queryFn: catalogue })

  // Filtering here rather than in SQL: the catalogue is one query the app
  // already holds, and four hundred titles filter faster than a round trip.
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return (rows.data ?? [])
      .filter((row) => (kind === null || row.kind === kind) && row.title.toLowerCase().includes(needle))
      .slice(0, 200)
  }, [rows.data, query, kind])

  const groups = useMemo(() => (shown.length === 0 ? [] : [{ items: shown }]), [shown])

  const nothing = rows.isPending
    ? t('search.searching')
    : query.trim() === ''
      ? t('pick.empty')
      : t('search.nothing', { query })

  return (
    <Palette
      items={groups}
      // The filtering is done above, against the kind as well as the text;
      // Base UI would otherwise narrow the same rows a second time by title
      // alone and undo the kind.
      filter={null}
      open
      onOpenChange={(next) => {
        if (!next) onOpenChange(false)
      }}
      onValueChange={(row: ScoredWork | null) => {
        if (row !== null) {
          onPick(row)
          onOpenChange(false)
        }
      }}
      isItemEqualToValue={(a: ScoredWork, b: ScoredWork) => a.work_id === b.work_id}
    >
      <CommandPalettePopup aria-label={title}>
        <CommandPaletteInput
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t('pick.placeholder')}
          aria-label={title}
          hint={['Escape']}
        />

        {/* The kinds as chips rather than a dropdown: there are four of them,
            and a menu that has to be opened to see four things is a menu in
            the way. */}
        <div className="flex flex-wrap items-center gap-1.5 border-b border-line px-3 py-2">
          <Chip on={kind === null} onClick={() => setKind(null)}>
            {t('pick.anyKind')}
          </Chip>
          {profile.config.work_kinds.map((entry) => (
            <Chip
              key={entry.key}
              on={kind === entry.key}
              onClick={() => setKind(kind === entry.key ? null : entry.key)}
            >
              {sayLabel(entry.label)}
            </Chip>
          ))}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto py-1.5">
          <CommandPaletteEmpty className="px-4 py-6 text-center text-xs text-faint">
            {nothing}
          </CommandPaletteEmpty>

          <CommandPaletteList>
            {(group: { items: ScoredWork[] }) => (
              <CommandPaletteGroup items={group.items}>
                <ComboboxGroupLabel className="px-3.5 pt-2 pb-1 tracking-[0.08em]">
                  {t('pick.group', { count: group.items.length })}
                </ComboboxGroupLabel>

                {group.items.map((row) => (
                  <Row key={row.work_id} row={row} />
                ))}
              </CommandPaletteGroup>
            )}
          </CommandPaletteList>
        </div>
      </CommandPalettePopup>
    </Palette>
  )
}

/** One work, with the stage it has reached beside it. */
function Row({ row }: { row: ScoredWork }) {
  const profile = useProfile()
  const covers = useCovers()
  const stop = stageAt(profile.config, row.stage)

  return (
    <CommandPaletteItem
      value={row}
      className={cn(commandPaletteItemVariants(), 'mx-1.5 pr-3 data-[highlighted]:bg-accent-soft')}
    >
      <CommandPaletteRow
        icon={
          <span
            aria-hidden
            className="size-6 shrink-0 rounded-md"
            style={{ background: coverImageFor(row.work_id, covers.get(row.work_id)) }}
          />
        }
        hint={
          // What says "nearly out" at a glance, which is the whole reason for
          // choosing from here rather than from a list of names.
          stop === undefined ? undefined : sayLabel(stop.label)
        }
      >
        <b className="block truncate text-[13px] font-medium">{row.title}</b>
      </CommandPaletteRow>
      <StageDial percent={row.stage} stage={stop} size={14} className="shrink-0" />
    </CommandPaletteItem>
  )
}

function Chip({
  on,
  onClick,
  children,
}: {
  on: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={on}
      className={cn(
        'cursor-pointer rounded-md px-2 py-0.5 text-xs transition-colors',
        on ? 'bg-accent-soft text-accent' : 'text-dim hover:bg-soft hover:text-text',
      )}
    >
      {children}
    </button>
  )
}
