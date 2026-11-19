import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { QuotaFinding, QuotaSubject } from '@/lib/api/types'
import { formatNumber } from '@/lib/format'
import { queries } from '@/lib/query/queries'
import { Dialog } from '@/components/AppDialog'
import { Loaded } from '@/components/Loaded'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { SkeletonList } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** What is checked: a work, a collection, a month, or - empty - the space. */
  subject: QuotaSubject
  title: string
}

/**
 * The space's rules checked over a subject (v0.151, FR-20 20.2): each rule a
 * breach was found of, with the works it was found in - what was measured
 * and what the rule wants. Asked afresh each time it opens.
 */
export function QuotaReportDialog({ open, onOpenChange, subject, title }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const report = useQuery({ ...queries.quotas(subject), enabled: open })

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={title} size="lg">
      <Loaded query={report} skeleton={<SkeletonList rows={3} />} plain>
        {(data) => (
          <div className="flex flex-col gap-3">
            <p className="text-sm text-dim">
              {t('quota.summary', {
                rules: data.rules,
                works: data.works,
                count: data.findings.length,
              })}
            </p>
            {data.rules === 0 ? (
              <EmptyState plain title={t('quota.noRules')} body={t('quota.noRulesHint')} />
            ) : data.findings.length === 0 ? (
              <EmptyState plain title={t('quota.allHold')} />
            ) : (
              <ul className="flex flex-col gap-2.5">
                {groupByRule(data.findings).map(([ruleId, found]) => (
                  <li key={ruleId} className="flex flex-col gap-1">
                    <span className="flex items-center gap-1.5 text-sm">
                      <span
                        aria-hidden
                        className={cn(
                          'size-2 rounded-full',
                          found[0]?.severity === 'error' ? 'bg-bad' : 'bg-warn',
                        )}
                      />
                      <b className="font-semibold">{found[0]?.label}</b>
                      <span className="font-mono text-2xs text-faint">{found[0]?.field}</span>
                    </span>
                    <ul className="flex flex-col gap-0.5 pl-3.5">
                      {found.map((one, index) => (
                        <li
                          key={`${one.work_id ?? ''}:${one.value ?? ''}:${String(index)}`}
                          className="flex flex-wrap items-baseline gap-1.5 text-xs"
                        >
                          {one.work_id !== null && one.title !== null ? (
                            <Button
                              variant="link"
                              className="text-xs"
                              onClick={() => {
                                onOpenChange(false)
                                void navigate(`/works/${one.work_id ?? ''}`)
                              }}
                            >
                              {one.title}
                            </Button>
                          ) : (
                            <span className="text-dim">{t(`quota.together.${one.scope}`)}</span>
                          )}
                          {one.value !== null && (
                            <span className="rounded-sm bg-soft px-1 font-mono text-2xs">
                              {one.value}
                            </span>
                          )}
                          <span className="font-mono text-2xs text-faint">
                            {one.metric === 'forbidden_terms'
                              ? t('quota.saysForbidden')
                              : t('quota.measured', {
                                  measured: formatNumber(
                                    one.measured,
                                    one.metric === 'share' ? 1 : 0,
                                  ),
                                  expected: one.expected,
                                })}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </Loaded>
    </Dialog>
  )
}

/** The findings by rule, in the order the rules came. */
function groupByRule(findings: readonly QuotaFinding[]): [string, QuotaFinding[]][] {
  const groups = new Map<string, QuotaFinding[]>()
  for (const one of findings) groups.set(one.rule_id, [...(groups.get(one.rule_id) ?? []), one])
  return [...groups.entries()]
}
