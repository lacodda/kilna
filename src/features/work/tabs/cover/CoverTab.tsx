import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { SkeletonList } from '@/components/ui/skeleton'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import { ConceptColumn } from '@/features/work/tabs/cover/ConceptColumn'
import { CoverProblems, CoverReferences } from '@/features/work/tabs/cover/CoverNotes'
import { CoverPrompts } from '@/features/work/tabs/cover/CoverPrompts'
import { CoverResult } from '@/features/work/tabs/cover/CoverResult'
import { CoverStage } from '@/features/work/tabs/cover/CoverStage'
import { useCoverEdit } from '@/features/work/tabs/cover/useCoverEdit'

interface Props {
  work: Work
}

/**
 * The cover constructor (v0.88, ADR 0049): what a publication's cover is
 * built from on the left, what that makes on the right - the stage with the
 * built frame drawn in the shape of a door, the prompt to copy into a
 * generator, the files to hand it, and what came back.
 *
 * Laid out as the mockup's constructor: a fixed column of choices, the rest
 * for what they make. Every choice is saved as it is made and the prompt is
 * written on the Rust side from what is saved, so the stage and the prompt
 * always say the same thing - and an agent reading the work over MCP gets
 * the same words.
 *
 * Where the release meta is being written stands on top: "Make..." lands a
 * new publication here, and the job it started is the first thing to see.
 */
export function CoverTab({ work }: Props) {
  const { t } = useTranslation()
  const [format, setFormat] = useState<string | null>(null)
  const edit = useCoverEdit(work)
  const view = useQuery(queries.coverView(work.id, format))

  return (
    <Frame>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
        <ReleaseMetaStatus work={work} />
        <div className="flex min-h-0 min-w-0 flex-1 gap-2.5">
          <div className="flex min-h-0 w-98 shrink-0 flex-col">
            <ConceptColumn cover={work.cover} view={view.data} change={edit.change} />
          </div>
          <Scroll
            label={t('cover.made')}
            className="min-w-0 flex-1"
            contentClassName="flex flex-col gap-2.5"
          >
            <Loaded query={view} skeleton={<SkeletonList rows={4} secondary={false} />} plain>
              {(data) => (
                <>
                  <CoverStage view={data} mark={work.cover.mark} onFormat={setFormat} />
                  <CoverProblems problems={data.problems} />
                  <CoverPrompts cover={work.cover} view={data} change={edit.change} />
                  <CoverReferences references={data.references} />
                  <CoverResult work={work} view={data} />
                </>
              )}
            </Loaded>
          </Scroll>
        </div>
      </div>
    </Frame>
  )
}
