import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Check, X } from 'lucide-react'
import { dismissProposal } from '@/lib/api/assistant'
import { markJournalRead } from '@/lib/api/journal'
import type { JournalEntry, PendingProposal } from '@/lib/api/types'
import { formatMoment } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useAssistant } from '@/lib/useAssistant'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { NotificationBell } from '@/components/ui/notification-bell'
import { sentence } from '@/features/journal/JournalFeed'
import { cn } from '@/lib/utils'

/** How many entries the bell shows before sending you to the whole history. */
const RECENT = 6

/**
 * The bell in the title bar, lit only by things that ask to be looked at.
 *
 * Ordinary edits are recorded but never counted: a bell that is always lit is a
 * bell nobody reads. Only warnings — a release that lost its slot — light it.
 * Pressing it no longer leaves the screen: the last few entries open under it,
 * the ones that need a look first, and the whole history is one more click.
 */
export function Bell() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const assistant = useAssistant()
  const [open, setOpen] = useState(false)

  const unread = useQuery({
    // The bell sits outside the routes and never remounts, so without this it
    // would only ever change when a mutation in this window invalidated it —
    // and an entry written by a background task, a plugin or a second window
    // would leave it reading zero for as long as the app stayed open.
    ...queries.journalUnread(),
    refetchInterval: 60_000,
  })
  const count = unread.data ?? 0

  // Fetched only while the panel is open: the feed is one page of up to two
  // hundred entries, and the bell is pressed far less often than it is seen.
  const entries = useQuery({ ...queries.journalFeed(), enabled: open })

  // The journal is always refreshed by the hook; nothing else to refresh here.
  const markRead = useAppMutation({
    mutationFn: markJournalRead,
    failure: 'toast.loadFailed',
  })

  // Proposals that arrived over MCP and have been neither applied nor turned
  // down. Always fetched, not only while the panel is open: this is what the
  // badge counts, and a count that only appeared once you looked would be no
  // notification at all.
  const proposals = useQuery({ ...queries.pendingProposals(), refetchInterval: 30_000 })
  const waiting = proposals.data ?? []

  const dismiss = useAppMutation({
    mutationFn: dismissProposal,
    refresh: [keys.pendingProposals],
    failure: 'assistant.dismissFailed',
  })

  // What needs a look comes first, then the newest of the rest, and the two
  // together fill the short list - so an unread warning from last week is not
  // pushed out by six edits made this morning.
  const recent = (() => {
    const all = entries.data ?? []
    const warned = all.filter((entry) => entry.level === 'warn' && entry.read_at === null)
    const rest = all.filter((entry) => !warned.includes(entry))
    return [...warned, ...rest].slice(0, RECENT)
  })()

  return (
    <NotificationBell
      // A proposal is counted too: the work is done and waiting on an answer,
      // which is a stronger claim on attention than an unread line about
      // something that already happened.
      count={count + waiting.length}
      label={t('journal.open')}
      title={t('journal.recent')}
      open={open}
      onOpenChange={setOpen}
      // "Mark all seen" clears read marks on the journal and nothing else.
      // It is deliberately NOT offered while only proposals are waiting: a
      // proposal is answered by applying it or refusing it, and one button
      // that made a screenful of them disappear unanswered is the click
      // nobody means to make. Hidden rather than disabled, because a button
      // that does not act on what is on screen should not be on screen.
      markAllLabel={count > 0 ? t('journal.markRead') : undefined}
      onMarkAll={() => markRead.mutate()}
      busy={markRead.isPending}
      seeAllLabel={t('journal.seeAll')}
      onSeeAll={() => navigate('/journal')}
      emptyLabel={t('journal.nothingRecent')}
    >
      {/* What is waiting on an answer, above what has already happened: these
          are the only lines in the panel that ask for something. */}
      {waiting.length > 0 && (
        <section className="border-b border-line py-2">
          <h4 className="pb-1 caption">{t('assistant.waitingOnYou', { count: waiting.length })}</h4>
          <ul>
            {waiting.map((proposal) => (
              <ProposalLine
                key={proposal.message_id}
                proposal={proposal}
                busy={dismiss.isPending}
                onOpen={() => {
                  setOpen(false)
                  // A comment or a reply is kept where comments are read,
                  // with the fields open to correction first; a proposal on
                  // a work, on the card's assistant tab. One in a chat about
                  // nothing - a note, a style's description - has no screen
                  // of its own, so the chat itself opens in the drawer. It
                  // used to go to `/assistant`, a route that never existed.
                  if (proposal.kind === 'comment' || proposal.kind === 'reply') {
                    navigate('/comments')
                  } else if (proposal.work_id !== null) {
                    navigate(`/works/${proposal.work_id}/assistant`)
                  } else {
                    assistant.open(proposal.chat_id)
                  }
                }}
                onDismiss={() => dismiss.mutate(proposal.message_id)}
              />
            ))}
          </ul>
        </section>
      )}

      {/* Rows only once they have arrived: until then the bell shows its
          empty line, which is truer than a list of nothing. */}
      {entries.isSuccess && recent.length > 0 && (
        <ul>
          {recent.map((entry) => (
            <RecentLine key={entry.id} entry={entry} />
          ))}
        </ul>
      )}
    </NotificationBell>
  )
}

/**
 * One proposal waiting for an answer.
 *
 * The line opens the chat it arrived in rather than applying anything from
 * here: applying is a decision made while looking at what is proposed, and a
 * bell is not where a version gets written. Turning one down does happen
 * here, because refusing needs nothing read that the line does not say.
 */
function ProposalLine({
  proposal,
  busy,
  onOpen,
  onDismiss,
}: {
  proposal: PendingProposal
  busy: boolean
  onOpen: () => void
  onDismiss: () => void
}) {
  const { t } = useTranslation()
  const what = t(`assistant.proposed.${proposal.kind}`, {
    defaultValue: t('assistant.proposed.other'),
  })

  return (
    <li className="flex items-center gap-1.5 py-1">
      <RowButton
        onClick={onOpen}
        description={proposal.chat_title ?? undefined}
        end={<time dateTime={proposal.created_at}>{formatMoment(proposal.created_at)}</time>}
        className="flex-1"
      >
        {what}
      </RowButton>
      <Button
        variant="icon"
        size="icon-sm"
        onClick={onOpen}
        disabled={busy}
        title={t('assistant.openToApply')}
        aria-label={t('assistant.openToApply')}
      >
        <Check aria-hidden className="size-4" />
      </Button>
      <Button
        variant="icon"
        size="icon-sm"
        onClick={onDismiss}
        disabled={busy}
        title={t('assistant.dismiss')}
        aria-label={t('assistant.dismiss')}
      >
        <X aria-hidden className="size-3.5" />
      </Button>
    </li>
  )
}

function RecentLine({ entry }: { entry: JournalEntry }) {
  const { t } = useTranslation()
  const needsALook = entry.level === 'warn' && entry.read_at === null
  return (
    <li className="flex items-baseline gap-2.5 border-b border-line py-2 last:border-b-0">
      <span
        aria-hidden
        className={cn(
          'mt-1.5 size-1.5 shrink-0 rounded-full',
          needsALook ? 'bg-warn' : 'bg-line-2',
        )}
      />
      <p className={cn('min-w-0 flex-1 text-sm text-dim', needsALook && 'text-text')}>
        {sentence(entry, t)}
      </p>
      <time dateTime={entry.created_at} className="shrink-0 text-xs tabular-nums text-faint">
        {formatMoment(entry.created_at)}
      </time>
    </li>
  )
}
