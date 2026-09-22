import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Copy, Diff, Eye, Maximize2, Minimize2, PenLine, Plus, Scan, X } from 'lucide-react'
import {
  createVersion,
  deleteVersion,
  getVersion,
  listVersions,
  scoreHistory,
  setCurrentVersion,
  type VersionRole,
} from '@/lib/api'
import { changedLines, countChanges, diffLines } from '@/lib/diff'
import { clearDraft, readDraft, writeDraft } from '@/lib/drafts'
import { predecessor } from '@/lib/history'
import { STAGE_LAYER } from '@/lib/layers'
import { keys } from '@/lib/query'
import { findRepeats } from '@/lib/repeats'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { labelOf, say as sayLabel, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { LayerProvider } from '@/components/ui/layer'
import { MarkedText, MarkedTextarea } from '@/components/ui/marked-text'
import { Markdown } from '@/components/ui/Markdown'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { SaveState } from '@/components/ui/SaveState'
import { Skeleton } from '@/components/ui/Skeleton'
import { VersionEditor } from '@/components/versions/VersionEditor'
import { ActionBar } from '@/components/assistant/ActionBar'
import { VersionList } from '@/components/versions/VersionList'

interface Props {
  workId: string
}

/**
 * How the open version is shown.
 *
 * `view` draws the body the way its role reads — a monospace column for
 * lyrics, rendered markdown for a review. `edit` is the same text in a box,
 * saving itself into the version of the sitting. Comparing is not a third
 * way of reading but a second column beside either: the version picked with
 * ± stands on the right with its lines that are gone marked, while the text
 * on the left is read or written with its new lines marked — so a rewrite
 * keeps the original in view, at any width.
 */
type Reading = 'view' | 'edit'

/**
 * How much of the screen the text takes.
 *
 * `inline` is the card. `expanded` gives the text the whole content area,
 * sidebar in place — entered by clicking into the text, because starting to
 * write is when the frame stops mattering. `focus` is the text alone over the
 * whole window. `Esc` steps back one level at a time.
 */
type Level = 'inline' | 'expanded' | 'focus'
type Pane = 'text' | 'comment'

/** Whether bodies in a role are drawn as markdown. See `VersionRole.body`. */
function readsAsMarkdown(roles: VersionRole[], role: string): boolean {
  return roles.find((r) => r.key === role)?.body === 'markdown'
}

// Versions of one role at a time: lyrics and style advance independently, and
// showing them interleaved would suggest otherwise.
export function VersionPanel({ workId }: Props) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const roles = useVocabulary(workId).version_roles

  // A role named in the address opens that lane: the Scenes tab sends the
  // shared context here with `?role=context`, and a link to a lane should
  // land in it rather than on the first role.
  const [params, setParams] = useSearchParams()
  const [role, setRole] = useState(params.get('role') ?? roles[0]?.key ?? '')
  // A version named in the address wins until something else is picked. That
  // is what lets a score row open the very draft it judged — and what makes
  // that link work from a note or a message, the same promise the tabs made.
  const asked = params.get('version')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  // A comparison named in the address, the way a version is: a link whose
  // source moved on points here with both — the version taken and the one
  // it is on now — and the diff is open on arrival.
  const [comparedId, setComparedId] = useState<string | null>(() => params.get('compare'))
  const [reading, setReading] = useState<Reading>('view')
  const [commentReading, setCommentReading] = useState<Reading>('view')
  const [stage, setStage] = useState<{ pane: Pane; level: Exclude<Level, 'inline'> } | null>(null)

  // The form for a version written from nothing or from a copy. It is not on
  // screen by default: revising is clicking into the text. It opens when
  // asked — a new version, a copy of one — or when the role has no version
  // yet and there is nothing to click into.
  const [composing, setComposing] = useState(false)
  // Drafts for that form belong to the work and the role they were typed
  // under, and outlive the window. See `lib/drafts`.
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const draft = drafts[role] ?? readDraft(workId, role)
  const setDraft = (body: string) => {
    setDrafts((all) => ({ ...all, [role]: body }))
    writeDraft(workId, role, body)
  }
  const [label, setLabel] = useState('')
  const [makeCurrentOnSave, setMakeCurrentOnSave] = useState(true)
  // The version the draft was copied from, when it was; recorded on the saved
  // version as its parent.
  const [derivedFrom, setDerivedFrom] = useState<string | null>(null)

  const versions = useQuery({
    queryKey: keys.versions(workId),
    queryFn: () => listVersions(workId),
  })

  // Every score this work has, so the list can put a mark beside the draft it
  // was given to. One request for the whole history rather than one per row,
  // and the Score tab has usually asked for it already — the same key, so the
  // two share an answer instead of fetching it twice.
  const scored = useQuery({
    queryKey: keys.scoreHistory(workId),
    queryFn: () => scoreHistory(workId),
  })

  // Version id to the score that speaks for it. Newest first is what the
  // history comes back as, so the first one seen per version is the one that
  // stands — a draft judged twice shows the later verdict, which is the same
  // rule the card reads by. Rounded, because a row has space for a number and
  // not for a decimal: the Score tab is where the working is.
  const scores: Record<string, number> = {}
  for (const score of scored.data ?? []) {
    if (score.version_id === null || score.version_id in scores) continue
    scores[score.version_id] = Math.round(score.total)
  }

  // The asked-for version also decides which lane is open: a link to a draft
  // that lands on the style tab has not opened it.
  const askedRole = (versions.data ?? []).find((version) => version.id === asked)?.role
  const shownRole = askedRole !== undefined && selectedId === null ? askedRole : role
  const summaries = (versions.data ?? []).filter((version) => version.role === shownRole)
  // Open the newest of this role by default, so the panel is never blank; an
  // explicit choice wins until it disappears.
  const openId =
    selectedId !== null && summaries.some((v) => v.id === selectedId)
      ? selectedId
      : asked !== null && summaries.some((v) => v.id === asked)
        ? asked
        : (summaries[0]?.id ?? null)

  // The previous version stays on screen while the next one loads. Without
  // this the pane unmounted for a frame every time the sitting minted a
  // version, and the box came back with the caret at the top — the second
  // keystroke of a revision landed at the start of the text.
  const open = useQuery({
    queryKey: keys.version(openId ?? ''),
    queryFn: () => getVersion(openId!),
    enabled: openId !== null,
    placeholderData: keepPreviousData,
  })

  // Roles written *about* this one. A profile that names none leaves the panel
  // exactly as it was: one role at a time, nothing beside it.
  const commentRoles = roles.filter((r) => r.comments_on === shownRole)
  const openSummary = summaries.find((version) => version.id === openId) ?? null

  // The commentary on the open revision — not the newest commentary there is.
  // A review of revision 2 says nothing about revision 5, and showing it beside
  // 5 would be the panel asserting something nobody wrote. Commentary that
  // says which version it is about (`about_version_id`, written by an action
  // started on that version) is paired by that; commentary that does not is
  // paired by revision number, the way it always was.
  const comments = (versions.data ?? []).filter(
    (version) =>
      openSummary !== null &&
      commentRoles.some((r) => r.key === version.role) &&
      (version.about_version_id !== null
        ? version.about_version_id === openSummary.id
        : version.revision === openSummary.revision),
  )
  const [openCommentId, setOpenCommentId] = useState<string | null>(null)
  const commentId =
    openCommentId !== null && comments.some((c) => c.id === openCommentId)
      ? openCommentId
      : (comments[0]?.id ?? null)

  const comment = useQuery({
    queryKey: keys.version(commentId ?? ''),
    queryFn: () => getVersion(commentId!),
    enabled: commentId !== null,
    placeholderData: keepPreviousData,
  })

  // The version beside the open one, when one was picked. The predecessor is
  // not assumed: it is offered first in the picker, one click away, rather
  // than opened by a mode of its own. A pick that no longer names a version
  // of this role, or names the open one, is no pick.
  const before = predecessor(summaries, openId)
  const againstId =
    comparedId !== null && comparedId !== openId && summaries.some((v) => v.id === comparedId)
      ? comparedId
      : null

  const compared = useQuery({
    queryKey: keys.version(againstId ?? ''),
    queryFn: () => getVersion(againstId!),
    enabled: againstId !== null,
  })

  // A version changes the list, the work's current pointer, the summary the
  // works list shows, and the history kept underneath the card.
  const refreshed = [keys.journal, keys.versions(workId), keys.work(workId), keys.works]

  const settle = () => {
    for (const key of refreshed) void client.invalidateQueries({ queryKey: key })
  }

  // The text on screen, saving itself. Two of them: the open version, and the
  // commentary beside it, which is a version of another role with its own
  // sitting.
  // The editor is handed a version only once it is the one asked for: the
  // placeholder above is the previous version, and adopting its text would
  // overwrite what was just typed.
  const editing = useBodyEditing({
    workId,
    role: shownRole,
    open: open.data?.id === openId ? open.data : undefined,
    onMinted: (id) => setSelectedId(id),
    failure: t('toast.versionSaveFailed'),
  })
  const commentEditing = useBodyEditing({
    workId,
    role: comment.data?.role ?? '',
    open: comment.data?.id === commentId ? comment.data : undefined,
    onMinted: (id) => setOpenCommentId(id),
    failure: t('toast.versionSaveFailed'),
  })

  const save = useMutation({
    mutationFn: () =>
      createVersion(workId, {
        role,
        body: draft,
        label: label.trim() === '' ? null : label.trim(),
        make_current: makeCurrentOnSave,
        parent_version_id: derivedFrom,
      }),
    onSuccess: (version) => {
      // The draft became a version; there is nothing left to keep.
      setDrafts((all) => ({ ...all, [role]: '' }))
      clearDraft(workId, role)
      setLabel('')
      setDerivedFrom(null)
      setComposing(false)
      // The new version shows itself: opened, in the list, read.
      setSelectedId(version.id)
      setReading('view')
      settle()
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const makeCurrent = useMutation({
    mutationFn: (versionId: string) => setCurrentVersion(workId, versionId),
    onSuccess: settle,
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const remove = useMutation({
    mutationFn: deleteVersion,
    onSuccess: (deletionId, versionId) => {
      if (selectedId === versionId) setSelectedId(null)
      if (comparedId === versionId) setComparedId(null)
      announceDeleted({
        client,
        deletionId,
        message: t('toast.versionDeleted'),
        refresh: refreshed,
        // Reopen what was being read when it was thrown away.
        onUndone: () => setSelectedId(versionId),
      })
    },
    onError: (cause) => say.failedTo(t('toast.versionSaveFailed'), cause),
  })

  const nameOf = (id: string | null): string => {
    const found = summaries.find((version) => version.id === id)
    if (found === undefined) return ''
    return found.label ?? t('versions.revision', { number: found.revision })
  }

  const comparing = againstId !== null && open.data != null && compared.data != null

  // A copy of a version, loaded into the form to be worked on as the next
  // one — for when the revision should not start from the open text as it
  // is: a rewrite that keeps the original open beside it, or a version that
  // wants a name. A draft already in the form is displaced rather than guarded
  // by a confirmation, with one click to take it back.
  const deriveFrom = async (id: string) => {
    const source = await client.fetchQuery({
      queryKey: keys.version(id),
      queryFn: () => getVersion(id),
    })
    if (source == null) return

    const displaced = drafts[source.role] ?? readDraft(workId, source.role)
    setRole(source.role)
    setSelectedId(id)
    setDraft(source.body)
    setDerivedFrom(id)
    setComposing(true)

    if (displaced.trim() === '') say.ok(t('toast.versionDerived'))
    else
      say.undoable(t('toast.versionDerived'), t('versions.restoreDraft'), () => {
        setDraft(displaced)
        setDerivedFrom(null)
      })
  }

  // Escape steps back: focus to expanded, expanded to the card. Bound while a
  // stage is open only, so it does not shadow anything else on the card.
  useEffect(() => {
    if (stage === null) return
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      setStage(stage.level === 'focus' ? { ...stage, level: 'expanded' } : null)
      if (stage.level === 'expanded') {
        if (stage.pane === 'text') setReading('view')
        else setCommentReading('view')
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [stage])

  // The comparison stays through it: writing with the original beside the
  // text is what the second column is for.
  const enterText = () => {
    setReading('edit')
    setStage({ pane: 'text', level: 'expanded' })
  }

  const textMarkdown = readsAsMarkdown(roles, shownRole)
  const formMarkdown = readsAsMarkdown(roles, role)
  const showForm = composing || (versions.isSuccess && summaries.length === 0)

  const textPane = (level: Level) =>
    open.data == null ? null : (
      <BodyPane
        level={level}
        title={`${labelOf(roles, open.data.role)} · ${
          open.data.label ?? t('versions.revision', { number: open.data.revision })
        } · ${open.data.created_at.slice(0, 10)}`}
        markdown={textMarkdown}
        body={open.data.body}
        reading={reading}
        onReading={setReading}
        compare={{
          against:
            comparing && compared.data != null
              ? { id: compared.data.id, body: compared.data.body, label: nameOf(againstId) }
              : null,
          candidates: summaries
            .filter((version) => version.id !== openId)
            .map((version) => ({
              id: version.id,
              label: nameOf(version.id),
              previous: version.id === before?.id,
            })),
          onPick: setComparedId,
        }}
        repeats
        fill={level === 'inline' && fill}
        editing={editing}
        onEnter={enterText}
        onLevel={(next) => setStage(next === 'inline' ? null : { pane: 'text', level: next })}
      />
    )

  const commentPane = (level: Level) =>
    comments.length === 0 ? null : (
      <BodyPane
        level={level}
        tabs={
          // One tab per kind of commentary — a look at the axes and a critique
          // of the lines answer different questions, and the predecessor kept
          // them as separate documents for that reason. With one kind this is
          // a label.
          <>
            {comments.map((entry) => (
              <button
                key={entry.id}
                type="button"
                onClick={() => setOpenCommentId(entry.id)}
                className={cn(
                  'cursor-pointer rounded-md px-2 py-0.5 transition-colors',
                  entry.id === commentId ? 'bg-soft text-text' : 'hover:text-text',
                )}
              >
                {labelOf(roles, entry.role)}
              </button>
            ))}
            <span className="ml-1">
              {t('versions.revision', { number: openSummary?.revision ?? 0 })}
            </span>
          </>
        }
        markdown={comment.data != null && readsAsMarkdown(roles, comment.data.role)}
        body={comment.data?.body ?? null}
        reading={commentReading}
        onReading={setCommentReading}
        fill={level === 'inline' && fill}
        editing={commentEditing}
        onEnter={() => {
          setCommentReading('edit')
          setStage({ pane: 'comment', level: 'expanded' })
        }}
        onLevel={(next) => setStage(next === 'inline' ? null : { pane: 'comment', level: next })}
      />
    )

  // The pane on stage renders through a portal — over the content area with
  // the sidebar in place, or over the whole window — and its inline place is
  // left empty rather than showing the same text twice.
  const staged = stage === null ? null : stage.pane === 'text' ? textPane(stage.level) : commentPane(stage.level)
  const stageHost =
    stage === null
      ? null
      : stage.level === 'focus'
        ? document.body
        : (document.getElementById('main-area') ?? document.body)

  // The lanes a person picks between. Commentary is not a lane: it belongs
  // beside what it comments on, and offering it here would show it stripped
  // of that.
  const lanes = roles.filter((r) => r.comments_on === undefined)
  const counts: Record<string, number> = {}
  for (const version of versions.data ?? []) {
    counts[version.role] = (counts[version.role] ?? 0) + 1
  }
  const pickLane = (next: string) => {
    setRole(next)
    // Picking a lane by hand ends the link's claim on the panel.
    if (asked !== null) setParams({}, { replace: true })
    // The selection and the comparison belonged to the role being left.
    setSelectedId(null)
    setComparedId(null)
    setReading('view')
  }

  // With the form open the right column is a page - the text above, the draft
  // below - and scrolls as one. Without it the text is the whole column and
  // scrolls inside its own frame, so the actions under it never move.
  const fill = !showForm

  return (
    // Two columns, each with its own scroll: the list of revisions on the
    // left, the open one on the right. Scrolling a history of twenty does not
    // move the text being read, and reading to the end of a long text does
    // not take the list away. The list is 262px at every window width, as in
    // the mockup: a list stacked above the text on a narrow window was a
    // second layout for the same tab.
    <section className="grid min-h-0 flex-1 grid-cols-[262px_minmax(0,1fr)] gap-3">
      <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-line bg-raise">
        {/* The lanes as chips, with how many each holds. A craft ships four
            of them (text, style, review, critique) and a two-way switch does
            not stretch to four; with one lane there is nothing to pick, and
            the caption says what the list is. */}
        <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-line p-2">
          {lanes.length > 1 ? (
            lanes.map((lane) => (
              <button
                key={lane.key}
                type="button"
                aria-pressed={lane.key === shownRole}
                onClick={() => pickLane(lane.key)}
                className={cn(
                  'inline-flex cursor-pointer items-center gap-1.5 rounded-md px-2 py-0.5 text-xs transition-colors',
                  lane.key === shownRole
                    ? 'bg-accent-soft font-semibold text-accent-2'
                    : 'text-dim hover:bg-soft hover:text-text',
                )}
              >
                {sayLabel(lane.label)}
                <span className="font-mono text-[10px] opacity-75">{counts[lane.key] ?? 0}</span>
              </button>
            ))
          ) : (
            <span className="px-1 text-[10px] font-semibold uppercase tracking-[0.08em] text-faint">
              {t('versions.title')} · {summaries.length}
            </span>
          )}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
          <VersionList
            versions={summaries}
            loading={versions.isPending}
            scores={scores}
            openId={openId}
            comparedId={againstId}
            onOpen={(id) => {
              setSelectedId(id)
              // Opening another version is reading it, whatever the last one
              // was being done to.
              setReading('view')
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
        </div>

        {/* At the foot of the list it adds to, where the next row will
            appear. */}
        <div className="shrink-0 border-t border-line p-1.5">
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-center"
            onClick={() => {
              setDerivedFrom(null)
              setComposing(true)
            }}
            title={t('versions.newHint')}
          >
            <Plus aria-hidden className="size-3.5" />
            {t('versions.new')}
          </Button>
        </div>
      </div>

      <div
        className={cn(
          'flex min-h-0 min-w-0 flex-col gap-3',
          !fill && 'overflow-x-hidden overflow-y-auto [scrollbar-gutter:stable]',
        )}
      >
        {open.isPending && openId !== null && <Skeleton className="h-32 w-full shrink-0" />}

        {/* The text and what was written about it, side by side. Reading a
            review away from the lines it discusses is reading half of it —
            which is exactly what the predecessor's two panels got right. The
            split only happens when there is a review of this very revision
            and room for both; below that the review sits underneath, and the
            two share the column's height. */}
        {open.data != null && (
          <div
            className={cn(
              'grid min-w-0 gap-3',
              fill && 'min-h-0 flex-1 auto-rows-[minmax(0,1fr)]',
              comments.length > 0 && '2xl:grid-cols-2',
            )}
          >
            {stage?.pane === 'text' ? <StagePlaceholder /> : textPane('inline')}
            {comments.length > 0 &&
              (stage?.pane === 'comment' ? <StagePlaceholder /> : commentPane('inline'))}
          </div>
        )}

        {/* The profile's actions, on this very revision: a critique or a
            score started here reads the text above and comes back bound to
            it — the same buttons the overview has, with the version named. */}
        {open.data != null && reading !== 'edit' && (
          <div className="shrink-0">
            <ActionBar
              workId={workId}
              menu
              versionId={open.data.id}
              hint={t('versions.actionsOn', {
                name:
                  open.data.label ?? t('versions.revision', { number: open.data.revision }),
              })}
            />
          </div>
        )}

        {showForm && (
          <VersionEditor
            draft={draft}
            onDraftChange={setDraft}
            label={label}
            onLabelChange={setLabel}
            makeCurrent={makeCurrentOnSave}
            onMakeCurrentChange={setMakeCurrentOnSave}
            onSave={() => {
              if (draft.trim() !== '') save.mutate()
            }}
            onCancel={
              composing && summaries.length > 0
                ? () => {
                    setComposing(false)
                    setDerivedFrom(null)
                  }
                : undefined
            }
            saving={save.isPending}
            kept={draft.trim() !== ''}
            markdown={formMarkdown}
          />
        )}
      </div>

      {staged !== null && stageHost !== null && createPortal(staged, stageHost)}
    </section>
  )
}

/** Where the staged pane was, so the card keeps its shape underneath. */
function StagePlaceholder() {
  const { t } = useTranslation()
  return (
    <div className="flex min-h-32 items-center justify-center rounded-xl border border-dashed border-line text-xs text-faint">
      {t('versions.onStage')}
    </div>
  )
}

interface PaneProps {
  level: Level
  title?: string
  tabs?: ReactNode
  markdown: boolean
  body: string | null
  reading: Reading
  onReading: (mode: Reading) => void
  /** The comparison: what stands beside the text, and what may. Absent on a
   *  pane that does not compare — the commentary. */
  compare?: {
    against: { id: string; body: string; label: string } | null
    candidates: { id: string; label: string; previous: boolean }[]
    onPick: (id: string | null) => void
  }
  /** Whether repeated words are marked while editing. */
  repeats?: boolean
  /** On the card, taking the whole height it is given and scrolling inside,
   *  rather than capping itself at a reading height. */
  fill?: boolean
  editing: ReturnType<typeof useBodyEditing>
  /** Clicking into the text: edit, on the whole content area. */
  onEnter: () => void
  onLevel: (level: Level) => void
}

/** How many tints a repeated word may be drawn in before they cycle. */
const REPEAT_TINTS = 6

/**
 * One body: a version's text, read or edited, with another beside it when
 * asked — on the card, over the content area, or over the whole window.
 *
 * Reading is the default. The text is a thing to look at until it is clicked,
 * and then it is a thing to type into, with the frame out of the way. The
 * pencil edits in place for the small fix that does not need the room.
 */
function BodyPane({
  level,
  title,
  tabs,
  markdown,
  body,
  reading,
  onReading,
  compare,
  repeats = false,
  fill = false,
  editing,
  onEnter,
  onLevel,
}: PaneProps) {
  const { t } = useTranslation()
  const staged = level !== 'inline'
  // Whether the frame is as tall as the box around it: always on stage, and on
  // the card whenever the column is the text's alone.
  const tall = staged || fill

  const modes: { mode: Reading; icon: typeof Eye; label: string }[] = [
    { mode: 'view', icon: Eye, label: t('versions.view') },
    { mode: 'edit', icon: PenLine, label: t('versions.edit') },
  ]

  // What is on the left right now: the text being typed, or the body as it
  // is on disk. The comparison and the repeats are read off this, so both
  // follow the keystrokes.
  const text = reading === 'edit' ? editing.text : (body ?? '')
  const against = compare?.against ?? null
  const diff = useMemo(
    () => (against === null ? null : diffLines(against.body, text)),
    [against, text],
  )
  const moved = useMemo(() => (diff === null ? null : changedLines(diff)), [diff])
  const counts = useMemo(() => (diff === null ? null : countChanges(diff)), [diff])
  const found = useMemo(
    () => (repeats && reading === 'edit' ? findRepeats(text) : null),
    [repeats, reading, text],
  )

  const addedLines = useMemo(
    () => [...(moved?.added ?? [])].map((line) => ({ line, className: 'bg-good-soft' })),
    [moved],
  )
  const removedLines = useMemo(
    () => [...(moved?.removed ?? [])].map((line) => ({ line, className: 'bg-bad-soft' })),
    [moved],
  )
  const marks = useMemo(
    () =>
      (found?.marks ?? []).map((mark) => ({
        start: mark.start,
        end: mark.end,
        className: `repeat-${mark.group % REPEAT_TINTS}`,
      })),
    [found],
  )

  // The metrics both layers of the editor share, and the read text with them:
  // the marks are drawn on a mirror and have to land on the same letters.
  // `selectable` with them: the registry copy grants nothing of its own, and a
  // version is someone's writing, which has to be copyable wherever it shows.
  const metrics = cn('selectable px-3 py-2.5 text-sm leading-relaxed', !markdown && 'font-mono')

  const content =
    body === null ? (
      <Skeleton className="h-32 w-full" />
    ) : reading === 'edit' ? (
      <MarkedTextarea
        autoFocus
        value={editing.text}
        onChange={editing.setText}
        onBlur={() => void editing.flush()}
        onKeyDown={(event) => {
          // The key everyone presses anyway. The text is already saving
          // itself; this writes it now rather than after the pause.
          if ((event.ctrlKey || event.metaKey) && event.key === 's') {
            event.preventDefault()
            void editing.flush()
          }
        }}
        aria-label={t('versions.edit')}
        marks={marks}
        lineMarks={addedLines}
        className={cn('block w-full', metrics, tall ? 'min-h-full' : 'min-h-[28rem]')}
      />
    ) : (
      // Reading. The whole body is the way in: clicking it is what starting
      // to write looks like, so it is a button rather than a hint.
      <div
        role="button"
        tabIndex={0}
        title={t('versions.clickToEdit')}
        onClick={onEnter}
        onKeyDown={(event) => {
          if (event.key === 'Enter') onEnter()
        }}
        className={cn(
          'cursor-text outline-none focus-visible:ring-2 focus-visible:ring-accent',
          tall && 'min-h-full',
        )}
      >
        {markdown ? (
          <div className="px-3 py-2.5">
            <Markdown body={body} />
          </div>
        ) : (
          <MarkedText text={body} lineMarks={addedLines} className={metrics} />
        )}
      </div>
    )

  // The ± control: one other version is compared with in one press; several
  // are offered by name, the predecessor first — the question asked most
  // often of a history should not cost a hunt through the list.
  const candidates = compare?.candidates ?? []
  const comparer =
    compare === undefined || candidates.length === 0 ? null : against !== null ? (
      <Button
        variant="soft"
        size="icon-sm"
        aria-pressed
        title={t('versions.stopComparing')}
        aria-label={t('versions.stopComparing')}
        onClick={() => compare.onPick(null)}
      >
        <Diff aria-hidden />
      </Button>
    ) : candidates.length === 1 ? (
      <Button
        variant="icon"
        size="icon-sm"
        title={t('versions.compareWith', { name: candidates[0]!.label })}
        aria-label={t('versions.compareWith', { name: candidates[0]!.label })}
        onClick={() => compare.onPick(candidates[0]!.id)}
      >
        <Diff aria-hidden />
      </Button>
    ) : (
      <Menu>
        <MenuTrigger
          render={
            <Button
              variant="icon"
              size="icon-sm"
              title={t('versions.compareMenu')}
              aria-label={t('versions.compareMenu')}
            />
          }
        >
          <Diff aria-hidden />
        </MenuTrigger>
        <MenuPopup align="end">
          {[...candidates]
            .sort((a, b) => Number(b.previous) - Number(a.previous))
            .map((candidate) => (
              <MenuItem key={candidate.id} onClick={() => compare.onPick(candidate.id)}>
                <span className="truncate">{candidate.label}</span>
                {candidate.previous && (
                  <span className="ml-auto pl-3 text-[11px] text-faint">{t('versions.previous')}</span>
                )}
              </MenuItem>
            ))}
        </MenuPopup>
      </Menu>
    )

  const frame = (
    <article
      className={cn(
        'flex min-w-0 flex-col overflow-hidden rounded-xl border border-line bg-bg',
        tall && 'h-full min-h-0',
      )}
    >
      <header className="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2 text-xs text-dim">
        {title !== undefined && <span>{title}</span>}
        {tabs}
        <SaveState status={editing.status} className="ml-2" />
        <div className="ml-auto flex items-center gap-1">
          {modes.map(({ mode, icon: Icon, label }) => (
            <Button
              key={mode}
              variant={reading === mode ? 'soft' : 'icon'}
              size="icon-sm"
              aria-pressed={reading === mode}
              title={label}
              aria-label={label}
              onClick={() => {
                if (reading === 'edit' && mode !== 'edit') void editing.flush()
                onReading(mode)
              }}
            >
              <Icon aria-hidden />
            </Button>
          ))}
          {comparer}
          {/* The body, on the clipboard. A style prompt exists to be pasted
              into something else, and the way to get one out was to click
              into the text, select it all and copy - on a body that opens in
              reading mode, where a click starts an edit. */}
          <Button
            variant="icon"
            size="icon-sm"
            title={t('versions.copyBody')}
            aria-label={t('versions.copyBody')}
            disabled={body === null || body === ''}
            onClick={() => {
              if (body === null) return
              // The tick only once the clipboard confirms, the rule from
              // v0.28: saying a copy succeeded when it did not is worse than
              // saying nothing.
              navigator.clipboard.writeText(body).then(
                () => say.ok(t('versions.bodyCopied')),
                (cause: unknown) => say.failedTo(t('versions.copyBody'), cause),
              )
            }}
          >
            <Copy aria-hidden />
          </Button>
          <span aria-hidden className="mx-1 h-4 w-px bg-line" />
          <Button
            variant={level === 'expanded' ? 'soft' : 'icon'}
            size="icon-sm"
            title={level === 'inline' ? t('versions.expand') : t('versions.collapse')}
            aria-label={level === 'inline' ? t('versions.expand') : t('versions.collapse')}
            onClick={() => onLevel(level === 'inline' ? 'expanded' : 'inline')}
          >
            {level === 'inline' ? <Maximize2 aria-hidden /> : <Minimize2 aria-hidden />}
          </Button>
          <Button
            variant={level === 'focus' ? 'soft' : 'icon'}
            size="icon-sm"
            title={level === 'focus' ? t('versions.exitFocus') : t('versions.focus')}
            aria-label={level === 'focus' ? t('versions.exitFocus') : t('versions.focus')}
            onClick={() => onLevel(level === 'focus' ? 'expanded' : 'focus')}
          >
            <Scan aria-hidden />
          </Button>
        </div>
      </header>

      {/* The words this text leans on, while it is being written. Nothing is
          drawn when there are none: a strip saying "no repeats" would be a
          strip taking the room the text wants. */}
      {found !== null && found.groups.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5 border-b border-line px-3 py-1.5 text-[11px] text-dim">
          <span className="mr-1 font-medium">{t('versions.repeats')}</span>
          {found.groups.map((group, index) => (
            <span
              key={group.stem}
              className={cn('rounded-sm px-1.5 py-px', `repeat-${index % REPEAT_TINTS}`)}
            >
              {group.word} ×{group.count}
            </span>
          ))}
        </div>
      )}

      {/* One scroller for both columns, so the two texts move together. */}
      <div
        className={cn(
          'grid min-h-0',
          against === null ? 'grid-cols-1' : 'grid-cols-2',
          tall ? 'flex-1 overflow-auto' : 'max-h-[32rem] overflow-auto',
        )}
      >
        <div className="min-w-0">{content}</div>
        {against !== null && compare !== undefined && (
          <aside className="flex min-w-0 flex-col border-l border-line">
            <header className="sticky top-0 z-10 flex items-center gap-2 border-b border-line bg-bg px-3 py-1 text-[11px] text-dim">
              <span className="truncate font-medium">{against.label}</span>
              {counts !== null && (
                <span className="ml-auto whitespace-nowrap text-faint">
                  {counts.added === 0 && counts.removed === 0
                    ? t('versions.diffSame')
                    : [
                        t('versions.diffAdded', { count: counts.added }),
                        t('versions.diffRemoved', { count: counts.removed }),
                      ].join(' · ')}
                </span>
              )}
              <Button
                variant="icon"
                size="icon-sm"
                title={t('versions.stopComparing')}
                aria-label={t('versions.stopComparing')}
                onClick={() => compare.onPick(null)}
              >
                <X aria-hidden />
              </Button>
            </header>
            <MarkedText text={against.body} lineMarks={removedLines} className={cn(metrics, 'text-dim')} />
          </aside>
        )}
      </div>
    </article>
  )

  if (level === 'inline') return frame

  /* The stage sits on the token ladder, not beside it.
   *
   * It carried a raw Tailwind `z-50` - a literal 50, chosen to clear the page
   * rather than to take a place in the scale. Menus are `--z-menu`, a 30, so
   * the ± control opened its list of versions UNDERNEATH the stage covering
   * the window: the button highlighted, nothing appeared, and it read as
   * broken. `LayerProvider` hands anything opened in here a floor above the
   * stage, the same way a dialog does for the popups inside it. */
  return (
    <LayerProvider rung="stage-popup">
      <div
        className={cn(
          'flex flex-col gap-2 bg-bg p-6',
          level === 'focus' ? 'fixed inset-0' : 'absolute inset-0',
        )}
        style={{ zIndex: STAGE_LAYER }}
      >
        <p className="text-xs text-faint">{t('versions.stageHint')}</p>
        <div className="flex min-h-0 flex-1 flex-col">{frame}</div>
      </div>
    </LayerProvider>
  )
}
