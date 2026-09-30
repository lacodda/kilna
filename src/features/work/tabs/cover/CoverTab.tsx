import { useTranslation } from 'react-i18next'
import type { Work } from '@/lib/api/types'
import { Frame, Scroll } from '@/components/frame'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import { CoverFormats } from '@/features/work/tabs/cover/CoverFormats'
import { CoverPrompt } from '@/features/work/tabs/cover/CoverPrompt'

interface Props {
  work: Work
}

/**
 * What a publication looks like where it goes out (v0.86): the shapes its
 * doors ask for, and the prompt its cover is drawn from.
 *
 * Deliberately modest. The mockup's board of ideas and its constructor are
 * v0.88-v0.89; this tab gives the prompt the room the Files tab could not,
 * and says which shapes the work owes. The pictures themselves stay on the
 * Files tab, with every other file.
 *
 * Where the release meta is being written stands on top: "Make..." lands a
 * new publication here, and the job it started is the first thing to see.
 */
export function CoverTab({ work }: Props) {
  const { t } = useTranslation()

  return (
    <Frame>
      {/* One column with its own gap rather than the frame's head: the status
          above draws nothing when nothing is being written, and a head that
          holds nothing still takes its gap. */}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
        <ReleaseMetaStatus work={work} />
        <CoverFormats work={work} />
        <Scroll label={t('card.tab.cover')} contentClassName="flex flex-col gap-2.5">
          <CoverPrompt work={work} />
        </Scroll>
      </div>
    </Frame>
  )
}
