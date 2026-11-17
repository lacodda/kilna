import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { CircleCheck, File, Film, Image as ImageIcon, Music, Play } from 'lucide-react'
import type { FileCategory, FileEntry, FileGroup, FileRole, FileView } from '@/lib/fileList'
import { formatBytes, formatMoment } from '@/lib/format'
import { extensionOf } from '@/lib/media'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { TileButton } from '@/components/ui/tile-button'
import { MediaPreview } from '@/components/MediaPreview'

interface Props {
  groups: FileGroup[]
  view: FileView
  /** What a group is called. */
  headingOf: (group: FileGroup) => string
  /** A heading that is a path - a folder's name - is set in the mono face. */
  pathHeadings?: boolean
  onOpen: (entry: FileEntry) => void
  /** What opening a file does, said for the control that does it. */
  openLabel: (entry: FileEntry) => string
  /** What else can be done with a file: the buttons beside it. */
  actions?: (entry: FileEntry) => ReactNode
}

const CATEGORY_ICON: Record<FileCategory, typeof File> = {
  image: ImageIcon,
  audio: Music,
  video: Film,
  other: File,
}

/**
 * A work's files, in groups, drawn one of four ways (FR-14): large icons,
 * tiles, a list, or a table with the size and the date.
 *
 * One drawing for the files attached to the work and the files in its folder
 * on disk: what differs between them - what opening one does, what else can
 * be done with it - is handed in, and how a file looks is not.
 */
export function FileEntries({
  groups,
  view,
  headingOf,
  pathHeadings = false,
  onOpen,
  openLabel,
  actions,
}: Props) {
  return (
    <div className="flex flex-col gap-3">
      {groups.map((group) => (
        <section key={group.key} className="flex flex-col gap-2">
          <h3
            className={cn(
              'caption',
              pathHeadings && group.by === 'place' && 'font-mono normal-case',
            )}
          >
            {headingOf(group)} · {group.entries.length}
          </h3>
          {view === 'table' ? (
            <EntryTable
              entries={group.entries}
              onOpen={onOpen}
              openLabel={openLabel}
              actions={actions}
            />
          ) : view === 'list' ? (
            <ul className="flex flex-col">
              {group.entries.map((entry) => (
                <EntryRow
                  key={entry.key}
                  entry={entry}
                  onOpen={onOpen}
                  openLabel={openLabel}
                  actions={actions}
                />
              ))}
            </ul>
          ) : (
            <ul
              className={cn(
                'grid gap-2.5',
                view === 'large'
                  ? 'grid-cols-[repeat(auto-fill,minmax(240px,1fr))]'
                  : 'grid-cols-[repeat(auto-fill,minmax(168px,1fr))]',
              )}
            >
              {group.entries.map((entry) => (
                <EntryTile
                  key={entry.key}
                  entry={entry}
                  large={view === 'large'}
                  onOpen={onOpen}
                  openLabel={openLabel}
                  actions={actions}
                />
              ))}
            </ul>
          )}
        </section>
      ))}
    </div>
  )
}

type One = Pick<Props, 'onOpen' | 'openLabel' | 'actions'> & { entry: FileEntry }

/**
 * A file as a tile: what it looks like, what it is called, and what can be
 * done with it - shown when the pointer is over the tile or the keyboard
 * reaches it, rather than standing on every picture of a board.
 *
 * The picture is shown whole rather than cropped to fill: a cover may be
 * square, a reference a tall screenshot, and a gallery that crops both to
 * one rectangle shows neither.
 */
function EntryTile({ entry, large, onOpen, openLabel, actions }: One & { large: boolean }) {
  const { t } = useTranslation()
  return (
    <li className="group relative overflow-hidden rounded-md border border-line bg-raise">
      {/* The card around it draws the edge and the corners, so the tile
          keeps neither; the focus ring is drawn inward to stay inside the
          card's clipping. */}
      <TileButton
        className={cn(
          'w-full items-center justify-center rounded-none border-0 bg-bg hover:bg-soft focus-visible:-outline-offset-2',
          large ? 'h-44' : 'h-26',
        )}
        title={openLabel(entry)}
        aria-label={openLabel(entry)}
        onClick={() => onOpen(entry)}
      >
        {entry.category === 'audio' ? (
          // A sound has nothing to look at: the way to hear it, and its kind.
          <span className="flex flex-col items-center gap-1.5 font-mono text-xs tracking-caption text-faint uppercase">
            <Play aria-hidden className="size-8 rounded-full bg-accent p-2 text-on-accent" />
            {extensionOf(entry.name)}
          </span>
        ) : (
          <MediaPreview
            path={entry.path}
            alt={entry.name}
            className="max-h-full max-w-full object-contain"
          />
        )}
      </TileButton>
      <p className="flex items-center gap-1 px-2 py-1.5 text-xs" title={entry.name}>
        {entry.roles.includes('finished') && (
          <CircleCheck
            aria-label={t('files.role.finished')}
            className="size-3.5 shrink-0 text-good"
          />
        )}
        <span className="truncate">{entry.name}</span>
        {large && (
          <span className="ml-auto shrink-0 font-mono text-2xs text-faint">
            {formatBytes(entry.size)}
          </span>
        )}
      </p>
      <span className="absolute top-1.5 left-1.5 flex gap-1">
        {entry.roles
          .filter((role) => role !== 'finished')
          .map((role) => (
            <RoleTag key={role} role={role} />
          ))}
      </span>
      {actions !== undefined && (
        <span
          className={cn(
            'absolute top-1.5 right-1.5 flex gap-1',
            'opacity-0 group-hover:opacity-100 focus-within:opacity-100',
          )}
        >
          {actions(entry)}
        </span>
      )}
    </li>
  )
}

