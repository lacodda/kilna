import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { startTask } from '@/lib/api/assistant'
import { startCommentTask } from '@/lib/api/comments'
import { startStyleTask } from '@/lib/api/styles'
import type { PromptTemplate, StartedTask } from '@/lib/api/types'
import { humanError } from '@/lib/errors'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { Field, FieldGroup } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

/**
 * What a task is aimed at: a work (on one of its versions, scenes or prompt
 * blocks), a style brick being described, or a comment being answered. The
 * three are composed and started by different commands, and only a work's
 * task takes reference files.
 */
export type TaskTarget =
  | {
      on: 'work'
      workId: string
      versionId?: string
      sceneId?: string
      /** One prompt block of that scene, when the action is aimed at one. */
      block?: string
    }
  | { on: 'style'; id: string }
  | { on: 'comment'; id: string }

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  target: TaskTarget
  action: PromptTemplate
  /** What must be stored before the task reads it - a steer typed into a
   *  style, a reply typed under a comment. Run before the task starts. */
  before?: () => Promise<unknown>
  onStarted: (started: StartedTask) => void
}

/** The paths typed in the box, one per line, blanks dropped. */
const pathsOf = (text: string): string[] =>
  text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '')

/**
 * What a task would send, before it is sent.
 *
 * The text is composed by the backend with the very call that starts a task,
 * so what is shown here is what goes — the message with the work's text
 * filled in and the instruction appended, and the method the run is briefed
 * with. Reference files are named by path; the run is given leave to read
 * their folders and told to read them before answering. A path that is not
 * a file is refused here, in the preview, rather than by a run ten minutes
 * later.
 *
 * A style's and a comment's task are previewed the same way since v0.79:
 * each screen that starts one offers to read it first, as a work's action
 * bar does.
 */
export function TaskPreviewDialog({
  open: isOpen,
  onOpenChange,
  target,
  action,
  before,
  onStarted,
}: Props) {
  const { t } = useTranslation()
  const [attachmentsText, setAttachmentsText] = useState('')
  // What the preview is composed against: set when the box is left, so a
  // half-typed path is not sent to be checked keystroke by keystroke.
  const [attachments, setAttachments] = useState<string[]>([])

  const read =
    target.on === 'work'
      ? queries.taskPreview(target.workId, action.key, {
          versionId: target.versionId,
          sceneId: target.sceneId,
          block: target.block,
          attachments,
        })
      : target.on === 'style'
        ? queries.styleTaskPreview(target.id, action.key)
        : queries.commentTaskPreview(target.id, action.key)
  const preview = useQuery({ ...read, enabled: isOpen, staleTime: 0, retry: false })

  const start = useAppMutation({
    mutationFn: async () => {
      await before?.()
      switch (target.on) {
        case 'work':
          return startTask(target.workId, action.key, {
            versionId: target.versionId,
            sceneId: target.sceneId,
            block: target.block,
            attachments: pathsOf(attachmentsText),
          })
        case 'style':
          return startStyleTask(target.id, action.key)
        case 'comment':
          return startCommentTask(target.id, action.key)
      }
    },
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
      onStarted(started)
    },
  })

  const choose = async () => {
    const chosen = await open({ multiple: true, title: t('assistant.attachChoose') })
    const picked = chosen === null ? [] : Array.isArray(chosen) ? chosen : [chosen]
    if (picked.length === 0) return
    const next = [...pathsOf(attachmentsText), ...picked.filter((p) => typeof p === 'string')]
    const unique = next.filter((path, index) => next.indexOf(path) === index)
    setAttachmentsText(unique.join('\n'))
    setAttachments(unique)
  }

  // A backend refusal is an object; String() of it read "[object Object]".
  const problem = preview.error === null ? null : humanError(preview.error)

  return (
    <Dialog
      open={isOpen}
      onOpenChange={onOpenChange}
      title={t('assistant.previewTitle', { label: action.label })}
      description={t('assistant.previewHint')}
      size="xl"
      footer={
        <Button
          variant="primary"
          disabled={start.isPending || preview.isError || preview.isPending}
          onClick={() => {
            start.mutate()
          }}
        >
          {t('assistant.previewStart')}
        </Button>
      }
    >
      <div className="flex max-h-[60vh] flex-col gap-3 overflow-y-auto pr-1">
        {problem !== null && (
          <p role="alert" className="text-sm text-bad">
            {problem}
          </p>
        )}
        {preview.data !== undefined && (
          <>
            <FieldGroup label={t('assistant.previewMessage')}>
              <pre className="selectable max-h-72 overflow-auto whitespace-pre-wrap rounded-xl border border-line bg-soft px-3 py-2 font-mono text-xs">
                {preview.data.prompt}
              </pre>
            </FieldGroup>
            {preview.data.method !== undefined && (
              <details className="flex flex-col gap-1">
                <summary className="cursor-pointer caption">{t('assistant.previewMethod')}</summary>
                <pre className="selectable mt-1 max-h-72 overflow-auto whitespace-pre-wrap rounded-xl border border-line bg-soft px-3 py-2 font-mono text-xs">
                  {preview.data.method}
                </pre>
              </details>
            )}
          </>
        )}
        {/* Reference files travel with a work's task only: a style is
            described from its own references, a reply from its comment. */}
        {target.on === 'work' && (
          <>
            <Field label={t('assistant.attachments')} help={t('assistant.attachmentsHint')}>
              <Textarea
                autoResize
                maxRows={6}
                rows={2}
                className="font-mono text-xs"
                value={attachmentsText}
                placeholder={t('assistant.attachmentsPlaceholder')}
                onChange={(event) => setAttachmentsText(event.target.value)}
                onBlur={(event) => setAttachments(pathsOf(event.target.value))}
              />
            </Field>
            <div>
              <Button size="sm" onClick={() => void choose()}>
                {t('assistant.attachChoose')}
              </Button>
            </div>
          </>
        )}
      </div>
    </Dialog>
  )
}
