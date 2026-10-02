import { useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronRight, TextCursor, WandSparkles } from 'lucide-react'
import type { StressNote } from '@/lib/api/types'
import { groupNotes, hasOneAnswer, stressMark, type NoteGroup } from '@/lib/stress'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'

interface Props {
  notes: readonly StressNote[]
  /** Write `option` over the word at each of the notes' places. */
  onAnswer: (notes: readonly StressNote[], option: string) => void
  /** Write every note that has one answer. */
  onAnswerAll: () => void
  /** Put the caret on the note's word, in the editor. */
  onFind: (note: StressNote) => void
  /** Where the focus goes when a menu closes: the editor, while writing -
   *  the chip that opened it would leave the caret in a box without focus. */
  returnTo: RefObject<HTMLTextAreaElement | null>
}

/**
 * What a sung text says about its stresses (ADR 0053), under the register's
 * strip: each word the singer may get wrong, a chip in the colour of its
 * mark in the text. A press offers the answers - a reading of a homograph,
 * the ё spelling, the dictionary's stress, the owner's own respelling - and
 * one choice writes it over exactly that word. A word the dictionary does not
 * know has no answer to offer, only the way to mark it.
 *
 * Homographs are most of what the check finds - about ten a song across the
 * owner's 248 ("один", "уже", "воды"), against one missing ё a song and a
 * mark against the dictionary in one song of eight. A chip each would bury
 * the few that are almost always a real mistake under the many that are
 * usually read right. So those come first, in the order of the text, and the
 * homographs fold into one chip that opens on a press.
 *
 * The answers with nothing to choose between go in at once with "Apply all";
 * a homograph never does - which reading is meant is the owner's to say.
 */
export function StressStrip({ notes, onAnswer, onAnswerAll, onFind, returnTo }: Props) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const ready = notes.filter(hasOneAnswer).length
  const groups = groupNotes(notes)
  const homographs = groups.filter((group) => group.notes[0]!.kind === 'homograph')
  const rest = groups.filter((group) => group.notes[0]!.kind !== 'homograph')
  const chip = (group: NoteGroup) => (
    // A word can carry two notes (a missing ё and a stress against the
    // dictionary): the kind, in the key, tells them apart.
    <NoteChip
      key={group.key}
      group={group}
      onAnswer={onAnswer}
      onFind={onFind}
      returnTo={returnTo}
    />
  )

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-1.5 border-b border-line px-3 py-1.5 text-xs text-dim">
      <span className="mr-1 font-medium">{t('versions.stress.title')}</span>
      {rest.map(chip)}
      {homographs.length > 0 && (
        <Button
          variant={open ? 'soft' : 'ghost'}
          size="xs"
          aria-expanded={open}
          title={t('versions.stress.homographsHint')}
          onClick={() => setOpen((was) => !was)}
        >
          {open ? <ChevronDown aria-hidden /> : <ChevronRight aria-hidden />}
          <span className={stressMark('homograph')}>
            {t('versions.stress.homographs', { count: homographs.length })}
          </span>
        </Button>
      )}
      {open && homographs.map(chip)}
      {ready > 0 && (
        <Button
          variant="ghost"
          size="xs"
          className="ml-auto"
          title={t('versions.stress.applyAllHint')}
          onClick={onAnswerAll}
        >
          <WandSparkles aria-hidden />
          {t('versions.stress.applyAll', { count: ready })}
        </Button>
      )}
    </div>
  )
}

/** One word of the strip, and the menu of its answers. */
function NoteChip({
  group,
  onAnswer,
  onFind,
  returnTo,
}: Pick<Props, 'onAnswer' | 'onFind' | 'returnTo'> & { group: NoteGroup }) {
  const { t } = useTranslation()
  // "Find" walks the places the text says the word, one a press.
  const [next, setNext] = useState(0)
  const note = group.notes[0]!
  const count = group.notes.length
  // Back to the editor when it is there; to the chip otherwise, as a menu does.
  const finalFocus = () => returnTo.current ?? true
  const find = () => {
    onFind(group.notes[next % count]!)
    setNext((was) => (was + 1) % count)
  }

  return (
    <Menu>
      <MenuTrigger
        render={
          <Button variant="ghost" size="xs" title={t(`versions.stress.hints.${note.kind}`)} />
        }
      >
        <span className={stressMark(note.kind)}>{note.word}</span>
        {count > 1 && <span className="font-mono text-faint">×{count}</span>}
        <span className="text-faint">{t(`versions.stress.kinds.${note.kind}`)}</span>
      </MenuTrigger>
      <MenuPopup align="start" finalFocus={finalFocus}>
        <p className="max-w-64 px-2 py-1.5 text-xs text-faint">
          {t(`versions.stress.hints.${note.kind}`)}
        </p>
        {note.options.map((option) => (
          <MenuItem key={option} onClick={() => onAnswer(group.notes, option)}>
            <span className="font-mono">{option}</span>
          </MenuItem>
        ))}
        <MenuSeparator />
        <MenuItem onClick={find}>
          <TextCursor aria-hidden />
          {t('versions.stress.find')}
        </MenuItem>
      </MenuPopup>
    </Menu>
  )
}