/** A file as a line of a list: its kind, its name, what it is, its size. */
function EntryRow({ entry, onOpen, openLabel, actions }: One) {
  const Icon = CATEGORY_ICON[entry.category]
  return (
    <li className="group flex min-w-0 items-center gap-2 rounded-md px-1.5 py-0.5 hover:bg-soft">
      <Icon aria-hidden className="size-3.5 shrink-0 text-faint" />
      <Button
        variant="link"
        className="min-w-0 justify-start truncate text-sm text-text"
        title={openLabel(entry)}
        onClick={() => onOpen(entry)}
      >
        <span className="truncate">{entry.name}</span>
      </Button>
      <Roles roles={entry.roles} />
      <span className="ml-auto flex shrink-0 items-center gap-2">
        {actions !== undefined && (
          <span className="flex gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100">
            {actions(entry)}
          </span>
        )}
        <span className="w-16 text-right font-mono text-xs text-faint tabular-nums">
          {formatBytes(entry.size)}
        </span>
      </span>
    </li>
  )
}

/** The files as a table: name, type, role, size, when it last changed. */
function EntryTable({
  entries,
  onOpen,
  openLabel,
  actions,
}: Pick<Props, 'onOpen' | 'openLabel' | 'actions'> & { entries: FileEntry[] }) {
  const { t } = useTranslation()
  return (
    // Fixed widths: each group is a table of its own, and columns sized by
    // what each happens to hold would not line up from one group to the next.
    <Table className="table-fixed">
      <colgroup>
        <col />
        <col className="w-44" />
        <col className="w-36" />
        <col className="w-24" />
        <col className="w-32" />
        {actions !== undefined && <col className="w-24" />}
      </colgroup>
      <TableHead>
        <TableRow>
          <TableHeader>{t('files.column.name')}</TableHeader>
          <TableHeader>{t('files.column.type')}</TableHeader>
          <TableHeader>{t('files.column.role')}</TableHeader>
          <TableHeader numeric>{t('files.column.size')}</TableHeader>
          <TableHeader>{t('files.column.modified')}</TableHeader>
          {actions !== undefined && (
            <TableHeader>
              <span className="sr-only">{t('files.column.actions')}</span>
            </TableHeader>
          )}
        </TableRow>
      </TableHead>
      <TableBody>
        {entries.map((entry) => {
          const Icon = CATEGORY_ICON[entry.category]
          const ending = extensionOf(entry.name)
          return (
            <TableRow key={entry.key} className="group">
              <TableCell className="min-w-0">
                <span className="flex min-w-0 items-center gap-2">
                  <Icon aria-hidden className="size-3.5 shrink-0 text-faint" />
                  <Button
                    variant="link"
                    className="min-w-0 justify-start truncate text-sm text-text"
                    title={openLabel(entry)}
                    onClick={() => onOpen(entry)}
                  >
                    <span className="truncate">{entry.name}</span>
                  </Button>
                </span>
              </TableCell>
              <TableCell className="whitespace-nowrap text-dim">
                {t(`files.category.${entry.category}`)}
                {ending !== '' && (
                  <span className="ml-1.5 font-mono text-xs text-faint uppercase">{ending}</span>
                )}
              </TableCell>
              <TableCell className="whitespace-nowrap">
                <Roles roles={entry.roles} />
              </TableCell>
              <TableCell numeric className="whitespace-nowrap text-dim">
                {formatBytes(entry.size)}
              </TableCell>
              <TableCell className="whitespace-nowrap text-dim">
                {entry.modified === undefined ? '' : formatMoment(entry.modified)}
              </TableCell>
              {actions !== undefined && (
                <TableCell className="whitespace-nowrap">
                  <span className="flex justify-end gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100">
                    {actions(entry)}
                  </span>
                </TableCell>
              )}
            </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}

/** What a file is to its work, in words: for a line and a table's cell. */
function Roles({ roles }: { roles: FileRole[] }) {
  if (roles.length === 0) return null
  return (
    <span className="flex shrink-0 gap-1">
      {roles.map((role) => (
        <RoleTag key={role} role={role} />
      ))}
    </span>
  )
}

/** One role as a tag. The cover's is the accent's - it is what the work
 *  wears; the finished file's is the colour of something done. */
function RoleTag({ role }: { role: FileRole }) {
  const { t } = useTranslation()
  return (
    <span
      className={cn(
        'rounded-xs px-1.5 text-2xs font-semibold tracking-wide',
        role === 'cover' && 'bg-accent text-on-accent',
        role === 'candidate' && 'bg-soft text-dim',
        role === 'finished' && 'bg-good/15 text-good',
      )}
    >
      {t(`files.role.${role}`)}
    </span>
  )
}
