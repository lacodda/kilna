import { useTranslation } from 'react-i18next'
import { startTask } from '@/lib/api/assistant'
import type { Dismissal } from '@/lib/api/types'
import { answeredOn, type Finding } from '@/lib/findings'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { PutAway } from '@/features/dashboard/PutAway'
import { Widget } from '@/features/dashboard/Widget'

interface Props {
  /** What stands and is said nowhere else on the screen (`lib/dashboard` `aside`). */
  findings: readonly Finding[]
  /** What the person has put away, for the way back. */
  putAway: readonly Dismissal[]
  onSelect: (workId: string, tab?: string) => void
  onDismiss: (finding: Finding) => void
  onRestore: (row: Dismissal) => void
  dismissing: boolean
}

/**
 * What the workspace noticed that is not a decision: standing complaints, each
 * with a way to the work, the profile action that answers it when there is
 * one, and "Heard it".
 *
 * "Heard it" is a word on a button now. It used to be a cross at the end of a
 * dashed pill, which read as "delete" - and a finding is not deleted, it is
 * put away until its complaint changes.
 *
 * Read-only about the workspace, as since v0.28: the only thing a finding can
 * start is a profile action, which lands in a chat like every other task.
 */
export function FindingsWidget({
  findings,
  putAway,
  onSelect,
  onDismiss,
  onRestore,
  dismissing,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()

  const start = useAppMutation({
    mutationFn: ({ workId, action }: { workId: string; action: string }) =>
      startTask(workId, action),
    refresh: [keys.activeTasks, keys.taskQueue, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
  })

  return (
    <Widget
      title={t('findings.title')}
      scroll={t('findings.title')}
      attention={findings.length > 0}
      aside={
        <span className="shrink-0 text-xs text-faint" title={t('findings.readOnly')}>
          {t('findings.readOnlyShort')}
        </span>
      }
    >
      {findings.length === 0 ? (
        <p className="text-sm text-faint">{t('findings.none')}</p>
      ) : (
        <ul className="flex flex-col">
          {findings.map((finding) => (
            <li
              key={`${finding.kind}:${finding.workId}`}
              className="flex min-w-0 flex-col items-start gap-1.5 border-b border-line py-2 first:pt-0 last:border-b-0"
            >
              {/* The complaint is the way to the work it names, so it is
                  pressed like a link; it reads as a sentence, and wraps. */}
              <Button
                variant="link"
                onClick={() => onSelect(finding.workId, answeredOn(finding.kind))}
                className="text-left text-sm whitespace-normal text-text"
              >
                {t(`findings.kind.${finding.kind}`, { title: finding.title })}
              </Button>
              <div className="flex flex-wrap items-center gap-1.5">
                {finding.action !== undefined && (
                  <Button
                    size="xs"
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
                <Button
                  size="xs"
                  variant="ghost"
                  title={t('findings.dismissHint')}
                  disabled={dismissing}
                  onClick={() => onDismiss(finding)}
                >
                  {t('findings.dismiss')}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}

      {putAway.length > 0 && <PutAway rows={putAway} onRestore={onRestore} />}
    </Widget>
  )
}
