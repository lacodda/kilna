import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Plus, X } from 'lucide-react'
import { moveTo } from '@/lib/collections'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { ReorderGrip, ReorderIndicator, useReorder } from '@/components/ui/reorderable-list'

interface Keyed {
  key: string
}

/** A key as the profile takes one: lower case, a letter first, then letters,
 *  digits, dashes and underscores. */
const KEY_SHAPE = /^[a-z][a-z0-9_-]*$/

interface KeyedListProps<T extends Keyed> {
  label: string
  /** What the list is for, under it. */
  help?: ReactNode
  entries: T[]
  onChange: (entries: T[]) => void
  /** What a row holds beside its key: the word, and whatever else the entry
   *  says. `set` merges changes into the entry. */
  children: (entry: T, set: (changes: Partial<T>) => void, index: number) => ReactNode
  /** What is drawn under a row, across its width: the answers of a choice,
   *  the marks of a rubric. */
  below?: (entry: T, set: (changes: Partial<T>) => void, index: number) => ReactNode
  /** A new entry under a key. Absent, the list is not added to. */
  make?: (key: string) => T
  /** What the button that adds says. */
  addLabel?: string
  /** Whether rows can be taken out. */
  removable?: boolean
  /** Whether rows can be moved. Not where the order is not the profile's to
   *  say - the stops of the dial stand by their place on it - and a grip
   *  there moved a row that the next reading put back (v0.166). */
  movable?: boolean
  /** What the cross of a row says, by the row's key. */
  removeLabel?: (key: string) => string
  /** Keys taken elsewhere, which a new entry may not repeat. */
  taken?: readonly string[]
  /** What is wrong with a row, by its key, said under it. */
  problems?: (key: string) => readonly string[]
  /** Rows as cards with a border, for entries with several lines of fields. */
  boxed?: boolean
}

/**
 * A list of the profile's entries that name themselves - kinds, axes, tiers,
 * statuses, roles, fields - edited whole (v0.112): each row dragged to its
 * place or moved with Alt and an arrow, taken out, and a new one added under
 * a key.
 *
 * The key is typed once and then stands: scores, versions, releases and works
 * are stored under these keys, and a key renamed here would orphan what was
 * written under it. What a row means beyond its key is the caller's to draw.
 */
export function KeyedList<T extends Keyed>({
  label,
  help,
  entries,
  onChange,
  children,
  below,
  make,
  addLabel,
  removable = true,
  movable = true,
  removeLabel,
  taken = [],
  problems,
  boxed = false,
}: KeyedListProps<T>) {
  const { t } = useTranslation()
  const [newKey, setNewKey] = useState('')

  const order = entries.map((entry) => entry.key)
  const reorder = useReorder<string>({
    order,
    onMove: (key, to) => {
      const next = moveTo(order, key, to)
      onChange(next.flatMap((moved) => entries.filter((entry) => entry.key === moved)))
    },
  })

  const set = (index: number) => (changes: Partial<T>) => {
    onChange(entries.map((entry, i) => (i === index ? { ...entry, ...changes } : entry)))
  }

  const key = newKey.trim().toLowerCase()
  const keyTaken = order.includes(key) || taken.includes(key)
  const keyValid = KEY_SHAPE.test(key) && !keyTaken

  const add = () => {
    if (make === undefined || !keyValid) return
    onChange([...entries, make(key)])
    setNewKey('')
  }

  return (
    <FieldGroup label={label} help={help}>
      <ul
        {...reorder.listProps}
        className={cn('relative flex flex-col', boxed ? 'gap-3' : 'gap-1.5')}
      >
        {entries.map((entry, index) => {
          const wrong = problems?.(entry.key) ?? []
          return (
            <li
              key={entry.key}
              {...(movable ? reorder.rowProps(entry.key) : {})}
              title={movable ? t('editor.moveHint') : undefined}
              className={cn(
                'flex flex-col gap-1.5',
                boxed && 'rounded-xl border border-line p-3',
                reorder.dragging === entry.key && 'opacity-50',
              )}
            >
              <div className="flex items-center gap-2">
                {movable && (
                  <ReorderGrip
                    {...reorder.gripProps(entry.key)}
                    title={t('editor.move', { key: entry.key })}
                  />
                )}
                <code
                  className="w-28 shrink-0 truncate font-mono text-xs text-dim"
                  title={entry.key}
                >
                  {entry.key}
                </code>
                {children(entry, set(index), index)}
                {removable && (
                  <Button
                    variant="danger"
                    size="icon-sm"
                    title={removeLabel?.(entry.key) ?? t('editor.remove', { key: entry.key })}
                    aria-label={removeLabel?.(entry.key) ?? t('editor.remove', { key: entry.key })}
                    onClick={() => onChange(entries.filter((_, i) => i !== index))}
                  >
                    <X aria-hidden className="size-3.5" />
                  </Button>
                )}
              </div>
              {below?.(entry, set(index), index)}
              {wrong.map((problem) => (
                <p key={problem} role="alert" className="text-xs text-bad">
                  {problem}
                </p>
              ))}
            </li>
          )
        })}
        <ReorderIndicator offset={reorder.slotOffset} />
      </ul>
      {make !== undefined && (
        <form
          className="flex items-center gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            // A list inside a form of its own: the key box must not submit
            // a dialog the editor may one day stand in.
            event.stopPropagation()
            add()
          }}
        >
          <Input
            className="w-48 font-mono text-xs"
            value={newKey}
            placeholder={t('editor.keyPlaceholder')}
            aria-label={t('editor.keyOf', { list: label })}
            onChange={(event) => setNewKey(event.target.value)}
          />
          <Button size="sm" type="submit" disabled={!keyValid}>
            <Plus aria-hidden />
            {addLabel ?? t('editor.add')}
          </Button>
          {key !== '' && !keyValid && (
            <span className="text-xs text-bad">
              {t(keyTaken ? 'editor.keyTaken' : 'editor.keyShape')}
            </span>
          )}
        </form>
      )}
    </FieldGroup>
  )
}
