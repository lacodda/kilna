import { useEffect, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Diff, Eye, Maximize2, Minimize2, PenLine, Plus, Scan } from 'lucide-react'
import {
  createVersion,
  deleteVersion,
  getVersion,
  listVersions,
  setCurrentVersion,
  type VersionRole,
} from '@/lib/api'
import { clearDraft, readDraft, writeDraft } from '@/lib/drafts'
import { predecessor } from '@/lib/history'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { announceDeleted } from '@/lib/trash'
import { useBodyEditing } from '@/lib/useBodyEditing'
import { labelOf, useVocabulary } from '@/lib/useProfile'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Markdown } from '@/components/ui/Markdown'
import { SaveState } from '@/components/ui/SaveState'
import { Select } from '@/components/ui/AppSelect'
import { Skeleton } from '@/components/ui/Skeleton'
import { VersionDiff } from '@/components/versions/VersionDiff'
import { VersionEditor } from '@/components/versions/VersionEditor'
import { VersionList } from '@/components/versions/VersionList'

interface Props {
  workId: string
}

/**
 * How the open version is shown.
 *
 * `view` draws the body the way its role reads — a monospace column for
 * lyrics, rendered markdown for a review. `edit` is the same text in a box,
 * saving itself into the version of the sitting. `changes` compares with the
 * revision before it — the question asked most often of a history, and one
 * that should not cost a hunt through the list for the other side. Picking a
 * version with the ± button overrides it, because then the comparison was
 * asked for explicitly.
 */
