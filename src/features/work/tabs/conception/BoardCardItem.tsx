import type { PointerEvent, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { GripVertical, Star, Trash2, X } from 'lucide-react'
import type { Board, BoardCard, CardField, CardVerdict } from '@/lib/api/types'
import { kindOf } from '@/lib/boards'
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
  /** Its place among the cards of its topic: where a card dropped on it
   *  stands. */
  slot: number
  /** Arrived while the board was watched. */
  fresh?: boolean
  /** Being carried: it waits, dimmed, where it was picked up from. */
  carried?: boolean
  busy?: boolean
  onVerdict: (verdict: CardVerdict | null) => void
  onField: (key: string, text: string) => void
  onDelete: () => void
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
 * Laid out as a cover's idea is, without a picture: a card is words. A field
 * left empty stays on the card as its hint, so a scenario put on the board as
 * one line shows what it still lacks.
 */
export function BoardCardItem({
  board,
  card,
  slot,
  fresh = false,
  carried = false,
  busy = false,
  onVerdict,
  onField,
  onDelete,
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

  return (
    <article
      data-card-slot={slot}
      data-card-topic={card.topic}
      aria-label={card.fields[fields[0]?.key ?? ''] ?? t('board.card')}
      className={cn(
        'flex flex-col gap-1.5 rounded-lg border bg-raise px-2.5 pt-2 pb-2',
        starred ? 'border-accent' : 'border-line',
        rejected && 'opacity-55',
        carried && 'opacity-35',
      )}
    >
      <div className="flex items-center gap-1.5">
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
        <SourceBadge source={card.source} />
        {board.cards.length > 1 && kind !== undefined && <Chip>{sayLabel(kind.label)}</Chip>}
        {fresh && (
          <Chip variant="good" className="ml-auto shrink-0">
            {t('board.fresh')}
          </Chip>
        )}
      </div>

      {fields.map((field, index) => (
        <CardText
          key={field.key}
          field={field}
          headline={index === 0}
          value={card.fields[field.key] ?? ''}
          onCommit={(text) => onField(field.key, text)}
        />
      ))}

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

/** One field of a card, written in place: the headline as a line in bold,
 *  the rest as paragraphs. Emptying the headline is refused by the backend
 *  only when the card would say nothing at all. */
function CardText({
  field,
  headline,
  value,
  onCommit,
}: {
  field: CardField
  headline: boolean
  value: string
  onCommit: (text: string) => void
}) {
  const label = sayLabel(field.label)
  const draft = useFieldDraft(
    value,
    (text) => {
      if (text.trim() !== value.trim()) onCommit(text)
    },
    { multiline: field.multiline === true },
  )
  return (
    <label className="flex flex-col gap-0.5">
      {!headline && <span className="caption text-2xs">{label}</span>}
      <Textarea
        aria-label={label}
        // The headline says what it is called while empty; a field under its
        // own caption shows its hint, or only that there is room.
        placeholder={field.hint != null ? sayLabel(field.hint) : headline ? label : '…'}
        autoResize
        rows={1}
        maxRows={headline ? 4 : 8}
        {...draft}
        className={cn(
          '-mx-1 w-auto resize-none border-transparent px-1 py-0.5 hover:border-line-2 hover:bg-soft focus:bg-soft',
          headline ? 'text-sm leading-tight font-semibold' : 'text-xs leading-relaxed text-dim',
        )}
      />
    </label>
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
