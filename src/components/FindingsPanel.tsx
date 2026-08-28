import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Sparkles } from 'lucide-react'
import { startTask, type ScheduledRelease, type ScoredWork } from '@/lib/api'
import { findings, type Finding } from '@/lib/findings'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/Button'
import { SectionLabel } from '@/components/ui/Panel'

interface Props {
  works: readonly ScoredWork[]
  calendar: readonly ScheduledRelease[]
  /** Kinds already spoken for by a section above; saying them twice is noise. */
  skip?: readonly Finding['kind'][]
  onSelect: (workId: string, tab?: string) => void
}

/**
 * What the workspace noticed about itself, and what to do about it.
 *
 * Read-only by construction. A finding names a work and a complaint; the only
 * thing it can start is a profile action, which lands in a chat of its own like
 * every other task. **Nothing here writes to the workspace** — the assistant
 * proposes, a person applies, the rule since v0.28.
 *
 * The dashboard passes `skip` for the kinds its own sections already show: a
 * work listed under "nothing has judged these yet" does not also need a line
 * saying it is unscored.
 */
export function FindingsPanel({ works, calendar, skip = [], onSelect }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const start = useMutation({
    mutationFn: ({ workId, action }: { workId: string; action: string }) =>
      startTask(workId, action),
    onSuccess: (started) => {
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      void client.invalidateQueries({ queryKey: keys.allChats })
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const hidden = new Set(skip)
  const list = findings(works, calendar, profile.config, today()).filter(
    (finding) => !hidden.has(finding.kind),
  )

  if (list.length === 0) return null

  return (
    <section className="flex flex-col gap-2">
      <SectionLabel>
        <Sparkles aria-hidden className="size-3.5" />
        {t('findings.title')}
        <span className="normal-case tracking-normal text-dim">{t('findings.readOnly')}</span>
      </SectionLabel>

      <div className="flex flex-wrap gap-2">
        {list.map((finding) => (
          <span
            key={`${finding.kind}:${finding.workId}`}
            className="flex items-center gap-2 rounded-xl border border-dashed border-line-2 px-3 py-1.5 text-[12.5px] text-dim"
          >
            <button
              type="button"
              onClick={() => onSelect(finding.workId, tabFor(finding.kind))}
              className="cursor-pointer text-left transition-colors hover:text-text"
            >
              {t(`findings.kind.${finding.kind}`, { title: finding.title })}
            </button>

            {finding.action !== undefined && (
              <Button
                size="sm"
                variant="soft"
                disabled={start.isPending}
                title={t('findings.askHint')}
                onClick={() => {
                  start.mutate({ workId: finding.workId, action: finding.action as string })
                }}
              >
                {labelOf(profile.config.prompts, finding.action)}
              </Button>
            )}
          </span>
        ))}
      </div>
    </section>
  )
}

/** Where a complaint is answered — the tab that closes it, when there is one. */
function tabFor(kind: Finding['kind']): string | undefined {
  switch (kind) {
    case 'unscored':
    case 'stale-score':
      return 'score'
    case 'ready-unscheduled':
    case 'weak-scheduled':
      return 'releases'
    case 'stale-draft':
      return undefined
  }
}
