import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Applied, MediaProposal } from '@/lib/api/types'
import { formatNumber } from '@/lib/format'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Checkbox } from '@/components/ui/checkbox'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: MediaProposal
  applied: Applied | null
  dismissed: boolean
}

/** The last step of a path, whichever slash it is written with. */
const nameOf = (path: string) => path.split(/[\\/]/).pop() ?? path

const itemOf = (index: number) => `media:${String(index)}`

/**
 * Files an assistant made or placed beside kilna, each for a work and as
 * what (v0.156, FR-21 21.1 p.4) - and, for pictures it matched by what they
 * show, why and how sure (v0.157, FR-21 21.2 p.2): read before anything
 * moves, ticked file by file, and moved into their works' folders on one
 * press - each checked, renamed after its work and given its role.
 */
export function ProposedMedia({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const all = proposal.items.map((_, index) => itemOf(index))
  const [chosen, setChosen] = useState<string[]>(all)
  const answered = applied !== null || dismissed
  const apply = useApplyProposal({
    messageId,
    message: t('receiver.proposalApplied', { count: chosen.length }),
    refresh: proposal.items.flatMap((item) => refresh.taken(item.work_id)),
  })

  const toggle = (item: string, on: boolean) =>
    setChosen(on ? [...chosen, item] : chosen.filter((one) => one !== item))

  return (
    <ProposalCard
      messageId={messageId}
      title={t('receiver.proposed', { count: all.length })}
      figure={answered ? undefined : String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('receiver.applyAll', { count: chosen.length })}
      onApply={() => {
        apply.mutate(chosen.length === all.length ? undefined : { items: chosen })
      }}
      applying={apply.isPending || chosen.length === 0}
      appliedLabel={t('receiver.proposalAppliedMark')}
    >
      <ul className="flex flex-col gap-1.5 text-xs">
        {proposal.items.map((item, index) => {
          const key = itemOf(index)
          return (
            <li key={key} className="flex items-start gap-2">
              {!answered && (
                <Checkbox
                  checked={chosen.includes(key)}
                  onCheckedChange={(on) => toggle(key, on)}
                  aria-label={t('receiver.takeItem', { name: nameOf(item.path) })}
                  className="mt-0.5"
                />
              )}
              <div className="flex min-w-0 flex-col">
                <span className="flex flex-wrap items-baseline gap-1.5">
                  <b className="font-semibold">{item.work_title}</b>
                  <span className="text-dim">{t(`receiver.what.${item.role}`)}</span>
                  <span className="min-w-0 truncate text-faint" title={item.path}>
                    {nameOf(item.path)}
                  </span>
                  {item.confidence !== undefined && (
                    <span className="text-faint">{`${formatNumber(item.confidence * 100, 0)}%`}</span>
                  )}
                </span>
                {item.reason !== undefined && item.reason.trim() !== '' && (
                  <span className="text-dim">{item.reason}</span>
                )}
              </div>
            </li>
          )
        })}
      </ul>
    </ProposalCard>
  )
}
