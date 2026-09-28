import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Rows3 } from 'lucide-react'
import { frameScenes } from '@/lib/api/scenes'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile, vocabularyOf } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'

/**
 * A board before its first scene: what it is waiting for, and the entrance
 * that fills it in one go when there is one.
 */
export function EmptyBoard({ work, hasActions }: { work: Work; hasActions: boolean }) {
  const { t } = useTranslation()
  const profile = useProfile()

  // What the board could be framed from: the first work this one is made
  // from. Nothing to frame without it, and the invitation says only what
  // can be done.
  const links = useQuery(queries.links(work.id))
  const donor = links.data?.sources[0]
  const donorRoles =
    donor === undefined ? [] : vocabularyOf(profile.config, donor.source_kind).version_roles

  const frame = useAppMutation({
    mutationFn: (role: string) => frameScenes(work.id, role),
    failure: 'toast.sceneFrameFailed',
    refresh: refresh.scene,
    onSuccess: (framed) => say.ok(t('scenes.framed', { count: framed.length })),
  })

  return (
    <EmptyState
      plain
      className="p-3"
      title={t(hasActions ? 'scenes.emptyWithActions' : 'scenes.empty')}
      action={
        // The frame from the source text: one scene per part the lyric marks
        // out. Offered only when there is a source to read and roles to read
        // it in — a button that can only refuse is no entrance.
        donorRoles.length > 0 ? (
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm text-dim">{t('scenes.frameFrom')}</span>
            {donorRoles.map((role) => (
              <Button
                key={role.key}
                variant="soft"
                size="sm"
                disabled={frame.isPending}
                onClick={() => frame.mutate(role.key)}
                title={t('scenes.frameHint')}
              >
                <Rows3 aria-hidden />
                {sayLabel(role.label)}
              </Button>
            ))}
          </div>
        ) : undefined
      }
    />
  )
}
