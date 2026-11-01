import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { MoreHorizontal, Star } from 'lucide-react'
import { offers, type Exchange } from '@/lib/chat'
import { formatCost, formatDuration } from '@/lib/format'
import { say } from '@/lib/toast'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Markdown } from '@/components/Markdown'
import { Proposed } from '@/features/assistant/Proposed'

interface Props {
  item: Exchange
  /** Absent when the chat is about nothing — there is nothing to score. */
  workId?: string
  /** Absent when the chat is about nothing — there is no work to version. */
  onInsert?: (body: string, role?: string, label?: string, messageId?: string) => void
  /** Keep the answer as a note. Always offered: a note needs no work. */
  onKeepAsNote: (body: string) => void
  /** Star the answer, or take the star off. */
  onStar: (messageId: string, starred: boolean) => void
  /** The answer the conversation was opened on - from the starred list -
   *  drawn with a ring until another is opened. */
  focused?: boolean
}

/**
 * One exchange, as the mockup draws a conversation (`.msgs`): the question on
 * the right in the accent's soft ground, what the run did on the way as small
 * mono lines, the answer on the left on the soft ground with a border, and
 * under it the card of whatever it proposed.
 *
 * What can be done with an answer - insert it as a version, keep it as a
 * note, copy it - is behind the three dots at its foot rather than a row of
 * buttons under every answer: read first, act second, and a transcript of
 * twenty answers is not a wall of sixty buttons.
 */
export function ExchangeItem({
  item,
  workId,
  onInsert,
  onKeepAsNote,
  onStar,
  focused = false,
}: Props) {
  const { t } = useTranslation()
  const run = item.run

  // The settled message is canonical; a run's own body stands in while it is
  // still growing, or when the run ended before an answer was stored.
  const body = item.answer?.body ?? run?.body ?? ''
  const cost = item.answer?.cost ?? run?.cost ?? null
  // What the run cost in time as well as in money: the two facts the CLI
  // reports about a finished turn, side by side under the answer.
  const took = formatDuration(item.answer?.durationMs ?? run?.durationMs ?? null)
  const settled = run?.working !== true
  const offered = offers(item, onInsert !== undefined)
  const measured = [cost == null ? null : formatCost(cost, 3), took].filter(
    (part): part is string => part !== null,
  )

  const answer = item.answer
  const starred = answer?.starred != null

  return (
    // The answer's id as the item's, so the starred list can bring it into view.
    <li className="flex flex-col gap-3" id={answer === null ? undefined : answerAnchor(answer.id)}>
      {item.prompt !== null && (
        <p className="selectable max-w-[78%] self-end rounded-lg bg-accent-soft px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap">
          {item.prompt}
        </p>
      )}

      {run !== null && run.steps.length > 0 && (
        <ul className="flex flex-col gap-1 self-start">
          {run.steps.map((step, index) => (
            <ToolLine
              key={`${item.key}-${String(index)}`}
              as="li"
              // The step in hand is the last one of a run still going.
              live={run.working && index === run.steps.length - 1}
            >
              {step}
            </ToolLine>
          ))}
        </ul>
      )}

      {body !== '' && (
        <div
          // What a focus scrolls to: the answer, not its question, which for
          // an action is a whole rendered template above it.
          data-answer
          className={cn(
            'flex max-w-[78%] scroll-mt-2 flex-col gap-1.5 self-start rounded-lg border border-line bg-soft px-3 py-2',
            focused && 'ring-2 ring-accent',
          )}
        >
          {/* Who said it, when it was not the assistant in this panel: an
              agent outside the window, named by its client, with what it said
              about its proposal. */}
          {item.answer?.source != null && (
            <p className="text-xs text-dim">
              {t('assistant.proposedBy', { client: item.answer.source })}
              {item.answer.note != null && item.answer.note !== '' && (
                <>
                  {' — '}
                  {item.answer.note}
                </>
              )}
            </p>
          )}
          <Markdown
            body={body}
            className="text-sm leading-relaxed"
            copyLabel={t('assistant.copy')}
          />
          <div className="flex items-center gap-2">
            {measured.length > 0 && (
              <span className="font-mono text-2xs text-faint tabular-nums">
                {measured.join(' · ')}
              </span>
            )}
            <span className="ml-auto flex items-center gap-0.5">
              {/* The star is a mark as much as a button: an answer worth coming
                back to says so where it stands, not only in a list. */}
              {answer !== null && settled && (
                <Button
                  variant="icon"
                  size="icon-sm"
                  aria-pressed={starred}
                  aria-label={t(starred ? 'assistant.unstar' : 'assistant.star')}
                  title={t(starred ? 'assistant.unstar' : 'assistant.star')}
                  onClick={() => {
                    onStar(answer.id, !starred)
                  }}
                >
                  <Star aria-hidden className={cn(starred && 'fill-current text-accent')} />
                </Button>
              )}
              <AnswerMenu
                body={body}
                onInsert={
                  offered.insert && onInsert !== undefined
                    ? () => {
                        onInsert(body)
                      }
                    : undefined
                }
                onKeepAsNote={
                  offered.keep
                    ? () => {
                        onKeepAsNote(body)
                      }
                    : undefined
                }
              />
            </span>
          </div>
        </div>
      )}

      {/* The action asked for something and the answer's block could not be
          read as it: said, so a card that never appears is never silent. */}
      {item.answer?.refused != null && settled && (
        <p role="alert" className="self-start pl-1 text-xs text-warn">
          {t('assistant.proposalRefused', { why: item.answer.refused })}
        </p>
      )}

      {/* What the answer proposed, with the button that applies it. Under the
          answer rather than in its menu: it is a decision, not a convenience,
          and it needs room to show the numbers first. */}
      {item.answer !== null && settled && (
        <Proposed
          answer={item.answer}
          workId={workId}
          onChooseVersion={(role, label) => {
            onInsert?.(body, role, label, item.answer?.id)
          }}
        />
      )}

      {run?.working === true && <ToolLine live>{t('assistant.working')}</ToolLine>}
      {run?.cancelled === true && <ToolLine>{t('assistant.stopped')}</ToolLine>}
      {run?.failure != null && <ToolLine tone="warn">{run.failure}</ToolLine>}
    </li>
  )
}

