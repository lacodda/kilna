import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { open as openFiles } from '@tauri-apps/plugin-dialog'
import {
  BookPlus,
  Copy,
  Ellipsis,
  FilePlus2,
  Flag,
  GitBranch,
  Lightbulb,
  Sparkles,
  Trash2,
  TriangleAlert,
  Wrench,
} from 'lucide-react'
import { attachAsset } from '@/lib/api/assets'
import { judgeTrial, updateTrial } from '@/lib/api/trials'
import type { Composition, TrialBoard, TrialCard, TrialPatch, TrialVerdict } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { useTextCheck } from '@/lib/useTextCheck'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { MarkedTextarea, type Mark } from '@/components/ui/marked-text'
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { InlineField } from '@/components/ui/inline-field'
import { Textarea } from '@/components/ui/textarea'
import { Pane } from '@/components/frame'
import { AudioPlayer } from '@/components/AudioPlayer'
import { PhraseStrip } from '@/features/work/tabs/versions/PhraseStrip'
import { PHRASE_MARK } from '@/lib/phrases'
import type { LabRun } from '@/features/work/tabs/trials/useLabTask'

interface Props {
  board: TrialBoard
  card: TrialCard
  /** The composition a trial is read and written by, when the lab has one. */
  composition: Composition | null
  /** A run writing a variation or the fix of this trial, if one is going. */
  run: LabRun | undefined
  /** Whether the profile has an action that proposes trials. */
  assisted: boolean
  onVary: () => void
  onAround: () => void
  onFix: () => void
  onStop: (key: string) => void
  onDelete: () => void
  onIntoWork: () => void
  onIntoDictionary: (phrase: string) => void
  onMakeWork: () => void
}

/**
 * The open trial (v0.95, ADR 0061): what it moves, its text read against the
 * dictionary as it is written, who to listen to, the takes it was heard by,
 * what came out, the verdict - and, once kept, where it goes.
 *
 * Everything is saved as it is left: the text and what came out a moment
 * after the typing stops, the short fields when the box is left.
 */
