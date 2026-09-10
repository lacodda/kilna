import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Pin } from 'lucide-react'
import { type Work, pinTier, unpinTier } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { labelOf, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Dialog } from '@/components/ui/AppDialog'
import { Input } from '@/components/ui/input'
import { Select } from '@/components/ui/AppSelect'

interface Props {
  work: Work
  /** The tier the score itself arrives at, shown as what the pin overrides. */
  scored: string | null
}

/**
 * Holding a work at a tier by hand, and letting the score speak again.
 *
 * The machinery has been here since v0.50 - the column, the command, the
 * journal entry, the undo - with no way to reach it. What it needed was the
 * one thing a column cannot store: the question "why", asked at the moment of
 * pinning rather than remembered afterwards. A tier nobody can argue with
 * later is a tier with a sentence attached, so the reason is required.
 *
 * Unpinning is called "follow the facts" rather than "unpin": the point is not
 * that a flag comes off, it is that the score gets its say back.
 */
export function TierPin({ work, scored }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const client = useQueryClient()

  const [open, setOpen] = useState(false)
  const [tier, setTier] = useState(work.tier_pinned ?? scored ?? '')
  const [reason, setReason] = useState(work.tier_pin_reason ?? '')

  const pinned = work.tier_pinned !== null && work.tier_pinned !== undefined

  const settle = (updated: Work) => {
    client.setQueryData(keys.work(work.id), updated)
    for (const key of [keys.works, keys.catalogue, keys.journal]) {
      void client.invalidateQueries({ queryKey: key })
    }
  }

  const pin = useMutation({
    mutationFn: () => pinTier(work.id, tier, reason.trim()),
    onSuccess: (updated) => {
      settle(updated)
      setOpen(false)
      say.ok(t('toast.tierPinned'))
    },
    onError: (cause) => say.failedTo(t('toast.tierPinFailed'), cause),
  })

  const release = useMutation({
    mutationFn: () => unpinTier(work.id),
    onSuccess: (updated) => {
      settle(updated)
      say.ok(t('toast.tierUnpinned'))
    },
    onError: (cause) => say.failedTo(t('toast.tierPinFailed'), cause),
  })

  const submit = (event: FormEvent) => {
    event.preventDefault()
    // A pin without a reason is the thing this feature exists to prevent.
    if (tier === '' || reason.trim() === '') return
    pin.mutate()
  }

  const start = () => {
    setTier(work.tier_pinned ?? scored ?? profile.config.tiers[0]?.key ?? '')
    setReason(work.tier_pin_reason ?? '')
    setOpen(true)
  }

  return (
    <>
      {pinned ? (
        <p className="text-xs text-dim">
          {t('score.tierPinnedBy', {
            tier: labelOf(profile.config.tiers, work.tier_pinned!),
            reason: work.tier_pin_reason ?? '',
          })}{' '}
          <button
            type="button"
            onClick={() => release.mutate()}
            disabled={release.isPending}
            title={t('score.followFactsHint')}
            className="cursor-pointer text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text disabled:opacity-50"
          >
            {t('score.followFacts')}
          </button>
        </p>
      ) : (
        <button
          type="button"
          onClick={start}
          className="flex cursor-pointer items-center gap-1 text-xs text-faint transition-colors hover:text-text"
        >
          <Pin aria-hidden className="size-3" />
          {t('score.pinTier')}
        </button>
      )}

      <Dialog
        open={open}
        onOpenChange={setOpen}
        title={t('score.pinTierTitle')}
        description={t('score.pinTierWhy')}
        footer={
          <Button
            type="submit"
            form="pin-tier"
            variant="primary"
            disabled={tier === '' || reason.trim() === '' || pin.isPending}
          >
            {t('score.pinTierConfirm')}
          </Button>
        }
      >
        <form id="pin-tier" onSubmit={submit} className="flex flex-col gap-3">
          <label className="flex flex-col gap-1">
            <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
              {t('score.pinTierWhich')}
            </span>
            <Select
              value={tier}
              onChange={setTier}
              aria-label={t('score.pinTierWhich')}
              options={profile.config.tiers.map((entry) => ({
                value: entry.key,
                label:
                  entry.key === scored
                    ? `${entry.label} · ${t('score.pinTierScored')}`
                    : entry.label,
              }))}
            />
          </label>

          <label className="flex flex-col gap-1">
            <span className="text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
              {t('score.pinTierReason')}
            </span>
            <Input
              autoFocus
              value={reason}
              onChange={(event) => setReason(event.target.value)}
              placeholder={t('score.pinTierReasonHint')}
            />
          </label>
        </form>
      </Dialog>
    </>
  )
}
