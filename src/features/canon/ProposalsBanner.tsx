import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { Sparkles } from 'lucide-react'
import { settleSection } from '@/lib/api/canon'
import type { CardView } from '@/lib/api/types'
import { announceEdited } from '@/lib/edited'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { Button } from '@/components/ui/button'

/**
 * What the assistant proposed for an open card and nobody has settled, said
 * in words over the card (v0.141, FR-18 18.2): how many, a press to go to the
 * first, and one to accept them all - each section's drafts settled as that
 * section's own button settles them. Nothing when there is nothing waiting.
 */
export function ProposalsBanner({ view }: { view: CardView }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const drafts = view.facts.filter(({ fact }) => fact.status === 'draft')
  const sections = [...new Set(drafts.map(({ fact }) => fact.section))]
  const accept = useAppMutation({
    mutationFn: async () => {
      let settled = 0
      for (const section of sections) settled += await settleSection(view.card.id, section)
      return settled
    },
    failure: 'canon.settleFailed',
    onSuccess: (count: number) =>
      announceEdited({
        client,
        message: t('canon.sectionSettled', { count }),
        refresh: refresh.canon,
      }),
  })
  if (drafts.length === 0) return null
  return (
    <div
      role="status"
      className="flex flex-wrap items-center gap-2 rounded-md bg-info-soft px-2.5 py-1.5 text-sm text-info"
    >
      <Sparkles aria-hidden className="size-4 shrink-0" />
      <span className="flex-1">{t('canon.proposalsWaiting', { count: drafts.length })}</span>
      <Button
        size="xs"
        variant="ghost"
        onClick={() =>
          document
            .querySelector('[data-draft]')
            ?.scrollIntoView({ block: 'center', behavior: 'smooth' })
        }
      >
        {t('canon.reviewProposals')}
      </Button>
      <Button
        size="xs"
        variant="primary"
        disabled={accept.isPending}
        onClick={() => accept.mutate()}
      >
        {t('canon.acceptProposals')}
      </Button>
    </div>
  )
}
