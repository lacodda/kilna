import { createElement, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Eye, LoaderCircle } from 'lucide-react'
import { startReleaseTask } from '@/lib/api/releases'
import type { PromptTemplate } from '@/lib/api/types'
import { actionIconOf } from '@/lib/actionIcon'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'

interface Props {
  releaseId: string
  /** The profile's action that writes a release's meta. */
  action: PromptTemplate
  /** Whether it is writing this release's meta now, by the backend's list. */
  writing: boolean
}

/**
 * "Release meta": the profile's action that writes what a release goes out
 * under, started on this release, with the eye beside it that shows what it
 * would send - the same pair a work's action bar draws.
 *
 * The run goes in the background and its answer lands in the fields: the
 * empty ones are filled, the ones already started wait under them as a
 * proposal (`ReleaseProposals`). Nothing here waits for it; the button says it
 * is writing for as long as the backend says so, from any screen.
 */
export function ReleaseMetaButton({ releaseId, action, writing }: Props) {
  const { t } = useTranslation()
  const [previewing, setPreviewing] = useState(false)

  const start = useAppMutation({
    mutationFn: () => startReleaseTask(releaseId, action.key),
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => say.info(t('assistant.taskStarted', { title: started.title })),
  })

  // A click still in flight counts as writing: the list of running tasks has
  // not heard of it yet, and a second click would only be refused.
  const working = writing || start.isPending
  const label = sayLabel(action.label)
  const description = sayLabel(action.description)

  return (
    <span className="inline-flex items-stretch">
      <Button
        size="sm"
        className="rounded-r-none"
        title={description === '' ? label : `${label} — ${description}`}
        disabled={working}
        onClick={() => start.mutate()}
      >
        {working ? (
          <LoaderCircle aria-hidden className="animate-spin" />
        ) : (
          createElement(actionIconOf(action), { 'aria-hidden': true })
        )}
        {working ? t('releases.task.writing') : label}
      </Button>
      <Button
        size="sm"
        className="rounded-l-none border-l-0 px-1.5"
        title={t('assistant.previewTask', { label })}
        aria-label={t('assistant.previewTask', { label })}
        disabled={working}
        onClick={() => setPreviewing(true)}
      >
        <Eye aria-hidden />
      </Button>
      {previewing && (
        <TaskPreviewDialog
          open
          onOpenChange={setPreviewing}
          target={{ on: 'release', id: releaseId }}
          action={action}
          onStarted={() => setPreviewing(false)}
        />
      )}
    </span>
  )
}
