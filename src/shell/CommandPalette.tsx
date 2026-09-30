import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import type { Hit, HitKind } from '@/lib/api/types'
import { matchCommands } from '@/lib/commands'
import { coverImageFor } from '@/lib/cover'
import { cardKindsOf } from '@/lib/canon'
import { hrefOfHit } from '@/lib/hits'
import { useProfile } from '@/lib/useProfile'
import { useCovers } from '@/lib/useCovers'
import { useDebounced } from '@/lib/useDebounced'
import { queries } from '@/lib/query/queries'
import { loadRecent } from '@/lib/recent'
import { cn } from '@/lib/utils'
import {
  CommandPalette as Palette,
  CommandPaletteEmpty,
  CommandPaletteGroup,
  CommandPaletteGroupLabel,
  CommandPaletteInput,
  CommandPaletteItem,
  CommandPaletteList,
  CommandPalettePopup,
  CommandPaletteRow,
  commandPaletteItemVariants,
} from '@/components/ui/command-palette'
import { Kbd } from '@/components/ui/kbd'
import { usePaletteCommands, type Command, type ShellCommands } from '@/shell/usePaletteCommands'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  shell: ShellCommands
}

/** The order the groups of hits appear in, coarsest first. */
const GROUPS: HitKind[] = ['work', 'version', 'note', 'fact', 'comment', 'message']

/** A row of the palette: something found, or something to do. `id` is unique
 *  across both, and is what the highlight compares by. */
type Entry =
  { type: 'hit'; id: string; hit: Hit } | { type: 'command'; id: string; command: Command }

/** One captioned group of rows, in the shape Base UI reads a grouped list in. */
interface Group {
  id: string
  label: string
  items: Entry[]
}

const hitEntry = (hit: Hit): Entry => ({
  type: 'hit',
  id: `hit-${hit.kind}-${hit.entity_id}-${hit.rank}`,
  hit,
})

const commandEntry = (command: Command): Entry => ({
  type: 'command',
  id: `command-${command.id}`,
  command,
})

/**
 * One box that finds anything, and does the few things the shell does.
 *
 * What it finds is for recognising something you already know exists - a song
 * whose title you half remember, a line you wrote last month - so it shows a
 * few hits per kind rather than everything, and every hit opens where it lives
 * (`hrefOfHit`). Under the hits are two groups of its own, the mockup's:
 * Actions - a new work, the theme, the menu, the shortcuts, and for an open
 * work its score and its release - and Screens, every door of the rail with
 * the `g` chord that opens it. Opened on nothing, the palette is a list of
 * what can be done; typed into, the same list narrows.
 */
export function CommandPalette({ open, onOpenChange, shell }: Props) {
  // Mounted only while open, so every visit starts from an empty box: the
  // palette is somewhere you pass through, not somewhere you return to with
  // your last query still in it.
  return open ? <Contents onOpenChange={onOpenChange} shell={shell} /> : null
}

