import { useState, type PointerEvent } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { ArrowUpRight, Ellipsis, FolderInput, Link2, X } from 'lucide-react'
import type { LineRow, NotePatch, NoteState, ShelfView } from '@/lib/api/types'
import { sungOf } from '@/lib/lines'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Menu,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuPopup,
  MenuSeparator,
  MenuTrigger,
} from '@/components/ui/menu'
import { ReorderGrip } from '@/components/ui/reorderable-list'

interface Props {
  line: LineRow
  /** The shelf on show, when the list is one: the phrase can be taken off it. */
  shelf: ShelfView | null
  /** Every shelf, for where the phrase stands and where it can go. */
  shelves: readonly ShelfView[]
  /** The title of the work it is kept for, when it is kept for one. */
  work: string | null
  /** It is the phrase being carried to a shelf. */
  carrying: boolean
  /** The pointer went down on its grip: the start of carrying it, if it moves. */
  onGrab: (event: PointerEvent<HTMLElement>) => void
  onPatch: (change: NotePatch) => void
  onAttach: () => void
  onPromote: () => void
  onPut: (shelfId: string) => void
  onTakeOff: () => void
  onDelete: () => void
}

/** Which of the two fields of the row is open for writing, if either. */
type Editing = 'line' | 'whose' | null

/**
 * One phrase: its words, whose words they are, how they are sung, the shelves
 * it stands on besides the one on show, its tags, how many works already say
 * it, the work it is kept for, and what can be done to it (ADR 0064).
 *
 * "Sung in" is read off the works' current texts the way the register counts
 * a term (ADR 0044) - never stored - so a song rewritten a minute ago already
 * counts. The phrase is carried to a shelf by its grip; the menu behind the
 * folder is the same act for a keyboard.
 */
