import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { createComment } from '@/lib/api/comments'
import { recallChannel, rememberChannel } from '@/lib/comments'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { publicationKinds, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/DatePicker'
import { Dialog } from '@/components/AppDialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { ChannelField } from '@/features/comments/ChannelField'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  channels: string[]
  /** The channel being looked at, offered first. */
  channel?: string
  /** Fixed when the comment is added from a work's card. */
  workId?: string
  onCreated: (id: string) => void
}

/** A comment typed in by hand — the way in when there is no screenshot. */
export function NewCommentDialog({
  open,
  onOpenChange,
  channels,
  channel,
  workId,
  onCreated,
}: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()

  const [where, setWhere] = useState('')
  const [author, setAuthor] = useState('')
  const [body, setBody] = useState('')
  const [day, setDay] = useState('')
  const [work, setWork] = useState<string | null>(null)
  const [picking, setPicking] = useState(false)
  const [wasOpen, setWasOpen] = useState(false)

  // Each opening starts clean, from the channel in view or the last one used.
  if (open && !wasOpen) {
    setWasOpen(true)
    setWhere(channel ?? recallChannel())
    setAuthor('')
    setBody('')
    setDay('')
    setWork(workId ?? null)
  } else if (!open && wasOpen) {
    setWasOpen(false)
  }

  const chosen = useQuery({
    ...queries.work(work ?? ''),
    enabled: work !== null,
  })

  const create = useAppMutation({
    mutationFn: () =>
      createComment({
        channel: where.trim(),
        body: body.trim(),
        author: author.trim() === '' ? undefined : author.trim(),
        commented_on: day === '' ? undefined : day,
        work_id: work ?? undefined,
      }),
    failure: 'comments.saveFailed',
    refresh: refresh.comment,
    onSuccess: (created) => {
      rememberChannel(where)
      onOpenChange(false)
      onCreated(created.id)
    },
  })

  const ready = where.trim() !== '' && body.trim() !== ''

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={onOpenChange}
        title={t('comments.newTitle')}
        description={t('comments.newBody')}
        footer={
          <Button
            type="submit"
            form="new-comment"
            variant="primary"
            disabled={!ready || create.isPending}
          >
            {t('comments.keep')}
          </Button>
        }
      >
        <form
          id="new-comment"
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            if (ready) create.mutate()
          }}
        >
          <ChannelField value={where} onChange={setWhere} known={channels} autoFocus />
          <div className="flex gap-2">
            <Input
              value={author}
              onChange={(event) => setAuthor(event.target.value)}
              placeholder={t('comments.authorPlaceholder')}
              aria-label={t('comments.author')}
            />
            <DatePicker
              value={day}
              onChange={setDay}
              placeholder={t('comments.dayPlaceholder')}
              aria-label={t('comments.day')}
              className="w-40 shrink-0"
            />
          </div>
          <Textarea
            rows={4}
            value={body}
            onChange={(event) => setBody(event.target.value)}
            placeholder={t('comments.bodyPlaceholder')}
            aria-label={t('comments.body')}
          />
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