type Reading = 'view' | 'edit' | 'changes'

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
  const [commentReading, setCommentReading] = useState<Exclude<Reading, 'changes'>>('view')
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
  // 5 would be the panel asserting something nobody wrote.
  const comments = (versions.data ?? []).filter(
    (version) =>
      openSummary !== null &&
      commentRoles.some((r) => r.key === version.role) &&
      version.revision === openSummary.revision,
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

  // What `changes` compares against when nothing was picked by hand.
  const before = predecessor(summaries, openId)
  const againstId = comparedId ?? (reading === 'changes' ? (before?.id ?? null) : null)

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

  const enterText = () => {
    setReading('edit')
    setComparedId(null)
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
        reading={comparing ? 'changes' : reading}
        onReading={(mode) => {
          setReading(mode)
          // Leaving the diff drops the hand-picked other side as well, or the
          // text would stay hidden behind it.
          if (mode !== 'changes') setComparedId(null)
        }}
        withChanges
        changes={
          comparing && compared.data != null ? (
            <VersionDiff
              before={compared.data.body}
              after={open.data.body}
              beforeLabel={nameOf(againstId)}
              afterLabel={nameOf(openId)}
            />
          ) : (
            <p className="text-sm text-dim">{t('versions.diffFirst')}</p>
          )
        }
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
                  'cursor-pointer rounded-[7px] px-2 py-0.5 transition-colors',
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
        onReading={(mode) => {
          if (mode !== 'changes') setCommentReading(mode)
        }}
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

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold">{t('versions.title')}</h3>
        {roles.length > 1 && (
          <Select
            className="w-40"
            aria-label={t('versions.role')}
            value={shownRole}
            onChange={(next) => {
              setRole(next)
              // Picking a lane by hand ends the link's claim on the panel.
              if (asked !== null) setParams({}, { replace: true })
              // The selection and the comparison belonged to the role being left.
              setSelectedId(null)
              setComparedId(null)
              setReading('view')
            }}
            options={roles
              // Commentary is not a lane: it belongs beside what it comments
              // on, and offering it here would show it stripped of that.
              .filter((r) => r.comments_on === undefined)
              .map((r) => ({ value: r.key, label: r.label }))}
          />
        )}
        <Button
          className="ml-auto"
          variant="ghost"
          size="sm"
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

      {/* The list only takes a column of its own once there is room for the
          text beside it. Below that it sits above, full width, because a
          comparison squeezed into 130px is two columns of hyphens. */}
      <div className="grid gap-4 xl:grid-cols-[15rem_minmax(0,1fr)]">
        <VersionList
          versions={summaries}
          loading={versions.isPending}
          openId={openId}
          comparedId={comparedId}
          onOpen={(id) => {
            setSelectedId(id)
            // Opening another version is reading it, whatever the last one
            // was being done to.
            setReading('view')
            // A hand-picked other side belonged to the version it was picked
            // against. Carrying it onto the next one turns stepping through a
            // history into comparing everything with one fixed revision, which
            // is not what the step asked for; `changes` falls back to each
            // version's own predecessor.
            setComparedId(null)
          }}
          onCompare={(id) => {
            const dropping = comparedId === id
            setComparedId(dropping ? null : id)
            // Picking a side to compare against is asking for the diff; letting
            // it go returns to whatever was being read before.
            setReading(dropping ? 'view' : 'changes')
          }}
          onMakeCurrent={(id) => makeCurrent.mutate(id)}
          onDelete={(id) => remove.mutate(id)}
          onDeriveFrom={(id) => void deriveFrom(id)}
        />

        <div className="flex min-w-0 flex-col gap-4">
          {open.isPending && openId !== null && <Skeleton className="h-32 w-full" />}

          {/* The text and what was written about it, side by side. Reading a
              review away from the lines it discusses is reading half of it —
              which is exactly what the predecessor's two panels got right. The
              split only happens when there is a review of this very revision
              and room for both; below that the review sits underneath. */}
          {open.data != null && (
            <div className={cn('grid min-w-0 gap-4', comments.length > 0 && '2xl:grid-cols-2')}>
              {stage?.pane === 'text' ? <StagePlaceholder /> : textPane('inline')}
              {comments.length > 0 &&
                (stage?.pane === 'comment' ? <StagePlaceholder /> : commentPane('inline'))}
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
  /** Whether the pane offers the comparison mode at all. */
  withChanges?: boolean
  changes?: ReactNode
  editing: ReturnType<typeof useBodyEditing>
  /** Clicking into the text: edit, on the whole content area. */
  onEnter: () => void
  onLevel: (level: Level) => void
}

/**
 * One body: a version's text, read, edited or compared — on the card, over
 * the content area, or over the whole window.
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
  withChanges = false,
  changes,
  editing,
  onEnter,
  onLevel,
}: PaneProps) {
  const { t } = useTranslation()
  const staged = level !== 'inline'

  const modes: { mode: Reading; icon: typeof Eye; label: string }[] = [
    { mode: 'view', icon: Eye, label: t('versions.view') },
    { mode: 'edit', icon: PenLine, label: t('versions.edit') },
    ...(withChanges ? [{ mode: 'changes' as const, icon: Diff, label: t('versions.changes') }] : []),
  ]

  const content =
    body === null ? (
      <Skeleton className="h-32 w-full" />
    ) : reading === 'changes' ? (
      <div className="px-3 py-2.5">{changes}</div>
    ) : reading === 'edit' ? (
      <textarea
        autoFocus
        value={editing.text}
        onChange={(event) => editing.setText(event.target.value)}
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
        className={cn(
          'selectable block w-full resize-none bg-transparent px-3 py-2.5 text-sm leading-relaxed text-text outline-none',
          !markdown && 'font-mono',
          staged ? 'h-full min-h-0 flex-1' : 'min-h-[28rem]',
        )}
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
          'cursor-text px-3 py-2.5 outline-none focus-visible:ring-2 focus-visible:ring-accent',
          staged ? 'min-h-full' : 'max-h-[28rem] overflow-auto',
        )}
      >
        {markdown ? (
          <Markdown body={body} />
        ) : (
          <pre className="selectable whitespace-pre-wrap font-mono text-sm">{body}</pre>
        )}
      </div>
    )

  const frame = (
    <article
      className={cn(
        'flex min-w-0 flex-col overflow-hidden rounded-xl border border-line bg-bg',
        staged && 'h-full',
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

      <div className={cn('flex min-h-0 flex-col', staged ? 'flex-1 overflow-auto' : '')}>
        {content}
      </div>
    </article>
  )

  if (level === 'inline') return frame

  return (
    <div
      className={cn(
        'z-40 flex flex-col gap-2 bg-bg p-6',
        level === 'focus' ? 'fixed inset-0 z-50' : 'absolute inset-0',
      )}
    >
      <p className="text-xs text-faint">{t('versions.stageHint')}</p>
      <div className="flex min-h-0 flex-1 flex-col">{frame}</div>
    </div>
  )
}
