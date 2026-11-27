import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { Check, TriangleAlert } from 'lucide-react'
import type { Applied } from '@/lib/api/types'
import { sayReason } from '@/lib/errors'
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
 *
 * What was kept only in part says so here too (v0.171): a field the register
 * would not take was said in a toast the moment it was kept, and the mark
 * that stays read "kept" - of a thing that was not.
 */
export function AppliedMark({ applied, label }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const unapplied = applied.unapplied ?? []

  return (
    <span className="flex min-w-0 flex-col gap-1 text-xs text-dim">
      <span className="flex items-center gap-1.5">
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
      {unapplied.length > 0 && (
        <span className="flex items-start gap-1.5 text-warn">
          <TriangleAlert aria-hidden className="mt-0.5 size-3.5 shrink-0" />
          <span className="flex min-w-0 flex-col gap-0.5">
            <span>{t('assistant.keptInPart', { count: unapplied.length })}</span>
            <ul className="selectable flex flex-col gap-0.5 text-dim">
              {unapplied.map((reason, index) => (
                // Two fields may be refused for one reason: the place is
                // the identity.
                <li key={index}>{sayReason(reason)}</li>
              ))}
            </ul>
          </span>
        </span>
      )}
    </span>
  )
}
