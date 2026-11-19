import { useTranslation } from 'react-i18next'
import type { PluginCandidate } from '@/lib/api/types'
import { keepPluginCandidate } from '@/lib/api/plugins'
import { formatNumber } from '@/lib/format'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Dialog } from '@/components/AppDialog'
import { Button } from '@/components/ui/button'

interface Props {
  workId: string
  candidates: readonly PluginCandidate[]
  onClose: () => void
}

/**
 * The results a plugin offered to choose from (v0.155, FR-21 21.1): the takes
 * a generator made, each with how long it plays and how far it strays from
 * what was asked. Nothing is kept until one is picked; the pick is kept as a
 * run's answer is - its fields merged, its files the work's.
 */
export function PluginCandidates({ workId, candidates, onClose }: Props) {
  const { t } = useTranslation()
  const keep = useAppMutation({
    mutationFn: (candidate: PluginCandidate) => keepPluginCandidate(workId, candidate),
    failure: 'plugins.candidateFailed',
    refresh: refresh.work,
    onSuccess: () => {
      say.ok(t('plugins.candidateKept'))
      onClose()
    },
  })
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={t('plugins.candidatesTitle', { count: candidates.length })}
      size="lg"
    >
      <ul className="flex flex-col gap-3">
        {candidates.map((candidate, index) => (
          <li
            key={`${candidate.label}:${String(index)}`}
            className="flex items-center gap-3 rounded-md border border-line px-3 py-2"
          >
            <span className="flex min-w-0 flex-1 flex-col">
              <b className="truncate text-sm font-semibold">{candidate.label}</b>
              <span className="text-xs text-dim">
                {[
                  candidate.seconds === null
                    ? null
                    : t('plugins.candidateSeconds', {
                        seconds: formatNumber(candidate.seconds, 0),
                      }),
                  candidate.note,
                  candidate.files.length === 0
                    ? null
                    : t('plugins.candidateFiles', { count: candidate.files.length }),
                ]
                  .filter((part) => part !== null && part !== '')
                  .join(' · ')}
              </span>
            </span>
            <Button
              size="sm"
              variant="soft"
              disabled={keep.isPending}
              onClick={() => keep.mutate(candidate)}
            >
              {t('plugins.candidateKeep')}
            </Button>
          </li>
        ))}
      </ul>
    </Dialog>
  )
}
