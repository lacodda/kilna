import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import type { Applied, StylesProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ProposalCard } from '@/features/assistant/ProposalCard'

interface Props {
  messageId: string
  proposal: StylesProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Styles an agent proposed for the dictionary (v0.134, FR-08): new ones and
 * rewrites, each with what it says and when it can go into a picture. Kept
 * style by style - each one the dictionary's own new style or edit, which
 * undo takes back.
 */
export function ProposedStyles({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const pack = proposal.package
  const all = pack.styles.map((_, index) => `style:${String(index)}`)
  const [chosen, setChosen] = useState<string[]>(all)

  const keep = useApplyProposal({
    messageId,
    message: t('styles.keptFromChat'),
    refresh: [...refresh.style, keys.pendingProposals],
  })
  const first = applied?.style_bricks?.[0]
  const answered = applied !== null || dismissed

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedStyles')}
      figure={answered ? undefined : String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={
        chosen.length === all.length
          ? t('phrases.keepAll')
          : t('phrases.keep', { n: chosen.length })
      }
      applying={keep.isPending || chosen.length === 0}
      onApply={() => keep.mutate(chosen.length === all.length ? undefined : { items: chosen })}
      appliedLabel={t('phrases.keptMark', { count: applied?.style_bricks?.length ?? 0 })}
      afterApplied={
        first === undefined ? undefined : (
          <Button size="xs" variant="ghost" onClick={() => void navigate(`/styles/${first}`)}>
            {t('phrases.openKept')}
          </Button>
        )
      }
    >
      <ul className="flex flex-col gap-1.5 text-xs">
        {pack.styles.map((style, index) => {
          const item = `style:${String(index)}`
          const contract = [
            ...(style.contract?.needs ?? []),
            ...(style.contract?.provides ?? []),
            ...(style.contract?.conflicts ?? []),
          ]
          return (
            <li key={item} className="flex items-start gap-2">
              {!answered && (
                <Checkbox
                  checked={chosen.includes(item)}
                  onCheckedChange={(on) =>
                    setChosen(on ? [...chosen, item] : chosen.filter((one) => one !== item))
                  }
                  aria-label={t('styles.keepItem', { name: style.name ?? style.was ?? '' })}
                  className="mt-0.5"
                />
              )}
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <b className="text-sm font-semibold text-text">
                  {style.was != null && style.name != null && style.name !== style.was
                    ? `${style.was} → ${style.name}`
                    : (style.name ?? style.was)}
                  <span className="ml-2 font-normal text-faint">
                    {style.was != null ? t('styles.rewrite') : (style.type ?? '')}
                  </span>
                </b>
                {style.description != null && (
                  <span className="font-mono text-dim">{style.description}</span>
                )}
                {contract.length > 0 && (
                  <span className="font-mono text-faint">{contract.join(' · ')}</span>
                )}
              </span>
            </li>
          )
        })}
      </ul>
    </ProposalCard>
  )
}