function Contents({
  onOpenChange,
  shell,
}: {
  onOpenChange: (open: boolean) => void
  shell: ShellCommands
}) {
  const covers = useCovers()
  const { t } = useTranslation()
  // A note of a kind of card opens on the Canon screen, where it lives.
  const cardKinds = cardKindsOf(useProfile().config)
  const navigate = useNavigate()
  const [query, setQuery] = useState('')
  const { actions, screens } = usePaletteCommands(shell)

  // The query trails the keystrokes: typing is faster than SQLite is slow, but
  // not by much once a workspace has a few hundred bodies in it. The actions
  // and screens are matched against the same settled query, so the list
  // changes once per pause rather than twice.
  const settled = useDebounced(query, 140)
  const empty = settled.trim() === ''

  const hits = useQuery({ ...queries.search(settled), enabled: !empty })

  // What the empty box offers first: the works opened lately. Read once, when
  // the palette mounts, so the list cannot shuffle under the cursor while it
  // is being read - the palette is mounted only while open, so every visit is
  // a fresh read anyway.
  const recent = useMemo(() => loadRecent(), [])

  const found = useMemo<Group[]>(() => {
    // The thing being worked on is nearly always one of the last few opened,
    // and recognising a name is faster than typing it.
    //
    // Recent works are shaped as hits rather than drawn separately so that
    // the arrows, Enter, the highlight and the opening are the ones the
    // palette already has. A second list beside the first would need all of
    // it again.
    if (empty) {
      if (recent.length === 0) return []
      return [
        {
          id: 'recent',
          label: t('search.group.recent'),
          items: recent.map((entry, at) =>
            hitEntry({
              kind: 'work',
              entity_id: entry.id,
              work_id: entry.id,
              work_title: entry.title,
              title: entry.title,
              detail: '',
              rank: recent.length - at,
            }),
          ),
        },
      ]
    }

    // Hits arrive grouped by kind already; this fixes the order they are
    // shown in, which is also the order the arrow keys walk.
    const data = hits.data ?? []
    return GROUPS.map((kind) => ({
      id: kind,
      label: t(`search.group.${kind}`),
      items: data.filter((hit) => hit.kind === kind).map(hitEntry),
    })).filter((group) => group.items.length > 0)
  }, [empty, hits.data, recent, t])

  const groups: Group[] = [
    ...found,
    {
      id: 'actions',
      label: t('search.group.action'),
      items: matchCommands(settled, actions).map(commandEntry),
    },
    {
      id: 'screens',
      label: t('search.group.screen'),
      items: matchCommands(settled, screens).map(commandEntry),
    },
  ].filter((group) => group.items.length > 0)

  const run = (entry: Entry) => {
    onOpenChange(false)
    if (entry.type === 'hit') navigate(hrefOfHit(entry.hit, cardKinds))
    else entry.command.run()
  }

  // What is shown while there is nothing to show: the wait, then the miss.
  // One line, because `Empty` must stay mounted for its announcement to fire
  // rather than be swapped in and out. An empty box always has actions and
  // screens to list, so it never gets here.
  const nothing = hits.isPending ? t('search.searching') : t('search.nothing', { query: settled })

  return (
    <Palette
      items={groups}
      // The searching is the server's, and the matching of the actions and
      // screens is `matchCommands`. Base UI would otherwise filter the rows a
      // second time against the same query, by label alone - and drop every
      // hit that matched on a body it cannot see.
      filter={null}
      open
      onOpenChange={(next) => {
        if (!next) onOpenChange(false)
      }}
      onValueChange={(entry: Entry | null) => {
        if (entry !== null) run(entry)
      }}
      // A hit is an object, and two fetches never return the same one twice;
      // without this the highlight would compare by reference and never match.
      isItemEqualToValue={(a: Entry, b: Entry) => a.id === b.id}
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
              <CommandPaletteGroup key={group.id} items={group.items}>
                <CommandPaletteGroupLabel className="caption px-3.5 pt-2 pb-1">
                  {group.label}
                </CommandPaletteGroupLabel>

                {/* Mapped by hand rather than through a second List: a List
                    is the `role="listbox"`, and there is one of those per
                    palette. Each row's place in the walk order is resolved by
                    the component from `value` and `isItemEqualToValue`. */}
                {group.items.map((entry) => (
                  <CommandPaletteItem
                    key={entry.id}
                    value={entry}
                    className={cn(
                      commandPaletteItemVariants(),
                      'mx-1.5 pr-3 data-[highlighted]:bg-accent-soft',
                    )}
                  >
                    {entry.type === 'hit' ? (
                      <HitRow hit={entry.hit} cover={covers.get(entry.hit.work_id ?? '')} />
                    ) : (
                      <CommandRow command={entry.command} />
                    )}
                  </CommandPaletteItem>
                ))}
              </CommandPaletteGroup>
            )}
          </CommandPaletteList>
        </div>

        <div className="flex gap-3.5 border-t border-line px-4 py-2 font-mono text-xs text-faint">
          <span>{t('search.navigate')}</span>
          <span>{t('search.openHit')}</span>
          <span>{t('search.close')}</span>
        </div>
      </CommandPalettePopup>
    </Palette>
  )
}

/** A thing found: its work's cover, what it is, and where it came from. */
function HitRow({ hit, cover }: { hit: Hit; cover: string | undefined }) {
  return (
    <CommandPaletteRow
      icon={
        <span
          aria-hidden
          className="size-6 shrink-0 rounded-md"
          style={{
            background: coverImageFor(
              hit.work_id ?? hit.entity_id,
              hit.work_id === null ? undefined : cover,
            ),
          }}
        />
      }
      hint={hit.detail}
    >
      <span className="block truncate text-sm">{hit.title}</span>
      {/* Which work it came from matters most for a hit that is a line of
          text rather than a title. */}
      {hit.kind !== 'work' && hit.work_title !== '' && (
        <span className="block truncate text-xs text-faint">{hit.work_title}</span>
      )}
    </CommandPaletteRow>
  )
}

/** A thing to do: its glyph where a hit has its cover, and its keys. */
function CommandRow({ command }: { command: Command }) {
  const Icon = command.icon
  return (
    <CommandPaletteRow
      icon={
        <span aria-hidden className="grid size-6 shrink-0 place-items-center text-faint">
          <Icon className="size-4" />
        </span>
      }
      hint={command.keys === undefined ? undefined : <Kbd keys={command.keys} aria-hidden />}
    >
      <span className="block truncate text-sm">{command.label}</span>
    </CommandPaletteRow>
  )
}
