import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Applied, MonthProposal } from '@/lib/api/types'
import { checkMonthProposal } from '@/lib/api/canon'
import { formatNumber } from '@/lib/format'
import { keys } from '@/lib/query/keys'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { ProposalCard } from '@/features/assistant/ProposalCard'
import { cn } from '@/lib/utils'

interface Props {
  messageId: string
  proposal: MonthProposal
  applied: Applied | null
  dismissed: boolean
}

/**
 * A month's drafts an agent wrote in one pass (v0.153, FR-20 20.4), with the
 * checklist the space's quotas give over the month as it would stand - before
 * anything is written - and the one button that applies all of it: every
 * work renamed and filled, its texts kept, the bank's items it takes marked
 * used. One undo takes it back. The message above reads each work.
 */
export function ProposedMonth({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const count = proposal.items.length
  const check = useQuery({
    queryKey: ['canon', 'quotas', 'proposal', messageId],
    queryFn: () => checkMonthProposal(messageId),
    enabled: applied === null && !dismissed,
  })
  const apply = useApplyProposal({
    messageId,
    message: t('month.applied', { count }),
    refresh: [keys.works, keys.catalogue, keys.collections, keys.canon, keys.calendar],
  })
  const findings = check.data?.quotas.findings ?? []
  const taken = check.data?.bank ?? []
  const said = check.data?.register ?? []
  const breaches = findings.filter((one) => one.severity === 'error').length

  return (
    <ProposalCard
      messageId={messageId}
      title={t('month.proposed', { count })}
      applied={applied}
      dismissed={dismissed}
      applyLabel={t('month.applyAll', { count })}
      onApply={() => {
        apply.mutate(undefined)
      }}
      applying={apply.isPending}
      appliedLabel={t('month.appliedMark')}
    >
      {applied === null && !dismissed && (
        <section aria-label={t('month.checklist')} className="flex flex-col gap-1 text-xs">
          <b className="font-semibold">{t('month.checklist')}</b>
          {check.isPending ? (
            <span className="text-faint">{t('month.checking')}</span>
          ) : check.data === undefined ? null : check.data.quotas.rules === 0 ? (
            <span className="text-faint">{t('quota.noRules')}</span>
          ) : findings.length === 0 ? (
            <span className="text-good">
              {t('month.allHold', { rules: check.data.quotas.rules })}
            </span>
          ) : (
            <>
              <span className={breaches > 0 ? 'text-bad' : 'text-warn'}>
                {t('month.breaches', { count: findings.length })}
              </span>
              <ul className="flex flex-col gap-0.5">
                {findings.map((one, index) => (
                  <li
                    key={`${one.rule_id}:${one.work_id ?? ''}:${String(index)}`}
                    className="flex flex-wrap items-baseline gap-1.5"
                  >
                    <span
                      aria-hidden
                      className={cn(
                        'size-1.5 rounded-full',
                        one.severity === 'error' ? 'bg-bad' : 'bg-warn',
                      )}
                    />
                    <span>{one.label}</span>
                    <span className="text-dim">
                      {one.title ?? t(`quota.together.${one.scope}`)}
                    </span>
                    {one.value !== null && (
                      <span className="rounded-sm bg-soft px-1 font-mono text-2xs">
                        {one.value}
                      </span>
                    )}
                    {one.metric !== 'forbidden_terms' && (
                      <span className="font-mono text-2xs text-faint">
                        {t('quota.measured', {
                          measured: formatNumber(one.measured, one.metric === 'share' ? 1 : 0),
                          expected: one.expected,
                        })}
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            </>
          )}
          {taken.length > 0 && (
            <>
              <b className="mt-1 font-semibold text-warn">{t('month.bankTaken')}</b>
              <ul className="flex flex-col gap-0.5">
                {taken.map((one) => (
                  <li key={`${one.work_id}:${one.item}`} className="flex flex-wrap gap-1.5">
                    <span>{one.item}</span>
                    <span className="text-dim">{one.title}</span>
                    <span className="text-2xs text-faint">
                      {one.state === 'locked'
                        ? t('month.locked')
                        : t('month.restingUntil', { month: one.until ?? '' })}
                    </span>
                  </li>
                ))}
              </ul>
            </>
          )}
          {said.length > 0 && (
            <>
              <b className="mt-1 font-semibold text-warn">{t('month.registerSaid')}</b>
              <ul className="flex flex-col gap-0.5">
                {said.map((one) => (
                  <li
                    key={`${one.work_id}:${one.word}:${one.scope}`}
                    className="flex flex-wrap gap-1.5"
                  >
                    <span className="rounded-sm bg-soft px-1 font-mono text-2xs">{one.word}</span>
                    <span className="text-dim">{one.title}</span>
                    <span className="text-2xs text-faint">{t(`register.scopes.${one.scope}`)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </section>
      )}
    </ProposalCard>
  )
}
