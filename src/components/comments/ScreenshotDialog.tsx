import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ScanText } from 'lucide-react'
import { getWork, startScreenshotTask } from '@/lib/api'
import { commentAction } from '@/lib/actions'
import { recallChannel, rememberChannel } from '@/lib/comments'
import { today } from '@/lib/month'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { PickWorkDialog } from '@/components/shell/PickWorkDialog'
import { ChannelField } from '@/components/comments/ChannelField'

interface Props {
  /** The picture pasted; null keeps the dialog closed. */
  file: File | null
  onClose: () => void
  /** The channels already used, for the chips. */
  channels: string[]
  /** The channel being looked at, offered first when there is one. */
  channel?: string
  /** The work it is about, when it was pasted on a work's card: fixed. */
  workId?: string
}

/**
 * A screenshot of a comment, about to be read.
 *
 * Asks the one thing the picture cannot say — which channel it came from —
 * and, off a work's card, which work it is about. Then the reading runs in
 * the background: the person can paste the next one straight away, and each
 * comes back as something to keep on the comments screen. The picture is
 * not kept (migration 0027).
 */
export function ScreenshotDialog({ file, onClose, channels, channel, workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { config } = useProfile()
  const reader = commentAction(config.prompts, 'comment')

  const [where, setWhere] = useState('')
  const [work, setWork] = useState<string | null>(workId ?? null)
  const [picking, setPicking] = useState(false)
  const [openedFor, setOpenedFor] = useState<File | null>(null)

  // Each picture starts from the channel in view, or the last one used here:
  // comments arrive in runs from one place.
  if (file !== null && openedFor !== file) {
    setOpenedFor(file)
    setWhere(channel ?? recallChannel())
    setWork(workId ?? null)
  }

  const preview = useMemo(() => (file === null ? null : URL.createObjectURL(file)), [file])
  useEffect(
    () => () => {
      if (preview !== null) URL.revokeObjectURL(preview)
    },
    [preview],
  )

  const chosen = useQuery({
    queryKey: keys.work(work ?? ''),
    queryFn: () => getWork(work!),
    enabled: work !== null,
  })

  const read = useMutation({
    mutationFn: async () => {
      if (file === null || reader === undefined) throw new Error('nothing to read')
      const bytes = new Uint8Array(await file.arrayBuffer())
      return startScreenshotTask({
        bytes,
        name: file.name || `comment.${file.type.split('/')[1] ?? 'png'}`,
        channel: where.trim(),
        workId: work,
        action: reader.key,
        today: today(),
      })
    },
    onSuccess: () => {
      rememberChannel(where)
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      void client.invalidateQueries({ queryKey: keys.allChats })
      say.info(t('comments.reading'))
      onClose()
    },
    onError: (cause) => say.failedTo(t('comments.readFailed'), cause),
  })

  return (
    <>
      <Dialog
        open={file !== null}
        onOpenChange={(open) => {
          if (!open) onClose()
        }}
        title={t('comments.screenshotTitle')}
        description={
          reader === undefined ? t('comments.noReader') : t('comments.screenshotBody')
        }
        footer={
          <Button
            type="submit"
            form="read-screenshot"
            variant="primary"
            disabled={reader === undefined || where.trim() === '' || read.isPending}
          >
            <ScanText aria-hidden />
            {t('comments.read')}
          </Button>
        }
      >
        <form
          id="read-screenshot"
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            if (where.trim() !== '' && reader !== undefined) read.mutate()
          }}
        >
          {preview !== null && (
            <img
              src={preview}
              alt={t('comments.screenshotAlt')}
              className="max-h-56 w-full rounded-lg border border-line object-contain"
            />
          )}
          <ChannelField value={where} onChange={setWhere} known={channels} autoFocus />
          {workId === undefined && (
            <div className="flex items-center gap-2 text-xs text-dim">
              <span className="min-w-0 truncate">
                {work === null
                  ? t('comments.aboutNothing')
                  : t('comments.aboutWork', { title: chosen.data?.title ?? '…' })}
              </span>
              <Button size="sm" className="ml-auto" onClick={() => setPicking(true)}>
                {t('comments.pickWork')}
              </Button>
              {work !== null && (
                <Button size="sm" variant="icon" onClick={() => setWork(null)}>
                  {t('comments.noWork')}
                </Button>
              )}
            </div>
          )}
        </form>
      </Dialog>
      <PickWorkDialog
        open={picking}
        onOpenChange={setPicking}
        title={t('comments.pickWorkTitle')}
        onPick={(picked) => setWork(picked.work_id)}
      />
    </>
  )
}
