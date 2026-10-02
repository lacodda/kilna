import {
  useLayoutEffect,
  useMemo,
  useRef,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
} from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Copy, Eye, Maximize2, MicVocal, Minimize2, PenLine, RemoveFormatting } from 'lucide-react'
import { cleanText } from '@/lib/api/register'
import type { StressNote, TextCheck } from '@/lib/api/types'
import { typing } from '@/lib/keys'
import { queries } from '@/lib/query/queries'
import { strictnessStatus, termMark } from '@/lib/register'
import {
  answerEach,
  answerAll,
  carryCaret,
  stressMark,
  toggleStress,
  vowelAt,
  vowelNear,
  withAccents,
  type Replaced,
} from '@/lib/stress'
import { say } from '@/lib/toast'
import { useTextCheck } from '@/lib/useTextCheck'
import type { useBodyEditing } from '@/lib/useBodyEditing'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { MarkedTextarea, type Mark } from '@/components/ui/marked-text'
import { SaveState } from '@/components/ui/save-state'
import { Skeleton } from '@/components/ui/skeleton'
import { StatusDot } from '@/components/ui/status-dot'
import { CompareColumn } from '@/features/work/tabs/versions/CompareColumn'
import { ReadingText } from '@/features/work/tabs/versions/ReadingText'
import { StressStrip } from '@/features/work/tabs/versions/StressStrip'
import { TextScroll } from '@/features/work/tabs/versions/TextScroll'
import { REPEAT_TINTS, textMetrics } from '@/features/work/tabs/versions/metrics'
import { charUnderPointer, revealIndex } from '@/features/work/tabs/versions/pointer'
import { useShowStresses } from '@/features/work/tabs/versions/stressView'
import { useLineDiff } from '@/features/work/tabs/versions/useLineDiff'

/**
 * How a body is shown: read, or typed into.
 *
 * `view` draws it the way its role reads - a monospace column for lyrics,
 * rendered markdown for a review. `edit` is the same text in a box, saving
 * itself into the version of the sitting. Comparing is not a third way of
 * reading but a second column beside either, so a rewrite keeps the original
 * in view.
 */
export type Reading = 'view' | 'edit'

interface Props {
  /** What the text's scroller is called, for a reader. */
  label: string
  /** The left of the bar: which version this is. */
  who: ReactNode
  /** What acts on this version, between who it is and the ways of reading
   *  it: the comparison, making it current, the profile's actions. */
  tools?: ReactNode
  markdown: boolean
  /** Null while it loads. */
  body: string | null
  reading: Reading
  onReading: (mode: Reading) => void
  /** The version standing beside this one, when one was picked. */
  against?: { label: string; body: string; onClose: () => void } | null
  /** Whether the text is checked: its repeated words marked while it is
   *  written, the register's terms marked always. */
  repeats?: boolean
  /** Whether the text is sung - its role says (`VersionRole.sung`). A sung
   *  text is checked for its stresses too, takes the stress gesture, and can
   *  be copied without the singer's marks (ADR 0053). */
  sung?: boolean
  editing: ReturnType<typeof useBodyEditing>
  /** Whether it is over the whole window rather than on the card. */
  staged: boolean
  onStage: (staged: boolean) => void
}

/**
 * One version's body in a panel: the mockup's `.edcol` - a bar across the
 * top (`.edbar`: who and when in mono on the left, what acts on it on the
 * right) and the text under it (`.lyrics`), scrolling on its own.
 *
 * Reading is the default and stays reading: a click selects, a drag selects
 * a line to copy. Writing starts when it is asked for - the pencil, or `E`
 * (or Enter on the text) - and it starts right here, in the panel, not over
 * the window. Escape puts the pen down again. The stage is its own button,
 * for reading or writing with nothing else in view.
 */
