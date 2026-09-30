import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { dismissProposal } from '@/lib/api/assistant'
import type { ReleaseProposal, ScheduledRelease } from '@/lib/api/types'
import { formatStamp } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { useAssistant } from '@/lib/useAssistant'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'

interface Props {
  release: ScheduledRelease
}

/**
 * What waits to be written into a release, under its fields (v0.86).
 *
 * The release's own action fills the fields nobody had written and leaves the
 * started ones here; an agent over MCP leaves all of them here. Each proposal
 * is read field by field - what is written now beside what would replace it -
 * and taken in part: a better title must not cost the description someone
 * had already polished. Nothing is written until the person takes it.
 */
export function ReleaseProposals({ release }: Props) {
  const proposals = useQuery(queries.releaseProposals(release.id))
  const waiting = proposals.data ?? []
  if (waiting.length === 0) return null

  return (
    <div className="flex flex-col gap-2">
      {waiting.map((proposal) => (
        <ProposalPanel key={proposal.message_id} release={release} proposal={proposal} />
      ))}
    </div>
  )
}

function ProposalPanel({
  release,
  proposal,
}: {
  release: ScheduledRelease
  proposal: ReleaseProposal
}) {
  const { t } = useTranslation()
  const assistant = useAssistant()
  // Every field taken until one is unchecked: the proposal was asked for, and
  // the question left is which of it to keep.
  const [chosen, setChosen] = useState<string[]>(() => proposal.fields.map((field) => field.key))

  // The fields and what waits beside them, the release's list where they are
  // read as a whole, the bell's count, and the whole of what a release
  // touches - a written field can move its work's readiness.
  const moved = [
    ...refresh.release,
    keys.releaseFields(release.id),
    keys.releaseProposalsFor(release.id),
    keys.pendingProposals,
  ]

  const take = useApplyProposal({
    messageId: proposal.message_id,
    message: t('releases.proposals.taken'),
    refresh: moved,
  })

  const dismiss = useAppMutation({
    mutationFn: () => dismissProposal(proposal.message_id),
    failure: 'assistant.dismissFailed',
    refresh: [...moved, keys.transcripts],
  })

  const busy = take.isPending || dismiss.isPending
  const toggle = (key: string, on: boolean) =>
    setChosen((was) => (on ? [...was, key] : was.filter((one) => one !== key)))

  return (
    <section
      aria-label={t('releases.proposals.label')}
      className="flex flex-col gap-2 rounded-lg border border-accent bg-accent-soft px-3 py-2.5"
    >
      <header className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
        <span className="text-2xs font-semibold tracking-caption text-accent-2 uppercase">
          {proposal.client === null
            ? t('releases.proposals.byAssistant')
            : t('releases.proposals.byClient', { client: proposal.client })}
        </span>
        <span className="text-xs text-faint">{formatStamp(proposal.created_at)}</span>
        <Button
          size="xs"
          variant="link"
          className="ml-auto"
          onClick={() => assistant.open(proposal.chat_id)}
        >
          {t('releases.proposals.openChat')}
        </Button>
      </header>

      {proposal.fields.map((field) => {
        const label = sayLabel(field.label)
        return (
          <div key={field.key} className="flex items-start gap-2">
            <Checkbox
              checked={chosen.includes(field.key)}
              onCheckedChange={(on) => toggle(field.key, on)}
              aria-label={t('releases.proposals.takeField', { label })}
              disabled={busy}
              className="mt-0.5"
            />
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <span className="caption">{label}</span>
              {/* What is there now, dim, above what would take its place:
                  the two are read against each other, top to bottom. */}
              <dl className="grid grid-cols-[auto_1fr] gap-x-2.5 gap-y-1 text-sm">
                <dt className="pt-0.5 text-2xs text-faint">{t('releases.proposals.now')}</dt>
                <dd className="selectable text-dim whitespace-pre-wrap">
                  {field.current.trim() === '' ? (
                    <span className="text-faint">{t('releases.meta.empty')}</span>
                  ) : (
                    field.current
                  )}
                </dd>
                <dt className="pt-0.5 text-2xs text-accent-2">
                  {t('releases.proposals.proposed')}
                </dt>
                <dd className="selectable whitespace-pre-wrap">
                  {field.proposed.trim() === '' ? (
                    <span className="text-faint">{t('releases.meta.empty')}</span>
                  ) : (
                    field.proposed
                  )}
                </dd>
              </dl>
            </div>
          </div>
        )
      })}

      <div className="flex flex-wrap items-center gap-1.5">
        <Button
          size="sm"
          variant="primary"
          disabled={busy || chosen.length === 0}
          onClick={() => take.mutate({ items: chosen })}
        >
          {t('releases.proposals.takeChosen')}
        </Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => dismiss.mutate()}>
          {t('releases.proposals.dismiss')}
        </Button>
      </div>
    </section>
  )
}
