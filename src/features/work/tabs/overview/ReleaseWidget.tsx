import { useTranslation } from 'react-i18next'
import type { Work } from '@/lib/api/types'
import { Widget } from '@/features/work/tabs/overview/Widget'
import { ReleaseBlock } from '@/features/work/tabs/releases/ReleaseBlock'

/**
 * A publication's release on its board (v0.90): the release whole - where,
 * when, whether it went, the link, what it goes out under - in the lead,
 * where the board is read first.
 *
 * Unlike the other widgets it leads nowhere: the release has no tab of its
 * own any more. A publication goes out once (ADR 0051), so the block is the
 * release's one home, and editing it here gives it no second one.
 */
export function ReleaseWidget({ work }: { work: Work }) {
  const { t } = useTranslation()
  return (
    <Widget caption={t('releases.caption')}>
      <ReleaseBlock work={work} />
    </Widget>
  )
}
