import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { dismissProposal } from '@/lib/api/assistant'
import type { Applied } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { AppliedMark } from '@/features/assistant/AppliedMark'

interface Props {
  /** The message carrying the proposal. */
  messageId: string
  /** What is proposed, as the caption across the top: "Proposed score". */
  title: ReactNode
  /** A figure at the far end of the caption - a score's total. */
  figure?: ReactNode
  /** What applying would write, in a line or two under the caption. */
  children?: ReactNode
  /** Said in the warning tone: what does not fit, what cannot be undone. */
  warnings?: readonly string[]
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
  /** The verb on the primary button, in this kind's words: "Apply as a score". */
  applyLabel: string
  onApply: () => void
  applying: boolean
  /** A second way to take it, beside the verb: "Choose role…". */
  more?: ReactNode
  /** The mark once applied: "Scored.", "Inserted". */
  appliedLabel: string
  /** What stands beside the mark once applied: "Open the board". */
  afterApplied?: ReactNode
}

/**
 * The one shape every proposal takes under its answer.
 *
 * The mockup's card (`.proposal`): an accent frame on the accent's soft
 * ground, a caption saying what is proposed with a figure at its far end, a
 * line or two of what applying would write, then the verb and "Turn it down".
 * Until v0.81 there were five cards in three looks, and none of them could be
 * refused from the chat - a proposal nobody wanted sat in the bell and the
 * count over *apply all* until someone found the bell's cross.
 *
 * Refusing writes nothing: the answer stays in the chat, readable, and the
 * card says it was turned down instead of offering the verb again. Applied or
 * refused, the card steps back to the quiet ground - it is answered, and the
 * accent is for what still waits on the person.
 */
export function ProposalCard({
  messageId,
  title,
  figure,
  children,
  warnings = [],
  applied,
  dismissed,
  applyLabel,
  onApply,
  applying,
  more,
  appliedLabel,
  afterApplied,
}: Props) {
  const { t } = useTranslation()

  const dismiss = useAppMutation({
    mutationFn: () => dismissProposal(messageId),
    failure: 'assistant.dismissFailed',
    // The mark lives on the message, the count in the bell, and a refused
    // reply or comment stops waiting on the comments screen as well.
    refresh: [keys.transcripts, keys.pendingProposals, keys.comments],
  })

  const answered = applied !== null || dismissed

  return (
    <section
      className={cn(
        'flex max-w-[78%] flex-col gap-2 self-start rounded-lg border px-3 py-2.5',
        answered ? 'border-line bg-soft' : 'border-accent bg-accent-soft',
      )}
    >
      <div className="flex items-center gap-2">
        <span className="text-2xs font-semibold tracking-caption text-accent-2 uppercase">
          {title}
        </span>
        {figure !== undefined && (
          <span className="ml-auto font-mono text-lg font-semibold tabular-nums">{figure}</span>
        )}
      </div>

      {children !== undefined && (
        <div className="flex flex-col gap-1 text-xs text-dim">{children}</div>
      )}

      {/* A proposal that only half fits is still worth applying - but never
          without saying so. Once it is answered the warning has had its say. */}
      {!answered &&
        warnings.map((warning) => (
          <p key={warning} className="text-xs text-warn">
            {warning}
          </p>
        ))}

      <div className="flex flex-wrap items-center gap-1.5">
        {applied !== null ? (
          <>
            <AppliedMark applied={applied} label={appliedLabel} />
            {afterApplied}
          </>
        ) : dismissed ? (
          <span className="flex items-center gap-1.5 text-xs text-dim">
            <X aria-hidden className="size-3.5" />
            {t('assistant.dismissedMark')}
          </span>
        ) : (
          <>
            <Button
              size="sm"
              variant="primary"
              disabled={applying || dismiss.isPending}
              onClick={onApply}
            >
              {applyLabel}
            </Button>
            {more}
            <Button
              size="sm"
              variant="ghost"
              disabled={applying || dismiss.isPending}
              onClick={() => {
                dismiss.mutate()
              }}
            >
              {t('assistant.dismiss')}
            </Button>
          </>
        )}
      </div>
    </section>
  )
}
