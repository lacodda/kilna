import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { Check } from 'lucide-react'
import type { Applied } from '@/lib/api'
import { Button } from '@/components/ui/button'

interface Props {
  /** What applying made, from the message's own meta. */
  applied: Applied
  /** The sentence for this kind of proposal: "Scored.", "Inserted", "Created". */
  label: string
}

/**
 * The mark a proposal carries once it is applied.
 *
 * Read from the message rather than remembered by the component: the chat is
 * refetched every few seconds and the tab is left and returned to, and a mark
 * kept in state was gone on the next fetch — which is how a package of five
 * proposals became a walk through the tabs to see which were already in. A
 * package that created a work links to it: the work is what was made, and
 * the chat is on nothing.
 */
export function AppliedMark({ applied, label }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()

  return (
    <span className="flex items-center gap-1.5 text-xs text-dim">
      <Check aria-hidden className="size-3.5" />
      {label}
      {applied.created_work === true && applied.work_id !== undefined && (
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            void navigate(`/works/${applied.work_id ?? ''}`)
          }}
        >
          {t('assistant.openWork')}
        </Button>
      )}
    </span>
  )
}
