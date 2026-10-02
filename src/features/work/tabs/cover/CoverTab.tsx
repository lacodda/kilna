import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { Work } from '@/lib/api/types'
import { queries } from '@/lib/query/queries'
import { SkeletonList } from '@/components/ui/skeleton'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Frame, Scroll } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { ReleaseMetaStatus } from '@/features/work/ReleaseMetaStatus'
import { ConceptColumn } from '@/features/work/tabs/cover/ConceptColumn'
import { CoverProblems, CoverReferences } from '@/features/work/tabs/cover/CoverNotes'
import { CoverPrompts } from '@/features/work/tabs/cover/CoverPrompts'
import { CoverResult } from '@/features/work/tabs/cover/CoverResult'
import { CoverStage } from '@/features/work/tabs/cover/CoverStage'
import { IdeaBoard } from '@/features/work/tabs/cover/IdeaBoard'
import { coverHoldsAnything, useCoverEdit } from '@/features/work/tabs/cover/useCoverEdit'

interface Props {
  work: Work
}

type View = 'ideas' | 'constructor'

/**
 * The Cover tab of a publication: the board of ideas (v0.89) and the
 * constructor (v0.88, ADR 0049), one switch between them as in the mockup.
 *
 * A cover with nothing in it opens on the ideas - "Make…" lands here with
 * ideas on the way - and one that holds anything opens on the constructor,
 * where its prompt is. An idea taken in from the board shows in the
 * constructor at once.
 *
 * The constructor is laid out as the mockup's: a fixed column of choices,
 * the rest for what they make. Every choice is saved as it is made and the
 * prompt is written on the Rust side from what is saved, so the stage and the
 * prompt always say the same thing - and an agent reading the work over MCP
 * gets the same words.
 *
 * Where the release meta is being written stands on top of both: "Make…"
 * lands a new publication here, and the job it started is the first thing to
 * see.
 */
export function CoverTab({ work }: Props) {
  const { t } = useTranslation()
  const [format, setFormat] = useState<string | null>(null)
  const [view, show] = useState<View>(() =>
    coverHoldsAnything(work.cover) ? 'constructor' : 'ideas',
  )
  const edit = useCoverEdit(work)
  const cover = useQuery(queries.coverView(work.id, format))
  const board = useQuery(queries.coverBoard(work.id))

  return (
    <Frame>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
        <ReleaseMetaStatus work={work} />
        <div className="flex shrink-0 flex-wrap items-center gap-2.5">
          <SegmentedControl
            aria-label={t('ideas.views')}
            value={view}
            onValueChange={(next) => show(next as View)}
          >
            <Segment value="ideas">
              {t('ideas.title')}
              <span className="font-mono text-2xs text-faint">
                {(board.data?.ideas.length ?? 0) + (board.data?.siblings.length ?? 0)}
              </span>
            </Segment>
            <Segment value="constructor">{t('ideas.constructor')}</Segment>
          </SegmentedControl>
          <span className="text-2xs text-faint">
            {view === 'ideas' ? t('ideas.noMaster') : t('ideas.savedAsMade')}
          </span>
        </div>

        {view === 'ideas' ? (
          <Loaded query={board} skeleton={<SkeletonList rows={3} secondary={false} />} plain fill>
            {(data) => <IdeaBoard work={work} board={data} onTaken={() => show('constructor')} />}
          </Loaded>
        ) : (
          <div className="flex min-h-0 min-w-0 flex-1 gap-2.5">
            <div className="flex min-h-0 w-98 shrink-0 flex-col">
              <ConceptColumn
                cover={work.cover}
                view={cover.data}
                board={board.data}
                change={edit.change}
                onAllIdeas={() => show('ideas')}
              />
            </div>
            {/* Bound: the prompt runs to thousands of characters, and its
                panel scrolls inside rather than taking the preview above it
                off the screen (`CoverPrompts`). */}
            <Scroll
              label={t('cover.made')}
              bound
              className="min-w-0 flex-1"
              contentClassName="flex flex-col gap-2.5"
            >
              <Loaded query={cover} skeleton={<SkeletonList rows={4} secondary={false} />} plain>
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
        )}
      </div>
    </Frame>
  )
}