export function TrialDetail({
  board,
  card,
  composition,
  run,
  assisted,
  onVary,
  onAround,
  onFix,
  onStop,
  onDelete,
  onIntoWork,
  onIntoDictionary,
  onMakeWork,
}: Props) {
  const { t } = useTranslation()
  const { trial } = card
  const refresh = [keys.trialBoard(trial.work_id)]

  const patch = useAppMutation({
    mutationFn: (changes: TrialPatch) => updateTrial(trial.id, changes),
    failure: 'trials.saveFailed',
    refresh,
  })
  const judge = useAppMutation({
    mutationFn: (verdict: TrialVerdict | null) => judgeTrial(trial.id, verdict),
    failure: 'trials.judgeFailed',
    refresh,
  })
  const takes = useAppMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) await attachAsset(path, { trial_id: trial.id })
    },
    failure: 'trials.takeFailed',
    refresh,
  })

  const addTakes = async () => {
    const chosen = await openFiles({
      multiple: true,
      title: t('trials.addTakes'),
      filters: [
        { name: t('trials.sounds'), extensions: ['mp3', 'wav', 'flac', 'ogg', 'm4a', 'aac'] },
      ],
    })
    const paths = chosen === null ? [] : Array.isArray(chosen) ? chosen : [chosen]
    if (paths.length > 0) takes.mutate(paths.filter((path) => typeof path === 'string'))
  }

  const copy = () => {
    navigator.clipboard.writeText(trial.body).then(
      () => say.ok(t('trials.copied')),
      (cause: unknown) => say.failedTo(t('trials.copyFailed'), cause),
    )
  }

  const kept = trial.verdict === 'keep'
  const harvestable = kept && board.harvest_role !== null && board.harvest_kinds.length > 0

  return (
    <Pane
      label={t('trials.open')}
      bodyClassName="flex flex-col gap-3 px-3.5 py-3"
      head={
        <>
          <InlineField
            label={t('trials.angle')}
            labelHidden
            placeholder={t('trials.anglePlaceholder')}
            className="min-w-40 flex-1"
            value={trial.angle}
            onCommit={(angle) => patch.mutate({ angle: angle ?? '' })}
          />
          <SegmentedControl
            aria-label={t('trials.verdict.label')}
            value={trial.verdict ?? 'open'}
            onValueChange={(next) => judge.mutate(next === 'open' ? null : (next as TrialVerdict))}
          >
            <Segment value="keep">{t('trials.verdict.keep')}</Segment>
            <Segment value="open">{t('trials.verdict.open')}</Segment>
            <Segment value="drop">{t('trials.verdict.drop')}</Segment>
          </SegmentedControl>
          <Button
            size="icon-sm"
            variant={trial.run_first ? 'soft' : 'ghost'}
            aria-pressed={trial.run_first}
            aria-label={t('trials.runFirst')}
            title={t('trials.runFirstHint')}
            onClick={() => patch.mutate({ run_first: !trial.run_first })}
          >
            <Flag aria-hidden className={cn(trial.run_first && 'text-accent')} />
          </Button>
          <Button size="sm" variant="soft" onClick={copy} title={t('trials.copyHint')}>
            <Copy aria-hidden />
            {t('trials.copy')}
          </Button>
          <Menu>
            <MenuTrigger
              render={<Button size="icon-sm" variant="ghost" aria-label={t('trials.more')} />}
            >
              <Ellipsis aria-hidden />
            </MenuTrigger>
            <MenuPopup align="end">
              <MenuItem onClick={onVary}>
                <GitBranch aria-hidden className="size-3.5" />
                {t('trials.vary')}
              </MenuItem>
              {assisted && (
                <MenuItem onClick={onAround} disabled={run !== undefined}>
                  <Sparkles aria-hidden className="size-3.5" />
                  {t('trials.around')}
                </MenuItem>
              )}
              {assisted && (
                <MenuItem
                  onClick={onFix}
                  disabled={run !== undefined || trial.outcome.trim() === ''}
                >
                  <Wrench aria-hidden className="size-3.5" />
                  {t('trials.fix')}
                </MenuItem>
              )}
              <MenuSeparator />
              <MenuItem onClick={onDelete}>
                <Trash2 aria-hidden className="size-3.5" />
                {t('trials.delete')}
              </MenuItem>
            </MenuPopup>
          </Menu>
        </>
      }
    >
      {run !== undefined && (
        <p className="flex items-center gap-2 text-xs text-dim" role="status">
          <span aria-hidden className="size-2 animate-pulse rounded-full bg-accent" />
          {run.key.includes(':fix:') ? t('trials.fixing') : t('trials.varying')}
          <Button size="xs" variant="ghost" onClick={() => onStop(run.key)}>
            {t('trials.stop')}
          </Button>
        </p>
      )}

      {card.lost_anchors.length > 0 && (
        <p className="flex items-start gap-2 rounded-md bg-warn-soft px-2.5 py-1.5 text-xs text-warn">
          <TriangleAlert aria-hidden className="mt-0.5 size-3.5 shrink-0" />
          <span>
            {t('trials.lost')}{' '}
            {card.lost_anchors.map((anchor) => (
              <code key={anchor} className="mr-1.5 font-mono">
                {anchor}
              </code>
            ))}
          </span>
        </p>
      )}

      <TrialText
        key={trial.id}
        workId={trial.work_id}
        role={board.harvest_role}
        body={trial.body}
        composition={composition}
        onSave={(body) => patch.mutate({ body })}
      />

      <InlineField
        label={t('trials.reference')}
        placeholder={t('trials.referencePlaceholder')}
        value={trial.reference}
        onCommit={(reference) => patch.mutate({ reference: reference ?? '' })}
      />
      {card.source !== null && (
        <p className="text-xs text-dim">
          {t('trials.reworks')}{' '}
          <Link to={`/works/${card.source.work_id}/versions?version=${card.source.version_id}`}>
            {card.source.title} · r{card.source.revision}
            {card.source.label !== null && ` · ${card.source.label}`}
          </Link>
        </p>
      )}

      <section aria-label={t('trials.takesTitle')} className="flex flex-col gap-1.5">
        <div className="flex items-center gap-2">
          <span className="caption">{t('trials.takesTitle')}</span>
          <Button
            size="xs"
            variant="ghost"
            disabled={takes.isPending}
            onClick={() => void addTakes()}
          >
            <FilePlus2 aria-hidden />
            {t('trials.addTakes')}
          </Button>
        </div>
        {card.takes.length > 0 ? (
          <Takes card={card} />
        ) : (
          <p className="text-xs text-faint">{t('trials.noTakes')}</p>
        )}
      </section>

      <Outcome
        key={`outcome-${trial.id}`}
        outcome={trial.outcome}
        onSave={(outcome) => patch.mutate({ outcome })}
      />

      {kept && (
        <section aria-label={t('trials.harvestTitle')} className="flex flex-col gap-1.5">
          <span className="caption">{t('trials.harvestTitle')}</span>
          <div className="flex flex-wrap gap-1.5">
            {harvestable && (
              <Button size="sm" variant="primary" onClick={onIntoWork}>
                <Lightbulb aria-hidden />
                {t('trials.intoWork')}
              </Button>
            )}
            {composition !== null && (
              <Button size="sm" variant="soft" onClick={() => onIntoDictionary(trial.body)}>
                <BookPlus aria-hidden />
                {t('trials.intoDictionary')}
              </Button>
            )}
            {harvestable && (
              <Button size="sm" variant="ghost" onClick={onMakeWork}>
                {t('trials.makeWork')}
              </Button>
            )}
          </div>
          {card.harvest.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-xs">
              {card.harvest.map((went) =>
                went.kind === 'version' ? (
                  <li key={went.version_id}>
                    →{' '}
                    <Link to={`/works/${went.work_id}/versions?version=${went.version_id}`}>
                      {went.title} · r{went.revision}
                      {went.label !== null && ` · ${went.label}`}
                    </Link>
                  </li>
                ) : (
                  <li key={went.brick_id}>
                    → <Link to={`/styles?brick=${went.brick_id}`}>{went.name}</Link>
                  </li>
                ),
              )}
            </ul>
          )}
        </section>
      )}
    </Pane>
  )
}

