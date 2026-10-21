import { useState, type PointerEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import {
  ArrowUpRight,
  BadgeCheck,
  Check,
  GripVertical,
  History,
  Pencil,
  Plus,
  Star,
  Trash2,
  X,
} from 'lucide-react'
import type { Board, BoardCard, CardField, CardVerdict, NoteKind, WorkedIn } from '@/lib/api/types'
import { chipsOf, entryHeadline, isLink, kindOf, mediumOf, quoted } from '@/lib/boards'
import { say as sayLabel } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { useFieldDraft } from '@/components/ui/field-draft'

interface Props {
  board: Board
  card: BoardCard
  /** The library's kind, for a card that links to an entry of it. */
  library?: NoteKind
  /** Where the card's entry has worked besides this work. */
  elsewhere?: WorkedIn[]
  /** The phrases already kept for the work, lowercased: a chip among them
   *  is sent. */
  kept?: ReadonlySet<string>
  /** Its place among the cards of its topic: where a card dropped on it
   *  stands. */
  slot: number
  /** Arrived while the board was watched. */
  fresh?: boolean
  /** Being carried: it waits, dimmed, where it was picked up from. */
  carried?: boolean
  /** Whether it can be carried at all: not while the board is read by kind. */
  movable?: boolean
  busy?: boolean
  onVerdict: (verdict: CardVerdict | null) => void
  onField: (key: string, text: string) => void
  onDelete: () => void
  /** A chip kept as a phrase of the work. */
  onChip?: (chip: string) => void
  /** The grip was pressed: a carry may begin. */
  onGrab: (event: PointerEvent) => void
  /** Its topic, changed by hand. */
  topicMenu: ReactNode
}

/**
 * One card of a board of the conception (ADR 0065): where it came from across
 * the top, each field of its kind written in place - the first is its
 * headline - and what a person does with it: star it, turn it down, move it,
 * take it off the board.
 *
 * A card of the research that names a work of culture (v0.100, ADR 0066) is
 * called by its entry of the library - one record for the whole workspace,
 * opened from here - and says where else it has worked. Its quote says
 * whether it was checked, and where; its chips go to the work's phrases one
 * by one.
 *
 * Laid out as a cover's idea is, without a picture: a card is words. A field
 * left empty stays on the card as its hint, so a scenario put on the board as
 * one line shows what it still lacks.
 */
export function BoardCardItem({
  board,
  card,
  library,
  elsewhere = [],
  kept,
  slot,
  fresh = false,
  carried = false,
  movable = true,
  busy = false,
  onVerdict,
  onField,
  onDelete,
  onChip,
  onGrab,
  topicMenu,
}: Props) {
  const { t } = useTranslation()
  const kind = kindOf(board, card)
  const starred = card.verdict === 'star'
  const rejected = card.verdict === 'rejected'
  const fields: CardField[] =
    kind?.fields ??
    Object.keys(card.fields).map((key) => ({ key, label: key, hint: null, multiline: true }))
  const proof = fields.find((field) => field.shape === 'proof')
  const checked = proof !== undefined && isLink(card.fields[proof.key])
  const sayable = quoted(library, card.entry)
  const linked = kind?.library === true

  return (
    <article
      data-card-slot={slot}
      data-card-topic={card.topic}
      aria-label={
        card.entry != null
          ? entryHeadline(card.entry)
          : (card.fields[fields[0]?.key ?? ''] ?? t('board.card'))
      }
      className={cn(
        'flex flex-col gap-1.5 rounded-lg border bg-raise px-2.5 pt-2 pb-2',
        starred ? 'border-accent' : 'border-line',
        rejected && 'opacity-55',
        carried && 'opacity-35',
      )}
    >
      <div className="flex items-center gap-1.5">
        {movable && (
          <Button
            variant="icon"
            size="icon-xs"
            aria-label={t('board.move')}
            title={t('board.move')}
            className="-ml-1 cursor-grab touch-none"
            onPointerDown={onGrab}
          >
            <GripVertical aria-hidden />
          </Button>
        )}
        <SourceBadge source={card.source} />
        {board.cards.length > 1 && kind !== undefined && <Chip>{sayLabel(kind.label)}</Chip>}
        {fresh && (
          <Chip variant="good" className="ml-auto shrink-0">
            {t('board.fresh')}
          </Chip>
        )}
      </div>

      {linked && <EntryHead card={card} library={library} elsewhere={elsewhere} />}

      {fields.map((field, index) =>
        field.shape === 'chips' ? (
          <Chips
            key={field.key}
            field={field}
            value={card.fields[field.key] ?? ''}
            kept={kept}
            busy={busy}
            onChip={onChip}
            onCommit={(text) => onField(field.key, text)}
          />
        ) : (
          <CardText
            key={field.key}
            field={field}
            headline={index === 0 && !linked}
            value={card.fields[field.key] ?? ''}
            onCommit={(text) => onField(field.key, text)}
            mark={
              field.shape === 'quote' && (card.fields[field.key] ?? '').trim() !== '' ? (
                <QuoteMark
                  sayable={sayable}
                  proof={checked && proof !== undefined ? card.fields[proof.key] : undefined}
                />
              ) : field.shape === 'proof' && checked ? (
                <a
                  href={card.fields[field.key]}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-0.5 text-2xs text-info hover:underline"
                >
                  {t('board.openProof')}
                  <ArrowUpRight aria-hidden className="size-3" />
                </a>
              ) : null
            }
          />
        ),
      )}

      <div className="mt-auto flex items-center gap-1.5 pt-1">
        <Verdict
          label={t('board.star')}
          on={starred}
          disabled={busy}
          onPress={() => onVerdict(starred ? null : 'star')}
        >
          <Star aria-hidden />
        </Verdict>
        <Verdict
          label={t('board.reject')}
          on={rejected}
          disabled={busy}
          onPress={() => onVerdict(rejected ? null : 'rejected')}
        >
          <X aria-hidden />
        </Verdict>
        <span className="ml-auto flex items-center gap-1">
          {topicMenu}
          <Button
            size="icon-sm"
            variant="icon"
            aria-label={t('board.delete')}
            title={t('board.delete')}
            disabled={busy}
            onClick={onDelete}
          >
            <Trash2 aria-hidden />
          </Button>
        </span>
      </div>
    </article>
  )
}

/** What a card of the library is about: its entry, opened in the library,
 *  its medium, and the other works it has worked in. */
function EntryHead({
  card,
  library,
  elsewhere,
}: {
  card: BoardCard
  library: NoteKind | undefined
  elsewhere: WorkedIn[]
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const entry = card.entry
  if (entry == null) {
    return <p className="text-xs text-faint">{t('board.entryGone')}</p>
  }
  const medium = mediumOf(library, entry.medium)
  const kindKey = library?.key ?? ''
  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-baseline gap-1.5">
        <Button
          variant="link"
          className="h-auto min-w-0 p-0 text-left text-sm leading-tight font-semibold whitespace-normal"
          title={t('board.openEntry')}
          onClick={() =>
            void navigate(`/notes/${entry.id}?${new URLSearchParams({ kind: kindKey }).toString()}`)
          }
        >
          {entryHeadline(entry)}
        </Button>
        {medium !== undefined && <Chip>{sayLabel(medium.label)}</Chip>}
      </div>
      {elsewhere.length > 0 && (
        <p className="flex flex-wrap items-center gap-1 text-2xs text-warn">
          <History aria-hidden className="size-3" />
          {t('board.workedIn')}
          {elsewhere.map((work) => (
            <Button
              key={work.work_id}
              variant="link"
              className="h-auto p-0 text-2xs text-warn"
              onClick={() => void navigate(`/works/${work.work_id}/conception`)}
            >
              «{work.title}»
            </Button>
          ))}
        </p>
      )}
    </div>
  )
}

/** Whether a quote reaches the writer word for word: checked at a link, or
 *  from memory - and a song's words never. */
function QuoteMark({ sayable, proof }: { sayable: boolean; proof: string | undefined }) {
  const { t } = useTranslation()
  if (!sayable) {
    return (
      <span className="text-2xs text-faint" title={t('board.notQuotedHint')}>
        {t('board.notQuoted')}
      </span>
    )
  }
  if (proof !== undefined) {
    return (
      <a
        href={proof}
        target="_blank"
        rel="noreferrer"
        title={proof}
        className="inline-flex items-center gap-0.5 text-2xs text-good hover:underline"
      >
        <BadgeCheck aria-hidden className="size-3" />
        {t('board.checked')}
      </a>
    )
  }
  return (
    <span className="text-2xs text-warn" title={t('board.fromMemoryHint')}>
      {t('board.fromMemory')}
    </span>
  )
}

/** One field of a card, written in place: the headline as a line in bold,
 *  the rest as paragraphs. Emptying the headline is refused by the backend
 *  only when the card would say nothing at all. */
function CardText({
  field,
  headline,
  value,
  onCommit,
  mark,
}: {
  field: CardField
  headline: boolean
  value: string
  onCommit: (text: string) => void
  /** What stands beside the field's word: a quote's check, a proof's link. */
  mark?: ReactNode
}) {
  const { t } = useTranslation()
  const label = sayLabel(field.label)
  // An empty field under its caption shows its hint as an example, so a card
  // of eight fields with two written does not read as saying all eight.
  const placeholder =
    field.hint != null
      ? headline
        ? sayLabel(field.hint)
        : t('board.hintExample', { hint: sayLabel(field.hint) })
      : headline
        ? label
        : '…'
  const draft = useFieldDraft(
    value,
    (text) => {
      if (text.trim() !== value.trim()) onCommit(text)
    },
    { multiline: field.multiline === true },
  )
  return (
    <label className="flex flex-col gap-0.5">
      {!headline && (
        <span className="flex items-center gap-1.5">
          <span className="caption text-2xs">{label}</span>
          {mark}
        </span>
      )}
      <Textarea
        aria-label={label}
        // The headline says what it is called while empty; a field under its
        // own caption shows its hint, or only that there is room.
        placeholder={placeholder}
        autoResize
        rows={1}
        maxRows={headline ? 4 : 8}
        {...draft}
        className={cn(
          '-mx-1 w-auto resize-none border-transparent px-1 py-0.5 hover:border-line-2 hover:bg-soft focus:bg-soft',
          headline ? 'text-sm leading-tight font-semibold' : 'text-xs leading-relaxed text-dim',
          field.shape === 'quote' && 'italic',
        )}
      />
    </label>
  )
}

/** A field of chips: each an image, a name, a place - kept as a phrase of
 *  the work with one press, marked once it is one - and the words
 *  themselves, written on "Edit". */
function Chips({
  field,
  value,
  kept,
  busy,
  onChip,
  onCommit,
}: {
  field: CardField
  value: string
  kept: ReadonlySet<string> | undefined
  busy: boolean
  onChip: ((chip: string) => void) | undefined
  onCommit: (text: string) => void
}) {
  const { t } = useTranslation()
  const [editing, setEditing] = useState(false)
  const chips = chipsOf(value)
  const label = sayLabel(field.label)
  if (editing || chips.length === 0) {
    return (
      <CardText
        field={field}
        headline={false}
        value={value}
        onCommit={(text) => {
          onCommit(text)
          setEditing(false)
        }}
      />
    )
  }
  return (
    <div className="flex flex-col gap-0.5">
      <span className="flex items-center gap-1.5">
        <span className="caption text-2xs">{label}</span>
        <Button
          size="icon-xs"
          variant="icon"
          aria-label={t('board.editChips')}
          title={t('board.editChips')}
          onClick={() => setEditing(true)}
        >
          <Pencil aria-hidden />
        </Button>
      </span>
      <div className="flex flex-wrap gap-1">
        {chips.map((chip) => {
          const sent = kept?.has(chip.toLowerCase()) === true
          return (
            <Button
              key={chip}
              size="xs"
              variant={sent ? 'soft' : 'ghost'}
              disabled={sent || busy || onChip === undefined}
              title={sent ? t('board.chipKept') : t('board.chipToPhrases', { chip })}
              onClick={() => onChip?.(chip)}
              className="h-6 px-1.5"
            >
              {sent ? <Check aria-hidden /> : <Plus aria-hidden />}
              {chip}
            </Button>
          )
        })}
      </div>
    </div>
  )
}

/** A verdict's button: pressed while the card has it. */
function Verdict({
  label,
  on,
  disabled,
  onPress,
  children,
}: {
  label: string
  on: boolean
  disabled?: boolean
  onPress: () => void
  children: ReactNode
}) {
  return (
    <Button
      size="icon-sm"
      variant={on ? 'soft' : 'ghost'}
      aria-label={label}
      aria-pressed={on}
      title={label}
      disabled={disabled}
      onClick={onPress}
    >
      {children}
    </Button>
  )
}

/** Where a card came from, as its first word. */
function SourceBadge({ source }: { source: BoardCard['source'] }) {
  const { t } = useTranslation()
  const tone =
    source === 'own'
      ? 'bg-info-soft text-info'
      : source === 'refined'
        ? 'bg-good-soft text-good'
        : 'bg-soft text-dim'
  return (
    <span
      className={cn(
        'shrink-0 rounded-xs px-1.5 py-px font-mono text-2xs tracking-caption uppercase',
        tone,
      )}
    >
      {t(`board.source.${source}`)}
    </span>
  )
}

/** A card while it is being written. */
export function CardSkeleton() {
  return (
    <div
      aria-hidden
      className="flex flex-col gap-1.5 rounded-lg border border-line bg-raise px-2.5 pt-2 pb-2.5"
    >
      <Skeleton className="h-3.5 w-1/3" />
      <Skeleton className="h-4 w-4/5" />
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-3 w-3/5" />
    </div>
  )
}
