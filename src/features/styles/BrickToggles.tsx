import { useTranslation } from 'react-i18next'
import { Star } from 'lucide-react'
import type { StyleBrick } from '@/lib/api/types'
import { updateStyleBrick } from '@/lib/api/styles'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'

/**
 * The two things done to a style without opening it (v0.132, FR-05 5.2): a
 * star that lifts it in every picker, and a switch that takes it out of them
 * - `ready` and `dropped` - without losing what it says. A draft has nothing
 * to offer yet, so its switch waits for a description. `withSwitch` leaves
 * the switch out where the status is said beside it.
 */
export function BrickToggles({
  brick,
  withSwitch = true,
  className,
}: {
  brick: StyleBrick
  withSwitch?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  const save = useAppMutation({
    mutationFn: (patch: { favorite?: boolean; status?: string }) =>
      updateStyleBrick(brick.id, patch),
    refresh: [keys.styles],
    failure: 'styles.saveFailed',
  })
  const starred = brick.favorite === true

  return (
    <span className={cn('flex items-center gap-1.5', className)}>
      <Button
        variant="icon"
        size="icon-sm"
        aria-pressed={starred}
        aria-label={t('styles.favorite')}
        title={t('styles.favorite')}
        disabled={save.isPending}
        onClick={() => save.mutate({ favorite: !starred })}
      >
        <Star aria-hidden className={starred ? 'fill-current text-warn' : undefined} />
      </Button>
      {withSwitch && (
        <Switch
          aria-label={t('styles.enabled')}
          title={brick.status === 'draft' ? t('styles.enabledDraft') : t('styles.enabled')}
          checked={brick.status === 'ready'}
          disabled={brick.status === 'draft' || save.isPending}
          onCheckedChange={(on) => save.mutate({ status: on ? 'ready' : 'dropped' })}
        />
      )}
    </span>
  )
}
