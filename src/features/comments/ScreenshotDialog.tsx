import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ScanText } from 'lucide-react'
import { startScreenshotTask } from '@/lib/api/comments'
import { commentAction } from '@/lib/actions'
import { recallChannel, rememberChannel } from '@/lib/comments'
import { today } from '@/lib/month'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { publicationKinds, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/AppDialog'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { ChannelField } from '@/features/comments/ChannelField'

interface Props {
  /** The pictures pasted, read as one batch (v0.162); none keeps the dialog
   *  closed. */
  files: readonly File[]
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
export function ScreenshotDialog({ files, onClose, channels, channel, workId }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const reader = commentAction(config.prompts, 'comment')

  const [where, setWhere] = useState('')
  const [work, setWork] = useState<string | null>(workId ?? null)
  const [picking, setPicking] = useState(false)
  const [openedFor, setOpenedFor] = useState<readonly File[]>([])

  // Each batch starts from the channel in view, or the last one used here:
  // comments arrive in runs from one place.
  if (files.length > 0 && openedFor !== files) {
    setOpenedFor(files)
    setWhere(channel ?? recallChannel())
    setWork(workId ?? null)
  }

  const previews = useMemo(() => files.map((one) => URL.createObjectURL(one)), [files])
  useEffect(
    () => () => {
      for (const one of previews) URL.revokeObjectURL(one)
    },
    [previews],
  )

  const chosen = useQuery({
    ...queries.work(work ?? ''),
    enabled: work !== null,
  })

  const read = useAppMutation({
    mutationFn: async () => {
      if (files.length === 0 || reader === undefined) throw new Error('nothing to read')
      // One reading per picture, all from the one channel and work.
      for (const one of files) {
        const bytes = new Uint8Array(await one.arrayBuffer())
        await startScreenshotTask({
          bytes,
          name: one.name || `comment.${one.type.split('/')[1] ?? 'png'}`,
          channel: where.trim(),
          workId: work,
          action: reader.key,
          today: today(),
        })
      }
    },
    failure: 'comments.readFailed',
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: () => {
      rememberChannel(where)
      say.info(t('comments.readingMany', { count: files.length }))
      onClose()
    },
  })

  return (
    <>
      <Dialog
        open={files.length > 0}
        onOpenChange={(open) => {
          if (!open) onClose()
        }}
        title={t('comments.screenshotTitle')}
        description={reader === undefined ? t('comments.noReader') : t('comments.screenshotBody')}
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
          {previews.length === 1 ? (
            <img
              src={previews[0]}
              alt={t('comments.screenshotAlt')}
              className="max-h-56 w-full rounded-lg border border-line object-contain"
            />
          ) : (
            <div className="grid grid-cols-3 gap-1.5">
              {previews.map((one, index) => (
                <img
                  key={one}
                  src={one}
                  alt={t('comments.screenshotAltOf', { n: index + 1 })}
                  className="h-24 w-full rounded-md border border-line object-cover"
                />
              ))}
            </div>
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
                <Button size="sm" onClick={() => setWork(null)}>
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
        // Comments are left under what goes out, never under a song.
        kinds={publicationKinds(config).map((kind) => kind.key)}
        onPick={(picked) => setWork(picked.work_id)}
      />
    </>
  )
}
