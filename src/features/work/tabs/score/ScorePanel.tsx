import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Eye, EyeOff, Plus, X } from 'lucide-react'
import { deleteScore, scoreWork } from '@/lib/api/scores'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { markReaching, rubricFor, tierFor, toNextTier, total as computeTotal } from '@/lib/scoring'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Chip } from '@/components/ui/chip'
import { EmptyState } from '@/components/ui/empty-state'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { RowButton } from '@/components/ui/list-row'
import { SkeletonList } from '@/components/ui/skeleton'
import { Sparkline } from '@/components/ui/sparkline'
import { AxisBar, TierRuler } from '@/components/ui/tier'
import { Select } from '@/components/AppSelect'
import { ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { TierPin } from '@/features/work/tabs/score/TierPin'
import { KindVerdicts } from '@/features/work/tabs/score/KindVerdicts'
import { useBlindJudging } from '@/lib/blindJudging'
import { cn } from '@/lib/utils'
import { formatDay, formatDelta, formatNumber } from '@/lib/format'

interface Props {
  workId: string
}

/**
 * Judging a work along the profile's axes.
 *
 * One row per axis — what it is called, what it weighs, what it asks — with a
 * scale you click rather than a box you type into. Scoring is a verdict, and
 * the interface should read like one being given.
 */
export function ScorePanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const { axes, tiers, version_roles } = useVocabulary(workId)

  // What the person has set on the scales this time — `null` until a mark is
  // moved. Until then the scales mirror the recorded score, so opening the
  // tab shows the verdict that stands rather than an empty form with the
  // verdict listed underneath; after a save they mirror the one just given.
  const [form, setForm] = useState<Record<string, number> | null>(null)
  // The mark the pointer is over, per axis, so the rubric can answer "what is
  // a seven here" while the person is deciding rather than after.
  const [hovered, setHovered] = useState<Record<string, number>>({})
  const [note, setNote] = useState('')
  // Who is judging. Empty means the author, which is what the column has meant
  // since v0.50 - so a second opinion is a name typed here, not a second
  // workspace.
  const [rater, setRater] = useState('')
  // Judging blind: the past verdict and the assistant's are held back until
  // this card has one of its own. What stays visible is what THESE marks add
  // up to - a mirror of the verdict being given, not a hint about the last
  // one. Per-session rather than stored: it is a way of working through one
  // batch, not a setting about the workspace.
  const { blind, setBlind, setRevealed, hiding } = useBlindJudging()
  // Empty means "whatever the work currently points at", which is what the
  // backend already does when no version is named.
  const [versionId, setVersionId] = useState('')
  // Which score in the list is open on the right: one picked by hand, a new
  // one being given (`new`, empty scales), or - when nothing is picked - the
  // newest, which is the verdict that stands.
  const [picked, setPicked] = useState<string | null>(null)

  const history = useQuery(queries.scoreHistory(workId))

  const versions = useQuery(queries.versions(workId))

  // The pin lives on the work, not on the score: it is one person holding one
  // work at a tier, which is why 0013 put it in a column there.
  const work = useQuery(queries.work(workId))

  const historyData = history.data ?? []
  const shown =
    picked === NEW
      ? undefined
      : (historyData.find((score) => score.id === picked) ?? historyData[0])

  // The roles that comment rather than stand as the work: a review, a
  // critique. Read from the profile, the same rule the versions tab and the
  // catalogue's count draw by.
  const commenting = new Set(
    version_roles.filter((role) => role.comments_on !== undefined).map((role) => role.key),
  )
  // Newest first, as the history above is.
  const verdicts = (versions.data ?? [])
    .filter((version) => commenting.has(version.role))
    .slice()
    .sort((left, right) => right.created_at.localeCompare(left.created_at))
  // The marks of the open score, restricted to axes the profile still has:
  // a snapshot taken before an axis was removed keeps that mark, but the
  // scale for it is gone.
  const recorded: Record<string, number> = {}
  for (const axis of axes) {
    const mark = shown?.axes[axis.key]
    if (typeof mark === 'number') recorded[axis.key] = mark
  }
  // Judging blind hides the past verdict, and the scales are the past verdict
  // too — so blind starts from empty scales, not from a mirror of last time.
  const values = form ?? (hiding || shown === undefined ? {} : recorded)
  const touched = form !== null
  const setValues = (update: (current: Record<string, number>) => Record<string, number>) =>
    setForm(update(values))

  const filled = Object.keys(values).length
  const preview = computeTotal(axes, values)
  const previewTier = tierFor(tiers, preview)
  // What the next tier costs, and where it is cheapest - the advice this
  // panel exists to give once the number stops being interesting on its own.
  const ahead = filled === 0 ? undefined : toNextTier(axes, values, tiers, preview)

  const save = useAppMutation({
    mutationFn: () =>
      scoreWork(workId, {
        axes: values,
        version_id: versionId === '' ? null : versionId,
        note: note.trim() === '' ? null : note.trim(),
        rater: rater.trim() === '' ? null : rater.trim(),
      }),
    failure: 'toast.scoreSaveFailed',
    refresh: refresh.score(workId),
    onSuccess: () => {
      // Back to mirroring: the score just saved is now the recorded one.
      setForm(null)
      setNote('')
      setVersionId('')
      // The score just given is the newest, which is what opens by default.
      setPicked(null)
      // The rater is deliberately kept: judging a batch as one person means
      // typing the name once, not once per work.
      // The verdict is in, so what was held back is the payoff rather than a
      // temptation: comparing is the whole point of having judged blind.
      setRevealed(true)
      say.ok(t('toast.scoreSaved'))
    },
  })

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

  // Oldest first for the lines and the trail; the list stays newest first.
  const oldestFirst = [...historyData].reverse()

  // One line per axis, out of the same snapshots. A score records what every
  // axis was worth at the time, so this needs no new storage - only reading
  // the history down a column instead of across it. A snapshot taken before
  // an axis existed simply has no point on that line.
  const axisTrend = (key: string) =>
    oldestFirst
      .map((score) => score.axes[key])
      .filter((value): value is number => typeof value === 'number')

  // The rubric entry for the mark being weighed on an axis, if the craft wrote one.
  const rubricLine = (axis: (typeof axes)[number]) => {
    const mark = hovered[axis.key] ?? values[axis.key]
    return mark === undefined ? undefined : rubricFor(axis, mark)
  }

  const versionOptions = (versions.data ?? []).map((version) => ({
    value: version.id,
    label: `${labelOf(version_roles, version.role)} · ${
      version.label ?? t('versions.revision', { number: version.revision })
    }${version.is_current ? ` · ${t('versions.current')}` : ''}`,
  }))

  // The head of the open score: which draft it judged and when, or that this
  // is a new one being given.
  const heading =
    shown === undefined
      ? t('score.newTitle')
      : `${
          shown.revision !== null
            ? t('score.ofRevision', { name: `v${shown.revision}` })
            : t('score.ofCurrent')
        } · ${formatDay(shown.scored_at)}`

  // The last few totals, oldest first: `78 → 86 → 91`. Five, because the
  // trail is read at a glance, and the line under each axis carries the rest.
  const trail = oldestFirst.slice(-5)

  // The verdict, fixed at the foot of the open score: the total, its tier,
  // where it has come from, and the button that records it. The axes scroll
  // above; this never does.
  const verdict = (
    <div className="flex w-full flex-wrap items-center gap-3">
      <span className="font-mono text-2xl font-semibold tabular-nums">
        {filled === 0 ? '—' : formatNumber(preview)}
      </span>

      {previewTier !== undefined && filled > 0 && (
        <Badge variant="accent">{sayLabel(previewTier.label)}</Badge>
      )}

      {trail.length > 1 && !hiding && (
        <span
          className="font-mono text-xs text-faint tabular-nums"
          title={t('score.trend', {
            from: formatNumber(trail[0]!.total, 0),
            to: formatNumber(trail.at(-1)!.total, 0),
          })}
        >
          {trail.map((score, index) => {
            const prior = trail[index - 1]?.total
            return (
              <span
                key={score.id}
                className={cn(
                  prior !== undefined && score.total > prior && 'text-good',
                  prior !== undefined && score.total < prior && 'text-bad',
                )}
              >
                {index > 0 && ' → '}
                {formatNumber(score.total)}
              </span>
            )
          })}
        </span>
      )}

      <Button
        className="ml-auto"
        variant="primary"
        disabled={filled === 0 || !touched || save.isPending}
        onClick={() => save.mutate()}
      >
        {t('score.save')}
      </Button>
    </div>
  )

  const list = (
    <Pane
      label={t('score.history', { count: historyData.length })}
      bodyClassName="flex flex-col gap-3 p-1.5"
      head={
        <>
          <span className="caption">{t('score.history', { count: historyData.length })}</span>
          {historyData.length > 0 && (
            <Chip
              pressed={blind}
              onPressedChange={(next) => {
                setBlind(next)
                setRevealed(false)
              }}
              title={t('score.blindOnHint')}
              className="ml-auto"
            >
              {blind ? (
                <EyeOff aria-hidden className="size-3.5" />
              ) : (
                <Eye aria-hidden className="size-3.5" />
              )}
              {t('score.blindOn')}
            </Chip>
          )}
        </>
      }
      foot={
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-center"
          onClick={() => {
            setPicked(NEW)
            setForm(null)
          }}
        >
          <Plus aria-hidden />
          {t('score.new')}
        </Button>
      }
    >
      <Loaded
        query={history}
        skeleton={<SkeletonList rows={3} />}
        isEmpty={(data) => data.length === 0}
        // Plain, and its way out is the button at the foot.
        emptyState={<EmptyState plain title={t('score.none')} className="p-2" />}
        plain
      >
        {() =>
          hiding ? (
            <p className="rounded-md border border-dashed border-line p-2 text-xs text-dim">
              {t('score.blindHidden')}{' '}
              <Button variant="link" onClick={() => setRevealed(true)}>
                {t('score.blindReveal')}
              </Button>
            </p>
          ) : (
            <ul className="flex flex-col gap-0.5">
              {historyData.map((score, index) => {
                // What this score did to the one before it: the list is
                // newest first, so the one before is the next row down.
                const before = historyData[index + 1]?.total
                const delta = before === undefined ? undefined : score.total - before
                const open = score.id === shown?.id

                return (
                  <li key={score.id} className="group relative">
                    <RowButton
                      selected={open}
                      onClick={() => {
                        setPicked(score.id)
                        // Opening another score is reading it: marks moved on
                        // the last one do not follow to this one.
                        setForm(null)
                      }}
                      title={score.note ?? undefined}
                      // Room at the right edge for the delete laid over it.
                      className="pr-8"
                      description={
                        <span className="font-mono">
                          {formatDay(score.scored_at)}
                          {score.tier !== null && ` · ${labelOf(tiers, score.tier)}`}
                          {/* Who judged, when it was not you: null has meant
                              the author since v0.50, so your own rows stay
                              unlabelled and a second opinion shows by
                              contrast. */}
                          {score.rater !== null && score.rater !== '' && ` · ${score.rater}`}
                        </span>
                      }
                      end={
                        <>
                          <span
                            className={cn(
                              'font-mono',
                              delta === undefined || Math.abs(delta) < 0.05
                                ? 'text-faint'
                                : delta > 0
                                  ? 'text-good'
                                  : 'text-bad',
                            )}
                          >
                            {delta === undefined || Math.abs(delta) < 0.05
                              ? '—'
                              : formatDelta(delta)}
                          </span>
                          <span className="w-9 text-right font-mono text-base font-semibold text-text">
                            {formatNumber(score.total)}
                          </span>
                        </>
                      }
                    >
                      {score.revision !== null
                        ? t('score.ofRevision', { name: `v${score.revision}` })
                        : t('score.ofCurrent')}
                    </RowButton>
                    {/* Over the row's right edge, on hover: deleting is rare,
                        and a cross on every row reads as the row's purpose. */}
                    <Button
                      variant="danger"
                      size="icon-sm"
                      title={t('score.delete')}
                      aria-label={t('score.delete')}
                      onClick={() => remove.mutate(score.id)}
                      className="absolute top-1/2 right-1 -translate-y-1/2 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                    >
                      <X aria-hidden className="size-3.5" />
                    </Button>
                  </li>
                )
              })}
            </ul>
          )
        }
      </Loaded>

      {/* What was written about this work, beside what it was marked. A
          critique is a version in a commenting role; these are the same
          rows the Versions tab holds, reached from a second place rather
          than copied. A score is a number with a reason, and the reason
          should be one click from the number. */}
      {verdicts.length > 0 && (
        <section className="flex flex-col gap-0.5 border-t border-line pt-2">
          <h4 className="px-2 pb-1 caption">{t('score.written')}</h4>
          {verdicts.map((verdict) => (
            <Link
              key={verdict.id}
              to={`/works/${workId}/versions?version=${verdict.id}`}
              className="flex items-baseline gap-2 rounded-md px-2 py-1.5 no-underline transition-colors hover:bg-soft"
            >
              <span className="shrink-0 text-xs text-dim">
                {labelOf(version_roles, verdict.role)}
              </span>
              <span className="min-w-0 flex-1 truncate text-sm text-text">
                {verdict.label ?? t('versions.revision', { number: verdict.revision })}
              </span>
              <span className="shrink-0 font-mono text-xs text-faint">
                {verdict.created_at.slice(5, 10)}
              </span>
            </Link>
          ))}
        </section>
      )}
    </Pane>
  )

  const detail = (
    <Pane
      label={heading}
      bodyClassName="flex flex-col gap-4 p-4"
      head={
        <>
          <span className="truncate font-mono text-xs text-faint">{heading}</span>
          {shown?.version_id != null && (
            // The judgement points at what was judged: the review of that
            // draft is the rest of the sentence this number starts.
            <Link
              to={`/works/${workId}/versions?version=${shown.version_id}`}
              title={t('score.openVersion')}
              className="ml-auto shrink-0 text-xs text-dim underline decoration-dotted underline-offset-2 transition-colors hover:text-text"
            >
              {t('score.openVersion')}
            </Link>
          )}
        </>
      }
      foot={verdict}
    >
      <div className="flex flex-col gap-2.5">
        {/* A kind with no axes yet — a video in a workspace whose owner has
            not written its judgement — is scored empty rather than on the
            song's axes. Said here, with the way to the editor, because an
            empty panel reads as broken and it is not. */}
        {axes.length === 0 && (
          <p className="text-sm text-dim">
            {t('score.noAxes')}{' '}
            <Link
              to="/settings"
              className="underline decoration-dotted underline-offset-2 hover:text-text"
            >
              {t('score.noAxesLink')}
            </Link>
          </p>
        )}
        {axes.map((axis) => (
          <div
            key={axis.key}
            className="grid items-center gap-3 sm:grid-cols-[minmax(10rem,16rem)_minmax(0,1fr)_2.5rem]"
          >
            <span className="min-w-0">
              <b className="block truncate text-sm font-semibold">
                {sayLabel(axis.label)}{' '}
                <span className="font-mono text-2xs font-normal text-faint">×{axis.weight}</span>
              </b>
              {/* The question the axis asks, in the open. It used to be a
                  tooltip, which is the same as not being there — but wrapping
                  it made a six-axis card taller than the screen, and scoring
                  is a judgement you make by looking at all the axes at once.
                  One line, with the whole of it on hover. */}
              {sayLabel(axis.description) !== '' && (
                <span
                  className="block truncate text-xs text-faint"
                  title={sayLabel(axis.description)}
                >
                  {sayLabel(axis.description)}
                </span>
              )}
            </span>

            <span className="flex flex-col gap-1">
              <AxisBar
                scale={axis.scale}
                value={values[axis.key]}
                label={sayLabel(axis.label)}
                valueText={
                  values[axis.key] === undefined ? t('score.unjudged') : String(values[axis.key])
                }
                threshold={
                  // Only drawn where it is true: the mark on THIS axis from
                  // which the total would cross into the tier ahead. No such
                  // mark, no line.
                  ahead === undefined
                    ? undefined
                    : (() => {
                        const mark = markReaching(axes, values, axis, ahead.tier.min)
                        return mark === undefined
                          ? undefined
                          : {
                              mark,
                              label: t('score.crossesHere', {
                                tier: sayLabel(ahead.tier.label),
                              }),
                            }
                      })()
                }
                onPreview={(mark) =>
                  setHovered((current) => {
                    const updated = { ...current }
                    if (mark === undefined) delete updated[axis.key]
                    else updated[axis.key] = mark
                    return updated
                  })
                }
                onChange={(next) =>
                  setValues((current) => {
                    const updated = { ...current }
                    if (next === undefined) delete updated[axis.key]
                    else updated[axis.key] = next
                    return updated
                  })
                }
              />

              {/* What the mark under consideration means, when the craft has
                  said. The mark being hovered wins over the one already set:
                  the question while scoring is about the mark being weighed,
                  not the one already given.

                  The line keeps its row whether or not it has anything to
                  say. Appearing on hover pushed the axes below out from under
                  the pointer, the hover ended, the line went, the axes came
                  back under the pointer — a strobe. */}
              <span
                className="block h-4 truncate text-xs leading-4 text-dim"
                title={sayLabel(rubricLine(axis)?.label)}
              >
                {(() => {
                  const entry = rubricLine(axis)
                  if (entry === undefined) return '\u00a0'
                  return (
                    <>
                      <b className="font-mono font-semibold">{entry.at}</b> —{' '}
                      {sayLabel(entry.label)}
                    </>
                  )
                })()}
              </span>
            </span>

            <span className="flex items-center justify-end gap-2">
              {/* This axis over time, beside the axis it belongs to. The one
                  line under the total says the card moved; these say which
                  axis moved it. */}
              {(() => {
                if (hiding) return null
                const line = axisTrend(axis.key)
                if (line.length < 2) return null

                return (
                  <Sparkline
                    values={line}
                    max={axis.scale}
                    size="sm"
                    label={t('score.axisTrend', {
                      axis: sayLabel(axis.label),
                      from: formatNumber(line[0]!, 0),
                      to: formatNumber(line.at(-1)!, 0),
                    })}
                  />
                )
              })()}

              <span className="w-6 text-right font-mono text-sm text-dim tabular-nums">
                {values[axis.key] ?? '—'}
              </span>
            </span>
          </div>
        ))}
      </div>

      <div className="flex flex-col gap-3 border-t border-line pt-4">
        {!touched && filled > 0 && <p className="text-xs text-faint">{t('score.mirroring')}</p>}

        {filled > 0 && (
          <div className="flex flex-col gap-1.5">
            <TierRuler
              tiers={tiers.map((tier) => ({
                key: tier.key,
                label: sayLabel(tier.label),
                min: tier.min,
              }))}
              value={preview}
              label={t('score.rulerLabel', { score: formatNumber(preview) })}
              valueText={
                previewTier === undefined
                  ? formatNumber(preview)
                  : `${formatNumber(preview)}, ${sayLabel(previewTier.label)}`
              }
            />

            {/* The sentence the panel is for: not "you are Silver" but "you
                are four points short, and the cheapest four are here". */}
            <p className="text-xs text-dim">
              {ahead === undefined ? (
                t('score.topTier')
              ) : (
                <>
                  <b className="font-semibold text-text">
                    {t('score.toNextTier', {
                      gap: formatNumber(ahead.gap),
                      tier: sayLabel(ahead.tier.label),
                    })}
                  </b>
                  {ahead.cheapest !== undefined && (
                    <>
                      {' · '}
                      {t('score.cheapest', {
                        axis: sayLabel(ahead.cheapest.axis.label),
                        weight: ahead.cheapest.axis.weight,
                        count: ahead.cheapest.marks,
                      })}
                    </>
                  )}
                </>
              )}
            </p>
          </div>
        )}

        {filled > 0 && filled < axes.length && (
          <p className="text-xs text-dim">{t('score.partial', { filled, count: axes.length })}</p>
        )}

        {/* Held by hand, or free to follow the score. Placed under the
            verdict it overrides, and shown even with nothing filled in:
            a pin is about the work, not about the form being typed. */}
        {work.data != null && (
          <TierPin work={work.data} scored={hiding ? null : (historyData[0]?.tier ?? null)} />
        )}

        {/* What the recorded score means to each channel. Reads the latest
            score rather than the form above: a verdict per kind is about
            what stands, not about what is being typed. */}
        {!hiding && <KindVerdicts workId={workId} />}

        {/* Both of these have been in the API since v0.3.0 and never sent.
            A score belongs to the draft it judged — usually the current one,
            which is what an empty choice means. */}
        <div className="flex flex-wrap items-end gap-3">
          {/* A group rather than a Field: the select is a button and a
              popup, not an input a Field could hand its id to. */}
          <FieldGroup label={t('score.ofVersion')}>
            <Select
              className="w-64"
              aria-label={t('score.ofVersion')}
              value={versionId}
              onChange={setVersionId}
              placeholder={t('score.currentVersion')}
              options={versionOptions}
            />
          </FieldGroup>

          <Field label={t('score.rater')}>
            <Input
              className="w-44"
              value={rater}
              onChange={(event) => setRater(event.target.value)}
              placeholder={t('score.raterHint')}
            />
          </Field>

          <Field label={t('score.note')} className="min-w-56 flex-1">
            <Input
              value={note}
              onChange={(event) => setNote(event.target.value)}
              placeholder={t('score.notePlaceholder')}
            />
          </Field>
        </div>
      </div>
    </Pane>
  )

  return (
    // The same shape as the Versions tab, on purpose: the list of scores on
    // the left, the one picked from it open on the right, each scrolling on
    // its own. The history of totals is the most valuable thing on this tab,
    // and a picker that folded it into a menu hid it.
    <ListDetail list={list} detail={detail} />
  )
}

/** The score being given anew rather than one picked from the list. */
const NEW = 'new'
