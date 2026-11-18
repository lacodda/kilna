import { useTranslation } from 'react-i18next'
import type { StyleBrick, StyleType } from '@/lib/api/types'
import { updateProfileConfig } from '@/lib/api/workspace'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { withFamilyOff } from '@/lib/styleBrick'
import { styleIconOf } from '@/lib/styleIcon'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'

/** The family filter that means "filed under none": the owner's own. */
export const NO_SET = ''

interface Props {
  /** The types of the half on screen, or the one type chosen. */
  types: readonly StyleType[]
  /** Every brick of those types, whatever the other filters say. */
  bricks: readonly StyleBrick[]
  typeKey: string | undefined
  family: string | undefined
  onPick: (typeKey: string, family: string | undefined) => void
}

/**
 * The dictionary's sets, type by type (v0.132, FR-05 5.3): each family with
 * how many styles it holds and a switch that takes the whole set out of the
 * pickers - and puts it back - without touching any style's own status. The
 * switch is the profile's, written as one `profile.update` undo takes back.
 * "No set" is listed only where the owner has styles of their own filed
 * under none: every shipped style is filed.
 */
export function SetsPanel({ types, bricks, typeKey, family, onPick }: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const toggle = useAppMutation({
    mutationFn: ({ type, key, off }: { type: string; key: string; off: boolean }) =>
      updateProfileConfig(profile.id, withFamilyOff(profile.config, type, key, off)),
    refresh: refresh.profile,
    failure: 'toast.profileSaveFailed',
  })

  const groups = types
    .map((type) => {
      const own = bricks.filter((brick) => brick.type_key === type.key)
      const unfiled = own.filter((brick) => brick.family === null).length
      return { type, own, unfiled }
    })
    .filter(({ type, unfiled }) => (type.families ?? []).length > 0 || unfiled > 0)

  if (groups.length === 0) return null
  return (
    <nav
      aria-label={t('styles.sets')}
      className="flex w-56 shrink-0 flex-col gap-3 overflow-y-auto pr-1"
    >
      {groups.map(({ type, own, unfiled }) => {
        const Icon = styleIconOf(type)
        return (
          <section
            key={type.key}
            aria-label={sayLabel(type.label)}
            className="flex flex-col gap-0.5"
          >
            <h2 className="caption flex items-center gap-1.5 px-1.5">
              <Icon aria-hidden className="size-3" />
              {sayLabel(type.label)}
            </h2>
            {(type.families ?? []).map((one) => {
              const count = own.filter((brick) => brick.family === one.key).length
              const on = one.off !== true
              const chosen = typeKey === type.key && family === one.key
              return (
                <div key={one.key} className="flex items-center gap-1.5">
                  <Button
                    size="sm"
                    variant={chosen ? 'soft' : 'icon'}
                    aria-pressed={chosen}
                    onClick={() => onPick(type.key, chosen ? undefined : one.key)}
                    className={cn(
                      'min-w-0 flex-1 justify-start gap-2 text-sm',
                      !on && 'text-faint',
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate text-start">
                      {sayLabel(one.label)}
                    </span>
                    <span className="font-mono text-xs text-faint">{count}</span>
                  </Button>
                  <Switch
                    aria-label={t('styles.setOn', { set: sayLabel(one.label) })}
                    checked={on}
                    disabled={toggle.isPending}
                    onCheckedChange={(next) =>
                      toggle.mutate({ type: type.key, key: one.key, off: !next })
                    }
                  />
                </div>
              )
            })}
            {unfiled > 0 && (
              <Button
                size="sm"
                variant={typeKey === type.key && family === NO_SET ? 'soft' : 'icon'}
                aria-pressed={typeKey === type.key && family === NO_SET}
                onClick={() =>
                  onPick(type.key, typeKey === type.key && family === NO_SET ? undefined : NO_SET)
                }
                className="mr-10.5 justify-start gap-2 text-sm"
              >
                <span className="min-w-0 flex-1 truncate text-start italic">
                  {t('styles.noSet')}
                </span>
                <span className="font-mono text-xs text-faint">{unfiled}</span>
              </Button>
            )}
          </section>
        )
      })}
    </nav>
  )
}
