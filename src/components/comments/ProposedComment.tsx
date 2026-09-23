import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, ScanText, X } from 'lucide-react'
import {
  applyProposal,
  catalogue,
  dismissProposal,
  type CommentProposal,
  type PendingCommentProposal,
} from '@/lib/api'
import { workByTitle } from '@/lib/comments'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { DatePicker } from '@/components/ui/DatePicker'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { PickWorkDialog } from '@/components/shell/PickWorkDialog'
import { ChannelField } from '@/components/comments/ChannelField'

interface Props {
  pending: PendingCommentProposal & { proposal: CommentProposal }
  channels: string[]
  onKept: (commentId: string) => void
}

/**
 * A comment read off a screenshot, waiting to be kept.
 *
 * Every field is open to correction before keeping — a misread name, a wrong
 * day, the work the picture's title pointed at — because keeping is the
 * person's decision (ADR 0018) and a reading is only a reading. The work is
 * suggested from the title the picture showed, when exactly one work goes by
 * it, and never chosen silently.
 */
export function ProposedComment({ pending, channels, onKept }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const read = pending.proposal

  const works = useQuery({ queryKey: keys.catalogue, queryFn: catalogue })
  const suggested =
    read.work_id ?? workByTitle(read.about, (works.data ?? []).map((w) => ({ id: w.work_id, title: w.title })))

  const [channel, setChannel] = useState(read.channel)
  const [author, setAuthor] = useState(read.author ?? '')
  const [body, setBody] = useState(read.body)
  const [day, setDay] = useState(read.commented_on ?? '')
  const [work, setWork] = useState<string | null | undefined>(undefined)
  const [picking, setPicking] = useState(false)
  // Until the person says otherwise, the work is the suggestion.
  const chosen = work === undefined ? (suggested ?? null) : work
  const title = (works.data ?? []).find((w) => w.work_id === chosen)?.title

  const settle = () => {
    for (const key of [keys.comments, keys.pendingProposals, keys.transcripts]) {
      void client.invalidateQueries({ queryKey: key })
    }
  }

  const keep = useMutation({
    mutationFn: () =>
      applyProposal(pending.message_id, {
        comment: {
          channel: channel.trim(),
          body: body.trim(),
          author: author.trim() === '' ? null : author.trim(),
          commented_on: day === '' ? null : day,
          work_id: chosen,
        },
      }),
    onSuccess: (applied) => {
      settle()
      if (applied.comment !== undefined) onKept(applied.comment)
    },
    onError: (cause) => say.failedTo(t('comments.saveFailed'), cause),
  })

  const drop = useMutation({
    mutationFn: () => dismissProposal(pending.message_id),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('assistant.dismissFailed'), cause),
  })

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-accent/40 bg-accent-soft/40 p-2.5">
      <p className="flex items-center gap-1.5 text-[11px] font-medium text-accent-2">
        <ScanText aria-hidden className="size-3.5" />
        {t('comments.readOff')}
      </p>
      <Textarea
        autoResize
        maxRows={8}
        rows={2}
        value={body}
        onChange={(event) => setBody(event.target.value)}
        aria-label={t('comments.body')}
        className="text-[12.5px]"
      />
      <div className="flex gap-1.5">
        <Input
          value={author}
          onChange={(event) => setAuthor(event.target.value)}
          placeholder={t('comments.authorPlaceholder')}
          aria-label={t('comments.author')}
          className="h-8 text-xs"
        />
        <DatePicker
          value={day}
          onChange={setDay}
          placeholder={t('comments.dayPlaceholder')}
          aria-label={t('comments.day')}
          className="h-8 w-32 shrink-0 text-xs"
        />
      </div>
      <ChannelField value={channel} onChange={setChannel} known={channels} />
      <div className="flex items-center gap-1.5 text-[11px] text-dim">
        <span className="min-w-0 flex-1 truncate">
          {chosen === null
            ? read.about === undefined
              ? t('comments.aboutNothing')
              : t('comments.aboutUnmatched', { title: read.about })
            : t('comments.aboutWork', { title: title ?? '…' })}
        </span>
        <Button size="sm" className="h-6 px-2 text-[11px]" onClick={() => setPicking(true)}>
          {t('comments.pickWork')}
        </Button>
      </div>
      <div className="flex justify-end gap-1.5">
        <Button size="sm" variant="icon" disabled={drop.isPending} onClick={() => drop.mutate()}>
          <X aria-hidden />
          {t('comments.discard')}
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={channel.trim() === '' || body.trim() === '' || keep.isPending}
          onClick={() => keep.mutate()}
        >
          <Check aria-hidden />
          {t('comments.keep')}
        </Button>
      </div>
      <PickWorkDialog
        open={picking}
        onOpenChange={setPicking}
        title={t('comments.pickWorkTitle')}
        onPick={(picked) => setWork(picked.work_id)}
      />
    </div>
  )
}
