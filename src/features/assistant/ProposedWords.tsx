import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useTranslation } from 'react-i18next'
import type { Applied, WordsProposal } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { refresh } from '@/lib/query/refresh'
import { useApplyProposal } from '@/lib/useApplyProposal'
import { wordItems } from '@/lib/words'
import { Button } from '@/components/ui/button'
import { ProposalCard } from '@/features/assistant/ProposalCard'
import { WordsPackageView } from '@/features/words/WordsPackageView'

interface Props {
  messageId: string
  proposal: WordsProposal
  /** Set once somebody applied it; read from the message. */
  applied: Applied | null
  /** Set once somebody turned it down; read from the message. */
  dismissed: boolean
}

/**
 * Words for the record an agent proposed (`propose_words`), or an action that
 * produces words answered with - the Studio's Meanings, which names the
 * meanings a song shares with the songs that went out (ADR 0052). Word by
 * word, each with what keeping it writes and a box to leave it out, like a
 * package for the canon: one wrong meaning must not cost the right ones.
 *
 * Kept, the mark says how many words the record took (`Outcome.terms`), and
 * the way on leads where they went: the bank when any went into it, the
 * register otherwise.
 */
export function ProposedWords({ messageId, proposal, applied, dismissed }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const pack = proposal.package
  const all = wordItems(pack)
  const [chosen, setChosen] = useState<string[]>(all)

  const keep = useApplyProposal({
    messageId,
    message: t('words.keptFromChat'),
    refresh: [...refresh.term, keys.pendingProposals],
  })

  const banked = pack.words.some((word) => word.bank === true || word.block !== undefined)
  const first = applied?.terms?.[0]
  const answered = applied !== null || dismissed

  return (
    <ProposalCard
      messageId={messageId}
      title={t('assistant.proposedWords')}
      // How many would be kept, while that is still the question; once kept,
      // the mark says how many the record took.
      figure={answered ? undefined : String(chosen.length)}
      applied={applied}
      dismissed={dismissed}
      applyLabel={
        chosen.length === all.length ? t('words.keepAll') : t('words.keep', { n: chosen.length })
      }
      applying={keep.isPending || chosen.length === 0}
      // Whole when every box is ticked: the backend keeps the package as it
      // read it, rather than a list that happens to name all of it.
      onApply={() => keep.mutate(chosen.length === all.length ? undefined : { items: chosen })}
      appliedLabel={t('words.keptMark', { count: applied?.terms?.length ?? 0 })}
      afterApplied={
        banked ? (
          <Button size="xs" variant="ghost" onClick={() => void navigate('/notes?words')}>
            {t('words.openBank')}
          </Button>
        ) : first !== undefined ? (
          <Button size="xs" variant="ghost" onClick={() => void navigate(`/register/${first}`)}>
            {t('words.open')}
          </Button>
        ) : undefined
      }
    >
      <WordsPackageView pack={pack} chosen={chosen} onChosen={setChosen} answered={answered} />
    </ProposalCard>
  )
}