/** How long the typing has to stop before a text is saved. */
const SETTLE = 700

/**
 * The text of a trial, read against the dictionary as it is typed (v0.94's
 * reading, ADR 0060): a phrase it knows marked, a tag it does not dotted, and
 * under the text what each known phrase means and the unknown ones one press
 * from the dictionary. Saved a moment after the typing stops, and when left.
 */
function TrialText({
  workId,
  role,
  body,
  composition,
  onSave,
}: {
  workId: string
  role: string | null
  body: string
  composition: Composition | null
  onSave: (body: string) => void
}) {
  const { t } = useTranslation()
  const [text, setText] = useState(body)
  const saved = useRef(body)
  const focused = useRef(false)

  // The stored text moved under a box nobody is typing in - a variation
  // written by the assistant, an undo: the box takes it.
  useEffect(() => {
    if (!focused.current && body !== saved.current) {
      saved.current = body
      setText(body)
    }
  }, [body])

  useEffect(() => {
    if (text === saved.current) return undefined
    const timer = window.setTimeout(() => {
      saved.current = text
      onSave(text)
    }, SETTLE)
    return () => window.clearTimeout(timer)
  }, [text, onSave])

  const { check, current } = useTextCheck(
    composition === null || role === null ? null : text,
    false,
    role === null ? null : { workId, role },
  )
  const marks: Mark[] =
    check === undefined || !current
      ? []
      : check.marks.flatMap((mark) => {
          if (mark.phrase !== undefined) {
            return [
              {
                start: mark.start,
                end: mark.end,
                className: `${PHRASE_MARK} phrase-${String(mark.phrase)}`,
              },
            ]
          }
          if (mark.unknown !== undefined) {
            return [{ start: mark.start, end: mark.end, className: 'phrase-unknown' }]
          }
          return []
        })

  return (
    <div className="selectable flex flex-col overflow-hidden rounded-md border border-line bg-bg">
      <MarkedTextarea
        aria-label={t('trials.body')}
        placeholder={t('trials.bodyPlaceholder')}
        value={text}
        onChange={setText}
        onFocus={() => {
          focused.current = true
        }}
        onBlur={() => {
          focused.current = false
          if (text !== saved.current) {
            saved.current = text
            onSave(text)
          }
        }}
        marks={marks}
        className="block min-h-16 w-full px-3 py-2 font-mono text-sm leading-relaxed whitespace-pre-wrap break-words"
      />
      {composition !== null && check !== undefined && (
        <PhraseStrip check={check} composition={composition} />
      )}
    </div>
  )
}

/** What came out once the trial was heard, saved as it is written. */
function Outcome({ outcome, onSave }: { outcome: string; onSave: (outcome: string) => void }) {
  const { t } = useTranslation()
  const [text, setText] = useState(outcome)
  const saved = useRef(outcome)
  useEffect(() => {
    if (text === saved.current) return undefined
    const timer = window.setTimeout(() => {
      saved.current = text
      onSave(text)
    }, SETTLE)
    return () => window.clearTimeout(timer)
  }, [text, onSave])
  return (
    <label className="flex flex-col gap-1">
      <span className="caption">{t('trials.outcome')}</span>
      <Textarea
        rows={2}
        autoResize
        maxRows={10}
        value={text}
        placeholder={t('trials.outcomePlaceholder')}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== saved.current) {
            saved.current = text
            onSave(text)
          }
        }}
      />
    </label>
  )
}

/** The takes of a trial, one player for all of them. */
function Takes({ card }: { card: TrialCard }) {
  const { t } = useTranslation()
  const tracks = card.takes.map((take, index) => ({
    path: take.path,
    name: take.label ?? take.original_name ?? t('trials.take', { n: index + 1 }),
    where: t('trials.attached'),
  }))
  const [selected, setSelected] = useState(tracks[0]?.path ?? '')
  return <AudioPlayer tracks={tracks} selected={selected} onSelect={setSelected} request={0} />
}
