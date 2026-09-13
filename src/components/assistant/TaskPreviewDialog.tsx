import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { previewTask, startTask, type PromptTemplate } from '@/lib/api'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Field } from '@/components/ui/Field'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  workId: string
  action: PromptTemplate
  versionId?: string
  sceneId?: string
  onStarted: () => void
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
 */
export function TaskPreviewDialog({
  open: isOpen,
  onOpenChange,
  workId,
  action,
  versionId,
  sceneId,
  onStarted,
}: Props) {
  const { t } = useTranslation()
  const [attachmentsText, setAttachmentsText] = useState('')
  // What the preview is composed against: set when the box is left, so a
  // half-typed path is not sent to be checked keystroke by keystroke.
  const [attachments, setAttachments] = useState<string[]>([])

  const preview = useQuery({
    queryKey: ['task-preview', workId, action.key, versionId ?? '', sceneId ?? '', attachments],
    queryFn: () => previewTask(workId, action.key, { versionId, sceneId, attachments }),
    enabled: isOpen,
    staleTime: 0,
    retry: false,
  })

  const start = useMutation({
    mutationFn: () => startTask(workId, action.key, { versionId, sceneId, attachments: pathsOf(attachmentsText) }),
    onSuccess: (started) => {
      say.info(t('assistant.taskStarted', { title: started.title }))
      onStarted()
    },
    onError: (cause) => {
      say.failed(cause)
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

  const problem = preview.error === null ? null : String(preview.error)

  return (
    <Dialog
      open={isOpen}
      onOpenChange={onOpenChange}
      title={t('assistant.previewTitle', { label: action.label })}
      description={t('assistant.previewHint')}
      className="max-w-3xl"
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
            <Field label={t('assistant.previewMessage')}>
              <pre className="selectable max-h-72 overflow-auto whitespace-pre-wrap rounded-xl border border-line bg-soft px-3 py-2 font-mono text-xs">
                {preview.data.prompt}
              </pre>
            </Field>
            {preview.data.method !== undefined && (
              <details className="flex flex-col gap-1">
                <summary className="cursor-pointer text-2xs font-semibold uppercase tracking-caption text-faint">
                  {t('assistant.previewMethod')}
                </summary>
                <pre className="selectable mt-1 max-h-72 overflow-auto whitespace-pre-wrap rounded-xl border border-line bg-soft px-3 py-2 font-mono text-xs">
                  {preview.data.method}
                </pre>
              </details>
            )}
          </>
        )}
        <Field label={t('assistant.attachments')} hint={t('assistant.attachmentsHint')}>
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
      </div>
    </Dialog>
  )
}