export function LineItem({
  line,
  shelf,
  shelves,
  work,
  carrying,
  onGrab,
  onPatch,
  onAttach,
  onPromote,
  onPut,
  onTakeOff,
  onDelete,
}: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [editing, setEditing] = useState<Editing>(null)
  const [text, setText] = useState(line.body)
  const [whose, setWhose] = useState(line.attribution ?? '')

  const save = () => {
    setEditing(null)
    const next = text.trim()
    if (next !== '' && next !== line.body) onPatch({ body: next })
    else setText(line.body)
  }
  const saveWhose = () => {
    setEditing(null)
    const next = whose.trim()
    if (next !== (line.attribution ?? '')) onPatch({ attribution: next === '' ? null : next })
    else setWhose(line.attribution ?? '')
  }

  const elsewhere = shelves
    .filter((one) => one.id !== shelf?.id && line.shelf_ids.includes(one.id))
    .map((one) => one.name)
  const free = shelves.filter((one) => !line.shelf_ids.includes(one.id))
  const sung = sungOf(line)
  // The ways out of the state it is in: never the one it is already in.
  const moves: { state: NoteState; label: string }[] = [
    { state: 'used' as const, label: t('notes.use') },
    { state: 'parked' as const, label: t('notes.park') },
    { state: 'dropped' as const, label: t('notes.drop') },
    { state: 'fresh' as const, label: t('notes.refresh') },
  ].filter((move) => move.state !== line.state)

  return (
    <li
      className={cn(
        'group flex min-w-0 items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-soft',
        line.state !== 'fresh' && 'text-dim',
        carrying && 'opacity-50',
      )}
    >
      <ReorderGrip onPointerDown={onGrab} title={t('shelves.carry')} />
      {editing === 'line' ? (
        <Input
          autoFocus
          value={text}
          onChange={(event) => setText(event.target.value)}
          onBlur={save}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
            if (event.key === 'Escape') {
              setText(line.body)
              setEditing(null)
            }
          }}
          aria-label={t('notes.lineEdit')}
          className="min-w-0 flex-1"
        />
      ) : (
        // The words are to be read and copied: a double-click - or the menu -
        // makes them a field.
        <span
          className="selectable max-w-[60%] min-w-0 shrink truncate"
          title={line.body}
          onDoubleClick={() => setEditing('line')}
        >
          {line.body}
        </span>
      )}
      {editing === 'whose' ? (
        <Input
          autoFocus
          value={whose}
          onChange={(event) => setWhose(event.target.value)}
          onBlur={saveWhose}
          onKeyDown={(event) => {
            if (event.key === 'Enter') event.currentTarget.blur()
            if (event.key === 'Escape') {
              setWhose(line.attribution ?? '')
              setEditing(null)
            }
          }}
          placeholder={t('notes.whosePlaceholder')}
          aria-label={t('notes.whose')}
          className="w-48 shrink-0"
        />
      ) : (
        line.attribution !== null && (
          <span
            className="selectable max-w-48 min-w-0 shrink truncate text-xs text-dim italic"
            title={`${t('notes.whose')}: ${line.attribution}`}
            onDoubleClick={() => setEditing('whose')}
          >
            — {line.attribution}
          </span>
        )
      )}
      {sung !== '' && (
        <span
          className="max-w-40 min-w-0 shrink truncate font-mono text-xs text-dim"
          title={`${t('notes.sungForms')}: ${line.sung
            .map((one) => `${one.written} → ${one.sung}`)
            .join(', ')}`}
        >
          {sung}
        </span>
      )}
      <span className="min-w-0 flex-1 truncate text-xs text-faint">
        {[
          elsewhere.length > 0
            ? t('shelves.on', { count: elsewhere.length, shelves: elsewhere.join(', ') })
            : null,
          line.tags.length > 0 ? line.tags.join(', ') : null,
        ]
          .filter((part) => part !== null)
          .join(' · ')}
      </span>
      {line.state !== 'fresh' && (
        <span className="shrink-0 text-xs text-faint">{t(`notes.state.${line.state}`)}</span>
      )}
      {line.uses > 0 && (
        <span className="shrink-0 text-xs text-faint tabular-nums" title={t('notes.sungInHint')}>
          {t('notes.sungIn', { count: line.uses })}
        </span>
      )}
      {work !== null && line.work_id !== null && (
        <Button
          variant="link"
          className="max-w-48 min-w-0 shrink-0 text-xs"
          title={t('notes.keptFor', { title: work })}
          onClick={() => void navigate(`/works/${line.work_id ?? ''}`)}
        >
          <span className="truncate">{work}</span>
          <ArrowUpRight aria-hidden />
        </Button>
      )}
      <Button
        size="icon-xs"
        variant="icon"
        title={t('notes.toWork')}
        aria-label={t('notes.toWork')}
        onClick={onAttach}
      >
        <Link2 aria-hidden />
      </Button>
      <Menu>
        <MenuTrigger
          render={
            <Button
              size="icon-xs"
              variant="icon"
              title={t('shelves.putOn')}
              aria-label={t('shelves.putOn')}
              disabled={shelves.length === 0}
            />
          }
        >
          <FolderInput aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          <MenuGroup>
            <MenuGroupLabel>{t('shelves.putOn')}</MenuGroupLabel>
            {free.length === 0 ? (
              <MenuItem disabled>{t('shelves.putNowhere')}</MenuItem>
            ) : (
              free.map((one) => (
                <MenuItem key={one.id} onClick={() => onPut(one.id)}>
                  {one.name}
                </MenuItem>
              ))
            )}
          </MenuGroup>
        </MenuPopup>
      </Menu>
      {shelf !== null && (
        <Button
          size="icon-xs"
          variant="icon"
          title={t('shelves.takeOff')}
          aria-label={t('shelves.takeOff')}
          onClick={onTakeOff}
        >
          <X aria-hidden />
        </Button>
      )}
      <Menu>
        <MenuTrigger
          render={
            <Button
              size="icon-xs"
              variant="icon"
              title={t('notes.lineActions')}
              aria-label={t('notes.lineActions')}
            />
          }
        >
          <Ellipsis aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          <MenuItem onClick={() => setEditing('line')}>{t('notes.lineEdit')}</MenuItem>
          <MenuItem onClick={() => setEditing('whose')}>{t('notes.whoseEdit')}</MenuItem>
          <MenuItem onClick={onAttach}>{t('notes.toWork')}</MenuItem>
          {line.work_id !== null && (
            <MenuItem onClick={() => onPatch({ work_id: null })}>{t('notes.detach')}</MenuItem>
          )}
          <MenuItem onClick={onPromote}>{t('notes.promote')}</MenuItem>
          <MenuSeparator />
          {moves.map((move) => (
            <MenuItem key={move.state} onClick={() => onPatch({ state: move.state })}>
              {move.label}
            </MenuItem>
          ))}
          <MenuSeparator />
          <MenuItem tone="danger" onClick={onDelete}>
            {t('notes.delete')}
          </MenuItem>
        </MenuPopup>
      </Menu>
    </li>
  )
}
