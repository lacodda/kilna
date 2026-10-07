import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { BadgeCheck, CornerUpLeft, MessageSquareText, Plus } from 'lucide-react'
import type { VersionSummary } from '@/lib/api/types'
import { deleteVersion, setCurrentVersion } from '@/lib/api/versions'
import { commentaryOn, isCommentary, subjectOf } from '@/lib/commentary'
import { formatDay } from '@/lib/format'
import { parentIn, predecessor } from '@/lib/history'
import { queries } from '@/lib/query/queries'
import { refresh } from '@/lib/query/refresh'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { announceDeleted } from '@/lib/trash'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { labelOf, say, useVocabulary } from '@/lib/useProfile'
import { graphOf } from '@/lib/versionTree'
import { formatTotal } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Skeleton, SkeletonList } from '@/components/ui/skeleton'
import { ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { ActionBar } from '@/features/assistant/ActionBar'
import { ToCanonButton } from '@/features/canon/ToCanonButton'
import { ToRegisterButton } from '@/features/register/ToRegisterButton'
import { BodyPane, type Reading } from '@/features/work/tabs/versions/BodyPane'
import { Commentary } from '@/features/work/tabs/versions/Commentary'
import { CompareControl } from '@/features/work/tabs/versions/CompareControl'
import { LaneChips } from '@/features/work/tabs/versions/LaneChips'
import { Stage } from '@/features/work/tabs/versions/Stage'
import { VersionEditor } from '@/features/work/tabs/versions/VersionEditor'
import { VersionList } from '@/features/work/tabs/versions/VersionList'
import { useLaneStats } from '@/features/work/tabs/versions/useLaneStats'
import { useVersionDraft } from '@/features/work/tabs/versions/useVersionDraft'

interface Props {
  workId: string
}

/**
 * The Versions tab: the mockup's `.split` - the roles and their versions on
 * the left, the open one on the right, each scrolling on its own.
 *
 * Versions of one role at a time: lyrics and style advance independently, and
 * showing them interleaved would suggest otherwise. What was written about the
 * open revision stands beside it. The pieces are their own files - the list,
 * the chips, the pane a text is read and written in, the comparison, the
 * commentary, the form for a new version, the stage - and this is the part
 * that knows which version is open and why.
 */
export function VersionPanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const roles = useVocabulary(workId).version_roles

  const [params, setParams] = useSearchParams()
  // A version named in the address wins until something else is picked. That
  // is what lets a score row open the very draft it judged - and what makes
  // that link work from a note or a message, the same promise the tabs made.
  const asked = params.get('version')
  // The lane picked by hand, or by opening a version in it. Until there is
  // one, the lane is the asked-for version's, then the one the address
  // names (the Scenes tab sends the shared context here with
  // `?role=context`), then the profile's first.
  const [pickedRole, setPickedRole] = useState<string | null>(null)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  // A comparison named in the address, the way a version is: a link whose
  // source moved on points here with both - the version taken and the one
  // it is on now - and the diff is open on arrival.
  const [comparedId, setComparedId] = useState<string | null>(() => params.get('compare'))
  // A new address wins again over what was picked by hand before it came: a
  // link to a version or a lane followed while the tab is open - from search,
  // from a note - opened the lane picked earlier instead. Only an address
  // that names something: picking a lane clears the address, and that is the
  // pick winning, not a new link arriving.
  const address = `${asked ?? ''}|${params.get('role') ?? ''}`
  const [seenAddress, setSeenAddress] = useState(address)
  if (address !== seenAddress) {
    setSeenAddress(address)
    if (address !== '|') {
      setPickedRole(null)
      setSelectedId(null)
    }
  }
  const [reading, setReading] = useState<Reading>('view')
  const [staged, setStaged] = useState<'text' | 'comment' | null>(null)
  // Whether the commentary on the open revision stands beside it. On by
  // default - reading a review away from its text is reading half of it -
  // and one press gives the text the panel's whole width on a narrow window.
  const [besideShown, setBesideShown] = useState(true)

  const versions = useQuery(queries.versions(workId))
  const all = versions.data ?? []

  // Every score this work has, so the list can put a mark beside the draft it
  // was given to. One request for the whole history rather than one per row,
  // and the Score tab has usually asked for it already - the same key, so the
  // two share an answer instead of fetching it twice.
  const scored = useQuery(queries.scoreHistory(workId))

  // Version id to the score that speaks for it. Newest first is what the
  // history comes back as, so the first one seen per version is the one that
  // stands - a draft judged twice shows the later verdict, which is the same
  // rule the card reads by. Said the way a total is said everywhere on the
  // card (`formatTotal`): a row rounded to whole numbers read 78 beside a
  // header that said 77.5, and a tier that starts at 78 not reached.
  const scores: Record<string, string> = {}
  for (const score of scored.data ?? []) {
    if (score.version_id === null || score.version_id in scores) continue
    scores[score.version_id] = formatTotal(score.total)
  }

  const known = (key: string | null | undefined) =>
    key != null && roles.some((r) => r.key === key) ? key : undefined
  const lane =
    pickedRole ??
    known(all.find((version) => version.id === asked)?.role) ??
    known(params.get('role')) ??
    roles[0]?.key ??
    ''
  const commentaryLane = isCommentary(roles, lane)
  // One list per lane while the answer stands, so what is worked out from
  // it - the tree, the figures of each row - is not worked out again on
  // every keystroke of the text beside it.
  const summaries = useMemo(
    () => (versions.data ?? []).filter((version) => version.role === lane),
    [versions.data, lane],
  )
  // Which version was written from which, beside the list (ADR 0055). A
  // review is written about a revision, not from another review.
  const graph = useMemo(
    () => (commentaryLane ? null : graphOf(summaries)),
    [commentaryLane, summaries],
  )
  const stats = useLaneStats(summaries, !commentaryLane)

  // Open the newest of this lane by default, so the panel is never blank; an
  // explicit choice wins until it disappears.
  const openId =
    selectedId !== null && summaries.some((v) => v.id === selectedId)
      ? selectedId
      : asked !== null && summaries.some((v) => v.id === asked)
        ? asked
        : (summaries[0]?.id ?? null)
  const openSummary = summaries.find((version) => version.id === openId) ?? null

  // The previous version stays on screen while the next one loads. Without
  // this the pane unmounted for a frame every time the sitting minted a
  // version, and the box came back with the caret at the top - the second
  // keystroke of a revision landed at the start of the text.
  const open = useQuery({
    ...queries.version(openId ?? ''),
    enabled: openId !== null,
    placeholderData: keepPreviousData,
  })

  // What was written about the open revision, and - in a lane of commentary -
  // the revision a review is about. See `lib/commentary`.
  const comments = commentaryOn(all, roles, openSummary)
  const subject = commentaryLane ? subjectOf(all, roles, openSummary) : null

  // The version beside the open one, when one was picked. A pick that no
  // longer names a version of this lane, or names the open one, is no pick.
  // `before` is the one compared by default: the version the open one was
  // written from, or the revision below it (`lib/history`).
  const before = predecessor(summaries, openId)
  const beforeIsParent = before !== null && parentIn(summaries, openId)?.id === before.id
  // Its text, for the marks on the open one: what changed since it is drawn
  // in the margin of every draft without a press (#24). A review is not a
  // draft of the review before it.
  const previous = useQuery({
    ...queries.version(before?.id ?? ''),
    enabled: before !== null && !commentaryLane,
  })
  const againstId =
    comparedId !== null && comparedId !== openId && summaries.some((v) => v.id === comparedId)
      ? comparedId
      : null
  const compared = useQuery({ ...queries.version(againstId ?? ''), enabled: againstId !== null })

  // Opening a version makes its lane the panel's own. The lane a link opened
  // used to hold only until something was picked: a click on another review
  // fell back to the first lane and its newest text, and so did a version
  // minted by typing into a text opened by a link. The lane no longer hangs
  // on the selection, and once a version is opened it no longer hangs on the
  // link either.
  const show = (id: string) => {
    setPickedRole(lane)
    setSelectedId(id)
  }

  // Straight to a version of any lane - the text a review is about.
  const goTo = (version: VersionSummary) => {
    setPickedRole(version.role)
    setSelectedId(version.id)
    setComparedId(null)
    setReading('view')
  }

  // The text on screen, saving itself. The editor is handed a version only
  // once it is the one asked for: the placeholder above is the previous
  // version, and adopting its text would overwrite what was just typed.
  const editing = useBodyEditing({
    workId,
    role: lane,
    open: open.data?.id === openId ? open.data : undefined,
    onMinted: show,
    failure: t('toast.versionSaveFailed'),
    current: !commentaryLane,
  })

  const skeleton = roles.find((role) => role.key === lane)?.skeleton
  const form = useVersionDraft({
    workId,
    role: lane,
    skeleton: skeleton == null ? '' : say(skeleton),
    // The new version shows itself: opened, in the list, read.
    onSaved: (version) => {
      show(version.id)
      setReading('view')
    },
  })
  const source = useQuery({
    ...queries.version(form.derivedFrom ?? ''),
    enabled: form.derivedFrom !== null,
  })

  const makeCurrent = useAppMutation({
    mutationFn: (versionId: string) => setCurrentVersion(workId, versionId),
    failure: 'toast.versionSaveFailed',
    refresh: refresh.version(workId),
  })

  const remove = useAppMutation({
    mutationFn: deleteVersion,
    failure: 'toast.versionSaveFailed',
    onSuccess: (deletionId, versionId) => {
      if (selectedId === versionId) setSelectedId(null)
      if (comparedId === versionId) setComparedId(null)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.versionDeleted'),
        refresh: refresh.version(workId),
        // Reopen what was being read when it was thrown away.
        onUndone: () => setSelectedId(versionId),
      })
    },
  })

  const nameOf = (version: { label: string | null; revision: number }): string =>
    version.label ?? t('versions.revision', { number: version.revision })

  // A copy of a version, in the form, to be worked on as the next one. The
  // version copied stays open in the list, and stands beside the draft.
  const deriveFrom = async (id: string) => {
    const copied = await client.fetchQuery(queries.version(id))
    if (copied == null) return
    setPickedRole(copied.role)
    setSelectedId(id)
    form.derive(copied)
  }

  const pickLane = (next: string) => {
    setPickedRole(next)
    // Picking a lane by hand ends the link's claim on the panel.
    if (asked !== null) setParams({}, { replace: true })
    // The selection, the comparison and the form belonged to the lane being
    // left. The form's draft is kept, and comes back with the form.
    setSelectedId(null)
    setComparedId(null)
    setReading('view')
    form.close()
  }

  const counts: Record<string, number> = {}
  for (const version of all) counts[version.role] = (counts[version.role] ?? 0) + 1

  // The form takes the detail while it is open, and whenever a lane of text
  // has nothing yet - there is nothing to read, only something to write. A
  // lane of commentary is written by the actions run on a text, so it gets no
  // form of its own: a review typed here would be about no revision.
  const showForm =
    !commentaryLane && (form.composing || (versions.isSuccess && summaries.length === 0))

  const list = (
    <Pane
      label={t('versions.title')}
      bodyClassName="p-1.5"
      // Every role as a chip, with how many versions it holds. With one role
      // there is nothing to pick, and the caption says what the list is.
      head={
        roles.length > 1 ? (
          <LaneChips roles={roles} counts={counts} value={lane} onPick={pickLane} />
        ) : (
          <span className="caption">
            {t('versions.title')} · {summaries.length}
          </span>
        )
      }
      // At the foot of the list it adds to, where the next row will appear.
      foot={
        commentaryLane ? undefined : (
          <Button
            variant={showForm ? 'soft' : 'ghost'}
            size="sm"
            className="w-full justify-center"
            aria-pressed={showForm}
            onClick={form.begin}
            title={t('versions.newHint')}
          >
            <Plus aria-hidden />
            {t('versions.new')}
          </Button>
        )
      }
    >
      <Loaded query={versions} plain skeleton={<SkeletonList rows={4} />}>
        {() => (
          <VersionList
            versions={summaries}
            graph={graph}
            stats={stats}
            empty={
              <EmptyState
                plain
                title={t(commentaryLane ? 'versions.noCommentary' : 'versions.none')}
                className="p-2"
              />
            }
            commentary={commentaryLane}
            scores={scores}
            openId={openId}
            comparedId={againstId}
            onOpen={(id) => {
              show(id)
              // Opening a version is reading it, whatever the last one was
              // being done to - and the form, if it was open, is put away:
              // its draft is kept.
              setReading('view')
              form.close()
              // A comparison with the predecessor follows the step: each
              // revision against its own. A comparison with a version picked
              // by hand stays where it was pointed - an original kept beside
              // a history being walked - unless the step lands on it.
              if (againstId !== null && againstId === before?.id) {
                setComparedId(predecessor(summaries, id)?.id ?? null)
              } else if (againstId === id) {
                setComparedId(null)
              }
            }}
            onCompare={(id) => setComparedId(againstId === id ? null : id)}
            onMakeCurrent={(id) => makeCurrent.mutate(id)}
            onDelete={(id) => remove.mutate(id)}
            onDeriveFrom={(id) => void deriveFrom(id)}
          />
        )}
      </Loaded>
    </Pane>
  )

  const shown = open.data ?? null
  const textPane =
    shown === null ? null : (
      <BodyPane
        label={`${labelOf(roles, shown.role)} · ${nameOf(shown)}`}
        who={
          <span className="truncate font-mono text-xs text-faint">
            {labelOf(roles, shown.role)} · {nameOf(shown)} · {formatDay(shown.created_at)}
          </span>
        }
        tools={
          <>
            {/* A review is read with its text; this is the way back to it. */}
            {subject !== null && (
              <Button
                variant="ghost"
                size="sm"
                title={t('versions.openSubject')}
                onClick={() => goTo(subject)}
              >
                <CornerUpLeft aria-hidden />
                <span className="@max-2xl:sr-only">
                  {labelOf(roles, subject.role)} · v{subject.revision}
                </span>
              </Button>
            )}
            <CompareControl
              againstId={againstId}
              candidates={summaries
                .filter((version) => version.id !== openId)
                .map((version) => ({
                  id: version.id,
                  label: nameOf(version),
                  revision: version.revision,
                  previous: version.id === before?.id,
                  parent: version.id === before?.id && beforeIsParent,
                }))}
              onPick={setComparedId}
            />
            {!commentaryLane && openSummary !== null && !openSummary.is_current && (
              <Button
                variant="ghost"
                size="sm"
                title={t('versions.makeCurrent')}
                disabled={makeCurrent.isPending}
                onClick={() => makeCurrent.mutate(openSummary.id)}
              >
                <BadgeCheck aria-hidden />
                <span className="@max-2xl:sr-only">{t('versions.setCurrent')}</span>
              </Button>
            )}
            {/* The profile's actions, on this very revision: a critique or a
                score started here reads the text and comes back bound to it.
                Not while it is being written: the revision the action would
                read is the one the typing is replacing. */}
            {/* Selected lines, handed to the canon: a fact read off the very
                line it came from, with this version as its source. */}
            {!commentaryLane && reading !== 'edit' && (
              <ToCanonButton workId={workId} versionId={shown.id} />
            )}
            {/* A selected word, into the register of repeats - while reading
                or while writing, where a word leaned on again is noticed. */}
            {!commentaryLane && <ToRegisterButton />}
            {!commentaryLane && reading !== 'edit' && (
              <ActionBar
                workId={workId}
                menu
                versionId={shown.id}
                hint={t('versions.actionsOn', { name: nameOf(shown) })}
              />
            )}
            {comments.length > 0 && (
              <Button
                variant={besideShown ? 'soft' : 'icon'}
                size="icon-sm"
                aria-pressed={besideShown}
                title={t('versions.commentaryBeside')}
                aria-label={t('versions.commentaryBeside')}
                onClick={() => setBesideShown((on) => !on)}
              >
                <MessageSquareText aria-hidden />
              </Button>
            )}
          </>
        }
        markdown={roles.find((r) => r.key === shown.role)?.body === 'markdown'}
        body={shown.body}
        reading={reading}
        onReading={setReading}
        against={
          againstId !== null && compared.data != null
            ? {
                label: nameOf(compared.data),
                body: compared.data.body,
                onClose: () => setComparedId(null),
              }
            : null
        }
        previous={
          !commentaryLane && before !== null && previous.data?.id === before.id
            ? {
                label: `v${before.revision}`,
                body: previous.data.body,
                onCompare: () => setComparedId(before.id),
              }
            : null
        }
        repeats={!commentaryLane}
        // The role says whether its text is sung (ADR 0053): a lyric is, a
        // style prompt in the lane beside it is not.
        sung={roles.find((r) => r.key === shown.role)?.sung === true}
        editing={editing}
        staged={staged === 'text'}
        onStage={(on) => setStaged(on ? 'text' : null)}
      />
    )

  const detail = showForm ? (
    <VersionEditor
      roleLabel={labelOf(roles, lane)}
      draft={form.draft}
      onDraftChange={form.setDraft}
      label={form.label}
      onLabelChange={form.setLabel}
      makeCurrent={form.makeCurrent}
      onMakeCurrentChange={form.setMakeCurrent}
      onSave={form.save}
      onCancel={form.composing && summaries.length > 0 ? form.close : undefined}
      saving={form.saving}
      kept={form.draft.trim() !== '' && !form.fromSkeleton}
      markdown={roles.find((r) => r.key === lane)?.body === 'markdown'}
      source={
        form.derivedFrom === null
          ? undefined
          : source.data == null
            ? null
            : { label: nameOf(source.data), body: source.data.body }
      }
    />
  ) : commentaryLane && versions.isSuccess && summaries.length === 0 ? (
    // Nothing written about the text in this role yet, and nothing to write
    // here: the invitation says where commentary comes from.
    <div className="flex min-h-0 flex-1 items-center justify-center rounded-lg border border-line bg-raise p-6">
      <EmptyState
        title={t('versions.noCommentary')}
        body={t('versions.noCommentaryHint', {
          role: labelOf(roles, roles.find((r) => r.key === lane)?.comments_on ?? ''),
        })}
      />
    </div>
  ) : textPane === null ? (
    // Still coming: the shape of the panel it will be. A list that failed to
    // load says so, with a way to try again, and the detail stays empty.
    versions.isError ? null : (
      <div className="flex min-h-0 flex-1 flex-col rounded-lg border border-line bg-raise p-4">
        <Skeleton className="h-32 w-full" />
      </div>
    )
  ) : (
    // The text and what was written about it, side by side, each as tall as
    // the column. The one on the stage is drawn there instead, through a
    // portal that keeps its state here.
    <div className="flex min-h-0 min-w-0 flex-1 gap-2.5">
      {staged === 'text' ? <Stage onClose={() => setStaged(null)}>{textPane}</Stage> : textPane}
      {comments.length > 0 && besideShown && openSummary !== null && (
        <Commentary
          workId={workId}
          roles={roles}
          comments={comments}
          revision={openSummary.revision}
          staged={staged === 'comment'}
          onStage={(on) => setStaged(on ? 'comment' : null)}
        />
      )}
    </div>
  )

  // The list keeps its width at every window width, as in the mockup: a list
  // stacked above the text on a narrow window was a second layout for the
  // same tab.
  return <ListDetail list={list} detail={detail} />
}