/** The id an answer's item carries in the page, for bringing it into view. */
export const answerAnchor = (messageId: string) => `answer-${messageId}`

/**
 * A line about what the run did rather than what it said: a tool it used, that
 * it is still working, that it stopped. Small and mono, as the mockup draws
 * them (`.toolline`), so the conversation reads over them.
 */
function ToolLine({
  children,
  as: Line = 'p',
  live = false,
  tone = 'plain',
}: {
  children: ReactNode
  /** `li` for a step in the run's list of them. */
  as?: 'p' | 'li'
  /** The run is on this step now: the dot pulses. */
  live?: boolean
  tone?: 'plain' | 'warn'
}) {
  return (
    <Line
      className={cn(
        'flex items-center gap-2 self-start pl-1 font-mono text-xs',
        tone === 'warn' ? 'text-warn' : 'text-faint',
      )}
    >
      <span
        aria-hidden
        className={cn(
          'size-1.5 shrink-0 rounded-full',
          live ? 'animate-pulse bg-warn' : tone === 'warn' ? 'bg-warn' : 'bg-line-2',
        )}
      />
      <span className="min-w-0 truncate">{children}</span>
    </Line>
  )
}

/** What can be done with an answer, behind the three dots at its foot. */
function AnswerMenu({
  body,
  onInsert,
  onKeepAsNote,
}: {
  body: string
  /** Absent where inserting is not offered: no work, or a card of its own. */
  onInsert?: () => void
  /** Absent where keeping is not offered. */
  onKeepAsNote?: () => void
}) {
  const { t } = useTranslation()

  // Said in a toast either way: the menu is gone by the time the clipboard
  // answers, and a refusal swallowed would read as a copy that worked.
  const copy = () => {
    void Promise.resolve()
      .then(() => navigator.clipboard.writeText(body))
      .then(
        () => {
          say.ok(t('assistant.copied'))
        },
        () => {
          say.failed(t('assistant.copyFailed'))
        },
      )
  }

  return (
    <Menu>
      <MenuTrigger
        render={
          <Button
            variant="icon"
            size="icon-sm"
            aria-label={t('assistant.answerMenu')}
            title={t('assistant.answerMenu')}
          />
        }
      >
        <MoreHorizontal aria-hidden />
      </MenuTrigger>
      <MenuPopup align="end">
        {onInsert !== undefined && <MenuItem onClick={onInsert}>{t('assistant.insert')}</MenuItem>}
        {onKeepAsNote !== undefined && (
          <MenuItem onClick={onKeepAsNote}>{t('assistant.keepAsNote')}</MenuItem>
        )}
        <MenuItem onClick={copy}>{t('assistant.copy')}</MenuItem>
      </MenuPopup>
    </Menu>
  )
}
