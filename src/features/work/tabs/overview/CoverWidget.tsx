import { useTranslation } from 'react-i18next'
import type { Work } from '@/lib/api/types'
import { coverImageFor } from '@/lib/cover'
import { useCovers } from '@/lib/useCovers'
import { cn } from '@/lib/utils'
import { Invite, InviteLink, useLook, Widget } from '@/features/work/tabs/overview/Widget'

/**
 * The cover: the widget catalogue's picture. Until there is one, the colour
 * the work has had from its first day (`lib/cover`) stands in its place, and
 * the widget says so and offers the Files tab, where a cover is set.
 */
export function CoverWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  const { presentation, size } = useLook()
  const cover = useCovers().get(work.id)
  const to = `/works/${work.id}/files`
  const missing = cover === undefined

  const picture = (
    <div
      role="img"
      aria-label={t('files.cover')}
      className={cn(
        'shrink-0 rounded-md',
        presentation === 'card' ? (size === 'l' ? 'h-48 w-full' : 'h-24 w-full') : 'h-10 w-16',
      )}
      style={{ background: coverImageFor(work.id, cover) }}
    />
  )

  return (
    <Widget
      caption={t('files.cover')}
      to={to}
      tone={missing ? 'empty' : 'plain'}
      actions={missing ? <InviteLink to={to}>{t('files.setCover')}</InviteLink> : undefined}
    >
      <div
        className={cn('flex min-w-0 gap-2', presentation === 'card' ? 'flex-col' : 'items-center')}
      >
        {picture}
        {missing && <Invite>{t('overview.coverMissing')}</Invite>}
      </div>
    </Widget>
  )
}
