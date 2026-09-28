import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { startTask } from '@/lib/api/assistant'
import { dismissFinding } from '@/lib/api/focus'
import type { Work } from '@/lib/api/types'
import { answeredOn, dismissalKey, type Finding } from '@/lib/findings'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Widget } from '@/features/work/tabs/overview/Widget'
import { useWorkFindings } from '@/features/work/tabs/overview/useWorkFindings'

/**
 * What needs a decision about this work: the widget catalogue's finding, in
 * the warm border of something that asks rather than tells.
 *
 * Each complaint is the way to the tab that answers it, and carries the
 * profile action that would, when the profile has one, and "Heard it" - the
 * dashboard's three moves, because they are the same complaints. It is drawn
 * only while there is one: an empty widget saying nothing is wrong is noise
 * on every healthy work (`applies`).
 */
export function FindingsWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const profile = useProfile()
  const standing = useWorkFindings(work.id)

  const start = useAppMutation({
    mutationFn: (action: string) => startTask(work.id, action),
    refresh: [keys.activeTasks, keys.taskQueue, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
  })

  const hide = useAppMutation({
    mutationFn: (finding: Finding) => dismissFinding(dismissalKey(finding)),
    refresh: refresh.focus,
  })

  return (
    <Widget caption={t('overview.attention')} tone="attention">
      <ul className="flex flex-col">
        {standing.map((finding) => {
          const tab = answeredOn(finding.kind)
          const said = t(`overview.finding.${finding.kind}`)
          return (
            <li
              key={`${finding.kind}:${finding.complaint}`}
              className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-line py-1.5 first:pt-0 last:border-b-0 last:pb-0"
            >
              <span className="min-w-0 flex-1 basis-48 text-sm text-text">
                {tab === undefined ? (
                  said
                ) : (
                  <Link
                    to={`/works/${work.id}/${tab}`}
                    className="text-text no-underline hover:underline"
                  >
                    {said}
                  </Link>
                )}
              </span>
              <span className="flex shrink-0 flex-wrap items-center gap-1.5">
                {finding.action !== undefined && (
                  <Button
                    size="xs"
                    variant="soft"
                    disabled={start.isPending}
                    title={t('findings.askHint')}
                    onClick={() => start.mutate(finding.action as string)}
                  >
                    {labelOf(profile.config.prompts, finding.action)}
                  </Button>
                )}
                <Button
                  size="xs"
                  title={t('findings.dismissHint')}
                  disabled={hide.isPending}
                  onClick={() => hide.mutate(finding)}
                >
                  {t('findings.dismiss')}
                </Button>
              </span>
            </li>
          )
        })}
      </ul>
    </Widget>
  )
}
