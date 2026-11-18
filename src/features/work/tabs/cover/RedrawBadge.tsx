import { useTranslation } from 'react-i18next'
import { Paintbrush, X } from 'lucide-react'
import type { Work } from '@/lib/api/types'
import { markWorks } from '@/lib/api/works'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'

/** The mark a cover to redraw carries (v0.147); the profile ships it. */
const REDRAW_MARK = 'redraw-cover'

/**
 * A cover to be redrawn says so on its tab (v0.147, FR-10 10.4): the mark
 * the Catalogue filters by, raised from the tab and taken off in one press
 * once the cover is new. Nothing in a profile that has no such mark.
 */
export function RedrawBadge({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const marked = work.marks.includes(REDRAW_MARK)
  const toggle = useAppMutation({
    mutationFn: (raised: boolean) => markWorks([work.id], REDRAW_MARK, raised),
    failure: 'toast.workSaveFailed',
    refresh: refresh.work,
  })
  if (!(config.marks ?? []).some((mark) => mark.key === REDRAW_MARK)) return null

  return marked ? (
    <span className="inline-flex items-center gap-1 rounded-md bg-warn-soft py-0.5 pr-0.5 pl-2 text-xs text-warn">
      <Paintbrush aria-hidden className="size-3" />
      {t('cover.redraw')}
      <Button
        variant="icon"
        size="icon-xs"
        aria-label={t('cover.redrawDone')}
        title={t('cover.redrawDone')}
        disabled={toggle.isPending}
        onClick={() => toggle.mutate(false)}
      >
        <X aria-hidden />
      </Button>
    </span>
  ) : (
    <Button
      variant="icon"
      size="icon-sm"
      aria-label={t('cover.redrawSet')}
      title={t('cover.redrawSet')}
      disabled={toggle.isPending}
      onClick={() => toggle.mutate(true)}
    >
      <Paintbrush aria-hidden />
    </Button>
  )
}
