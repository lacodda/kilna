import { useTranslation } from 'react-i18next'
import type { Fact } from '@/lib/api/types'
import { bankState, usesOf } from '@/lib/bank'
import { today } from '@/lib/month'
import { draftOf, expectedOf } from '@/lib/quota'
import { cn } from '@/lib/utils'

/** A rule of the quotas as its row reads (v0.151): its name, then what it
 *  measures and against what, over which works. */
export function QuotaWords({ fact }: { fact: Fact }) {
  const { t } = useTranslation()
  const rule = draftOf(fact.data)
  const parts = [
    rule.field === '' ? null : `\`${rule.field}\``,
    `${t(`canon.quota.metric.${rule.metric}`)} ${expectedOf(rule)}`,
    t(`canon.quota.scope.${rule.scope}`),
    rule.pattern === '' ? null : `/${rule.pattern}/`,
    rule.filterField === '' || rule.filterEquals === ''
      ? null
      : t(rule.filterNot ? 'canon.quota.filterSaidNot' : 'canon.quota.filterSaid', {
          field: rule.filterField,
          values: rule.filterEquals,
        }),
  ].filter((part) => part !== null)
  return (
    <>
      <span className="flex flex-wrap items-center gap-1.5">
        <b className="font-semibold">{fact.body}</b>
        {rule.severity === 'error' && (
          <span className="rounded-sm bg-bad-soft px-1 text-2xs text-bad">
            {t('canon.quota.severity.error')}
          </span>
        )}
        {!rule.active && (
          <span className="rounded-sm bg-soft px-1 text-2xs text-faint">
            {t('canon.quota.off')}
          </span>
        )}
      </span>
      <span className="block font-mono text-2xs leading-normal text-dim">{parts.join(' · ')}</span>
    </>
  )
}

/** An item of a bank as its row reads (v0.151): its name and group, where it
 *  stands today, what it is made of, and the works that used it. */
export function BankWords({ fact }: { fact: Fact }) {
  const { t } = useTranslation()
  const { state, until } = bankState(fact.data, today())
  const group = typeof fact.data.group === 'string' ? fact.data.group : ''
  const attributes = Array.isArray(fact.data.attributes)
    ? fact.data.attributes.filter((one): one is string => typeof one === 'string')
    : []
  const uses = usesOf(fact.data)
  const last = typeof fact.data.last_used_at === 'string' ? fact.data.last_used_at : null
  return (
    <>
      <span className="flex flex-wrap items-center gap-1.5">
        <b className="font-semibold">{fact.body}</b>
        {group !== '' && <span className="text-2xs text-faint">{group}</span>}
        <span
          className={cn(
            'rounded-sm px-1 text-2xs',
            state === 'available' && 'bg-good-soft text-good',
            state === 'cooldown' && 'bg-warn-soft text-warn',
            state === 'locked' && 'bg-soft text-faint',
          )}
        >
          {state === 'cooldown'
            ? t('canon.bank.restingUntil', { month: until ?? '' })
            : t(`canon.bank.state.${state}`)}
        </span>
      </span>
      {attributes.length > 0 && (
        <span className="block text-xs text-dim">{attributes.join(', ')}</span>
      )}
      {(uses.length > 0 || last !== null) && (
        <span
          className="block text-2xs text-faint"
          title={uses.map((one) => `${one.date} · ${one.title}`).join('\n')}
        >
          {uses.length > 0
            ? t('canon.bank.used', { count: uses.length, last: last ?? '' })
            : t('canon.bank.lastUsedOn', { day: last ?? '' })}
        </span>
      )}
    </>
  )
}
