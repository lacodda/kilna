import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { Score } from '@/lib/api/types'
import { deleteScore } from '@/lib/api/scores'
import { useBlindJudging } from '@/lib/blindJudging'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { labelOf, useVocabulary } from '@/lib/useProfile'
import { SkeletonList } from '@/components/ui/skeleton'
import { ListDetail, Pane } from '@/components/frame'
import { ScoreForm } from '@/features/work/tabs/score/ScoreForm'
import { ScoreList } from '@/features/work/tabs/score/ScoreList'
import { ScoreReading } from '@/features/work/tabs/score/ScoreReading'

interface Props {
  workId: string
}

/**
 * Judging a work along the profile's axes.
 *
 * The scores given so far on the left; on the right, one of two things and
 * never both at once - a recorded score, read, or a new one being given on
 * empty scales. Scoring is a verdict, and the interface should read like one
 * being given; a verdict already given reads like a record.
 */
export function ScorePanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { version_roles: roles } = useVocabulary(workId)

  // Who is judging, kept across scores: judging a batch as one person means
  // typing the name once, not once per score.
  const [rater, setRater] = useState('')
  // Judging blind: the past verdict is held back until this card has one of
  // its own - the list, the trail, the lines beside the axes, the tier a pin
  // would override. What stays visible is what THESE marks add up to.
  const { blind, setBlind, setRevealed, hiding } = useBlindJudging()
  // Which score is open on the right: one picked by hand, a new one being
  // given (`NEW`), or - when nothing is picked - the newest, which is the
  // verdict that stands.
  const [picked, setPicked] = useState<string | null>(null)
  // Bumped by "New score", so pressing it again starts over on empty scales
  // rather than leaving the half-given one in place.
  const [fresh, setFresh] = useState(0)

  const history = useQuery(queries.scoreHistory(workId))
  const versions = useQuery(queries.versions(workId))
  // The pin lives on the work, not on the score: it is one person holding one
  // work at a tier, which is why 0013 put it in a column there.
  const work = useQuery(queries.work(workId))

  const scores = history.data ?? []
  const shown = picked === NEW ? undefined : (scores.find((s) => s.id === picked) ?? scores[0])
  // Blind hides the past verdict, and a reading of it is the past verdict:
  // judging blind always opens the scales.
  const reading = hiding ? undefined : shown

  const remove = useAppMutation({
    mutationFn: deleteScore,
    failure: 'toast.scoreSaveFailed',
    onSuccess: (deletionId, scoreId) => {
      if (picked === scoreId) setPicked(null)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.scoreDeleted'),
        refresh: refresh.score(workId),
      })
    },
  })

  // Oldest first for the trail and the lines; the list stays newest first.
  const oldestFirst = [...scores].reverse()
  // The last five totals: the trail is read at a glance, and the line beside
  // each axis carries the rest.
  const trail = hiding ? [] : oldestFirst.slice(-5)
  // One line per axis, out of the same snapshots. A score records what every
  // axis was worth at the time, so this needs no new storage - only reading
  // the history down a column instead of across it. A snapshot taken before
  // an axis existed simply has no point on that line.
  const trend = (key: string) =>
    hiding
      ? undefined
      : oldestFirst
          .map((score) => score.axes[key])
          .filter((value): value is number => typeof value === 'number')
  // What a pin overrides: the tier the standing score arrives at.
  const scored = hiding ? null : (scores[0]?.tier ?? null)

  // Which draft a score judged, by role and revision - "Lyrics v2" - because
  // "v2" alone does not say whether the number is about the lyrics or the
  // style prompt beside them.
  const draftOf = (score: Score) =>
    score.version_id === null
      ? undefined
      : versions.data?.find((version) => version.id === score.version_id)
  const titleOf = (score: Score) => {
    const draft = draftOf(score)
    const name =
      draft !== undefined
        ? `${labelOf(roles, draft.role)} v${draft.revision}`
        : score.revision !== null
          ? `v${score.revision}`
          : undefined
    return name === undefined ? t('score.ofCurrent') : t('score.ofRevision', { name })
  }
  const judgedOf = (score: Score) => {
    const draft = draftOf(score)
    if (draft === undefined) return undefined
    const name = `${labelOf(roles, draft.role)} v${draft.revision}`
    return draft.label === null ? name : `${name} · ${draft.label}`
  }

  const startNew = () => {
    setPicked(NEW)
    setFresh((count) => count + 1)
  }

  const detail = history.isPending ? (
    // Neither a reading nor a form until the history says which is due: an
    // empty form flashing up before the verdict that stands would read as
    // the verdict having gone.
    <Pane label={t('card.tab.score')} bodyClassName="p-4">
      <SkeletonList rows={4} />
    </Pane>
  ) : reading !== undefined ? (
    <ScoreReading
      key={reading.id}
      workId={workId}
      score={reading}
      standing={reading.id === scores[0]?.id}
      title={titleOf(reading)}
      judged={judgedOf(reading)}
      trail={trail}
      trend={trend}
      work={work.data ?? undefined}
      scored={scored}
    />
  ) : (
    <ScoreForm
      key={fresh}
      workId={workId}
      trail={trail}
      trend={trend}
      work={work.data ?? undefined}
      scored={scored}
      rater={rater}
      onRater={setRater}
      onSaved={() => {
        // The score just given is the newest, which is what opens.
        setPicked(null)
        // The verdict is in, so what was held back is the payoff rather than
        // a temptation: comparing is the whole point of having judged blind.
        setRevealed(true)
      }}
    />
  )

  return (
    // The same shape as the Versions tab, on purpose: the list of scores on
    // the left, the one picked from it open on the right, each scrolling on
    // its own.
    <ListDetail
      list={
        <ScoreList
          workId={workId}
          history={history}
          openId={reading?.id}
          titleOf={titleOf}
          blind={blind}
          hiding={hiding}
          onBlind={(next) => {
            // Going blind is starting a verdict of one's own; seeing again
            // mid-way keeps the marks already set on the scales.
            if (next || reading === undefined) setPicked(NEW)
            setBlind(next)
            setRevealed(false)
          }}
          onReveal={() => {
            // Peeking keeps the scales being marked; the rows come back
            // beside them.
            setPicked(NEW)
            setRevealed(true)
          }}
          onOpen={setPicked}
          onNew={startNew}
          onDelete={(scoreId) => remove.mutate(scoreId)}
        />
      }
      detail={detail}
    />
  )
}

/** The score being given anew rather than one picked from the list. */
const NEW = 'new'