export function BodyPane({
  label,
  who,
  tools,
  markdown,
  body,
  reading,
  onReading,
  against = null,
  repeats = false,
  sung = false,
  editing,
  staged,
  onStage,
}: Props) {
  const { t } = useTranslation()
  const metrics = textMetrics(markdown)

  // What is on the left right now: the text being typed, or the body as it
  // is on disk. The comparison and the check are read off this, so both
  // follow the keystrokes.
  const text = reading === 'edit' ? editing.text : (body ?? '')
  const diff = useLineDiff(against?.body ?? null, text)
  // A markdown body is drawn by the renderer, where an offset into the text
  // names no letter on screen: it is not marked (ADR 0044) - nor is a stress
  // on it, which would have no letter to sit over either.
  const singing = sung && !markdown
  const { check, current } = useTextCheck(
    repeats && body !== null && !markdown ? text : null,
    singing,
  )
  const [accents, setAccents] = useShowStresses()
  const marks = useMemo(() => {
    if (!current || check === undefined) return []
    const runs = marksOf(check, reading === 'edit', accents)
    return singing && accents ? withAccents(runs, check.accents, text) : runs
  }, [check, current, reading, singing, accents, text])

  // The editor, for the edits made to it from outside the keyboard: the
  // stress gesture, an answer chosen in the strip. A controlled textarea
  // whose value is replaced puts its caret at the end, so where the caret
  // belongs is held here and put back once the new text is on screen.
  const box = useRef<HTMLTextAreaElement>(null)
  const placing = useRef<{ start: number; end: number } | null>(null)
  useLayoutEffect(() => {
    const field = box.current
    const pending = placing.current
    if (field === null || pending === null) return
    placing.current = null
    field.setSelectionRange(pending.start, pending.end)
    revealIndex(field, field.value, pending.start)
  }, [editing.text, reading])

  // When the text being read takes the focus: after the pen is put down by
  // the keyboard, so the next `E` lands where the last Escape did, and on
  // arriving on the stage, where the button that brought it there is gone.
  // A pen put down with the pointer leaves the focus where the pointer is.
  // The pane mounts anew on the stage, so `staged` at mount is the arrival.
  const returning = useRef(staged)
  const takeFocus = (node: HTMLDivElement | null) => {
    if (node === null || !returning.current) return
    returning.current = false
    node.focus()
  }

  const write = () => onReading('edit')
  const read = () => {
    void editing.flush()
    onReading('view')
  }

  // The stress on one vowel put on or taken off (ADR 0053), the caret kept
  // beside the letter it was at.
  const stressAt = (at: number | null, caret: number) => {
    if (at === null) return
    const edited = toggleStress(editing.text, at, caret)
    if (edited === null) return
    placing.current = { start: edited.caret, end: edited.caret }
    editing.setText(edited.text)
  }

  // Alt and a click on a vowel. The click has already put the caret beside
  // the letter; the mirror says which side of it the pointer was on, and
  // where it cannot be measured the vowel before the caret is taken first.
  const onStressClick = (event: MouseEvent<HTMLTextAreaElement>) => {
    if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return
    const field = event.currentTarget
    const caret = field.selectionStart
    if (caret !== field.selectionEnd) return
    const hit = charUnderPointer(field, editing.text, caret, event.clientX, event.clientY)
    stressAt(
      hit === null
        ? (vowelAt(editing.text, caret - 1) ?? vowelAt(editing.text, caret))
        : vowelAt(editing.text, hit),
      caret,
    )
  }

  // A sung text rewritten from the strip. In the editor, as if typed, the
  // caret carried over what changed before it. While reading, saved the way
  // the editor saves - into the revision of the sitting - and reading stays
  // reading, as it does for every press on the text that is not the pencil.
  const rewrite = (next: string, replaced: readonly Replaced[]) => {
    if (reading === 'edit') {
      const field = box.current
      if (field !== null) {
        const caret = carryCaret(field.selectionStart, replaced)
        placing.current = { start: caret, end: caret }
      }
      editing.setText(next)
      return
    }
    editing.setText(next)
    void editing.flush()
  }

  const answerNotes = (notes: readonly StressNote[], option: string) => {
    const { text: next, replaced } = answerEach(text, notes, () => option)
    if (replaced.length > 0) rewrite(next, replaced)
  }

  const answerEvery = () => {
    if (check === undefined) return
    const { text: next, replaced } = answerAll(text, check.stress)
    if (replaced.length > 0) rewrite(next, replaced)
  }

  // The word a note is about, selected in the editor - opened for it while
  // the text is read - so the vowel to mark is right there.
  const find = (note: StressNote) => {
    placing.current = { start: note.start, end: note.end }
    const field = box.current
    if (reading === 'edit' && field !== null) {
      placing.current = null
      field.setSelectionRange(note.start, note.end)
      revealIndex(field, field.value, note.start)
      return
    }
    write()
  }

  // `E`, anywhere in the panel but a field, starts writing; so does Enter,
  // on the text rather than on a button (where Enter presses the button).
  // By the key's place rather than its letter, so it is the same key on a
  // Cyrillic layout, where that key types `у`.
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (reading !== 'view' || body === null || event.defaultPrevented) return
    if (event.ctrlKey || event.metaKey || event.altKey || typing(event.target)) return
    const onControl =
      event.target instanceof Element &&
      event.target.closest('button, a, [role="menuitem"]') !== null
    if (event.code === 'KeyE' || (event.key === 'Enter' && !onControl)) {
      event.preventDefault()
      write()
    }
  }

  const content =
    body === null ? (
      <div className="px-5.5 py-4">
        <Skeleton className="h-32 w-full" />
      </div>
    ) : reading === 'edit' ? (
      <MarkedTextarea
        ref={box}
        autoFocus
        value={editing.text}
        onChange={editing.setText}
        onBlur={() => void editing.flush()}
        onClick={singing ? onStressClick : undefined}
        onKeyDown={(event) => {
          // The key everyone presses anyway. The text is already saving
          // itself; this writes it now rather than after the pause.
          if ((event.ctrlKey || event.metaKey) && event.key === 's') {
            event.preventDefault()
            void editing.flush()
          }
          // Alt+' puts the stress on the word at the caret, on the vowel just
          // typed (ADR 0053). By the key's place rather than its character:
          // on a Cyrillic layout that key types "э". Alt alone types nothing
          // on Windows, and AltGr arrives as Ctrl+Alt, which this leaves be.
          if (
            singing &&
            event.altKey &&
            !event.ctrlKey &&
            !event.metaKey &&
            !event.shiftKey &&
            event.code === 'Quote'
          ) {
            event.preventDefault()
            const caret = event.currentTarget.selectionEnd
            stressAt(vowelNear(editing.text, caret), caret)
          }
          // Escape puts the pen down, the way it leaves every editor here -
          // and only that: a stage around the text is the next press.
          if (event.key === 'Escape') {
            event.preventDefault()
            event.stopPropagation()
            returning.current = true
            read()
          }
        }}
        aria-label={t('versions.edit')}
        marks={marks}
        lineMarks={diff.added}
        className={cn('block w-full', metrics)}
      />
    ) : (
      <ReadingText
        ref={takeFocus}
        body={body}
        markdown={markdown}
        added={diff.added}
        marks={marks}
        metrics={metrics}
      />
    )

  const modes: { mode: Reading; icon: typeof Eye; label: string }[] = [
    { mode: 'view', icon: Eye, label: t('versions.view') },
    { mode: 'edit', icon: PenLine, label: t('versions.edit') },
  ]

  return (
    <section
      onKeyDown={onKeyDown}
      // A container, so the bar can say its words at the width it is given
      // rather than at the window's: beside a review the pane is half as wide,
      // and the named buttons fold to their marks there (`@max-2xl`).
      className="@container flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise"
    >
      {/* Who and when take what the buttons leave, down to a floor; below it
          the buttons wrap under them, flush right, rather than being cut off
          at the pane's edge. */}
      <div className="flex shrink-0 flex-wrap items-center justify-end gap-x-1.5 gap-y-1 border-b border-line px-2.5 py-1.5">
        <div className="flex min-w-0 grow basis-40 items-center gap-2">
          {who}
          <SaveState
            savingLabel={t('save.saving')}
            savedLabel={t('save.saved')}
            status={editing.status}
            className="shrink-0"
          />
        </div>
        {tools}
        <span aria-hidden className="mx-0.5 h-4 w-px shrink-0 bg-line" />
        {modes.map(({ mode, icon: Icon, label: name }) => (
          <Button
            key={mode}
            variant={reading === mode ? 'soft' : 'icon'}
            size="icon-sm"
            aria-pressed={reading === mode}
            title={name}
            aria-label={name}
            disabled={body === null}
            onClick={() => (mode === 'edit' ? write() : read())}
          >
            <Icon aria-hidden />
          </Button>
        ))}
        {/* The stresses drawn over a sung text, to read it the way the
            singer will: an accent over every stressed vowel, the marked ones
            and the ones the dictionary is sure of. Drawn, never written in. */}
        {singing && (
          <Button
            variant={accents ? 'soft' : 'icon'}
            size="icon-sm"
            aria-pressed={accents}
            title={t('versions.stress.show')}
            aria-label={t('versions.stress.show')}
            disabled={body === null}
            onClick={() => setAccents(!accents)}
          >
            <MicVocal aria-hidden />
          </Button>
        )}
        {/* The body, on the clipboard. A style prompt exists to be pasted
            into something else, and this is one press where selecting it
            all is three. */}
        <Button
          variant="icon"
          size="icon-sm"
          title={t('versions.copyBody')}
          aria-label={t('versions.copyBody')}
          disabled={body === null || body === ''}
          onClick={() => {
            if (body === null) return
            // The tick only once the clipboard confirms, the rule from
            // v0.28: saying a copy succeeded when it did not is worse than
            // saying nothing.
            navigator.clipboard.writeText(body).then(
              () => say.ok(t('versions.bodyCopied')),
              (cause: unknown) => say.failedTo(t('versions.copyBody'), cause),
            )
          }}
        >
          <Copy aria-hidden />
        </Button>
        {/* A sung text as the public reads it: the capitals that mark a
            stress lowered, the acutes dropped, the owner's respellings put
            back - by the backend, with the same reading of a mark the check
            uses (ADR 0053). The text on screen, which while it is written is
            a pause ahead of the one on disk. */}
        {singing && (
          <Button
            variant="icon"
            size="icon-sm"
            title={t('versions.stress.copyClean')}
            aria-label={t('versions.stress.copyClean')}
            disabled={body === null || text === ''}
            onClick={() => {
              // The tick only once the clipboard confirms, as for the copy
              // beside it.
              cleanText(text)
                .then((clean) => navigator.clipboard.writeText(clean))
                .then(
                  () => say.ok(t('versions.stress.cleanCopied')),
                  (cause: unknown) => say.failedTo(t('versions.stress.copyClean'), cause),
                )
            }}
          >
            <RemoveFormatting aria-hidden />
          </Button>
        )}
        <Button
          variant={staged ? 'soft' : 'icon'}
          size="icon-sm"
          aria-pressed={staged}
          title={staged ? t('versions.collapse') : t('versions.expand')}
          aria-label={staged ? t('versions.collapse') : t('versions.expand')}
          onClick={() => onStage(!staged)}
        >
          {staged ? <Minimize2 aria-hidden /> : <Maximize2 aria-hidden />}
        </Button>
      </div>

      {/* What the register says of this text, read or written: the spent
          terms it takes, strictest first. */}
      {check !== undefined && check.terms.length > 0 && <RegisterStrip check={check} />}

      {/* What a sung text says about its stresses, read or written: the
          words the singer may get wrong, each with its answers. */}
      {singing && check !== undefined && check.stress.length > 0 && (
        <StressStrip
          notes={check.stress}
          onAnswer={answerNotes}
          onAnswerAll={answerEvery}
          onFind={find}
          returnTo={box}
        />
      )}

      {/* The words this text leans on, while it is being written. Nothing is
          drawn when there are none: a strip saying "no repeats" would be a
          strip taking the room the text wants. */}
      {reading === 'edit' && check !== undefined && check.repeats.length > 0 && (
        <div className="flex shrink-0 flex-wrap items-center gap-1.5 border-b border-line px-3 py-1.5 text-xs text-dim">
          <span className="mr-1 font-medium">{t('versions.repeats')}</span>
          {check.repeats.map((group, index) => (
            <span
              key={group.stem}
              className={cn('rounded-sm px-1.5 py-px', `repeat-${index % REPEAT_TINTS}`)}
            >
              {group.word} ×{group.count}
            </span>
          ))}
        </div>
      )}

      {/* One scroller for both columns, so the two texts move together. */}
      <TextScroll label={label}>
        <div className="flex min-w-0 flex-1">
          <div className="flex min-w-0 flex-1 flex-col *:flex-1">{content}</div>
          {against !== null && (
            <CompareColumn
              label={against.label}
              body={against.body}
              removed={diff.removed}
              counts={diff.counts}
              metrics={metrics}
              onClose={against.onClose}
            />
          )}
        </div>
      </TextScroll>
    </section>
  )
}

