import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'

/** Who a score kept without a person is by (FR-01): the core's `auto`. */
export const AUTO_RATER = 'auto'

/**
 * The mark of a score the assistant gave by itself (v0.170): muted, because
 * it is an opinion beside the author's and never stands over it - the
 * catalogue reads the author's score wherever there is one.
 */
export function AiOpinion({ className }: { className?: string }) {
  const { t } = useTranslation()
  return (
    <Badge variant="soft" className={className} title={t('score.aiOpinionHint')}>
      {t('score.aiOpinion')}
    </Badge>
  )
}