/**
 * The runs of a checked text as marks: a repeated word tinted while the text
 * is written, a term of the register marked by its strictness always, and in
 * a sung text a word the singer may get wrong marked by what is wrong with it
 * (ADR 0053). The backend cut the runs so that one never sits inside another.
 *
 * A homograph is marked only while the stresses are shown. There are about
 * ten in a song, nearly all read right, and a line under each would hide the
 * missing ё and the stress against the dictionary - one a song, and almost
 * always a mistake - which are marked always.
 */
function marksOf(check: TextCheck, writing: boolean, stresses: boolean): Mark[] {
  const out: Mark[] = []
  for (const mark of check.marks) {
    const tint =
      writing && mark.repeat !== undefined ? `repeat-${mark.repeat % REPEAT_TINTS}` : undefined
    const hit = mark.term === undefined ? undefined : check.terms[mark.term]
    const term = hit === undefined ? undefined : termMark(hit.strictness)
    const note = mark.stress === undefined ? undefined : check.stress[mark.stress]
    const stress =
      note === undefined || (note.kind === 'homograph' && !stresses)
        ? undefined
        : stressMark(note.kind)
    if (tint === undefined && term === undefined && stress === undefined) continue
    out.push({ start: mark.start, end: mark.end, className: cn(tint, term, stress) })
  }
  return out
}

/**
 * The register under a version's toolbar (ADR 0044): each spent term the
 * text takes, how often, and in how many works it already stands - one press
 * away from its entry. The count of works is the register's own, read off
 * the works' texts now.
 */
function RegisterStrip({ check }: { check: TextCheck }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const terms = useQuery(queries.terms())
  const uses = useMemo(
    () => new Map((terms.data ?? []).map((entry) => [entry.id, entry.uses])),
    [terms.data],
  )

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-1.5 border-b border-line px-3 py-1.5 text-xs text-dim">
      <span className="mr-1 font-medium">{t('register.inText')}</span>
      {check.terms.map((hit) => {
        const works = uses.get(hit.term_id)
        return (
          <Button
            key={hit.term_id}
            variant="ghost"
            size="xs"
            title={t('register.chip', { word: hit.word, count: hit.count, uses: works ?? '…' })}
            onClick={() => void navigate(`/register/${hit.term_id}`)}
          >
            <StatusDot
              size="sm"
              status={strictnessStatus(hit.strictness)}
              label={t(`register.strictnesses.${hit.strictness}`)}
            />
            <span>{hit.word}</span>
            <span className="tabular-nums">×{hit.count}</span>
            {works !== undefined && <span className="text-faint tabular-nums">· {works}</span>}
          </Button>
        )
      })}
    </div>
  )
}
