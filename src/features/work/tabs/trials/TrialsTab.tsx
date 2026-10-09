import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { BookOpen, Eye, FilePlus2, Music, Plus, Sparkles } from 'lucide-react'
import { applyProposal, dismissProposal } from '@/lib/api/assistant'
import {
  createTrial,
  deleteTrial,
  harvestTrial,
  harvestTrialWork,
  trialFromVersion,
  varyTrial,
} from '@/lib/api/trials'
import type { NewTrial, ScoredWork, TrialBoard, TrialRequest, Work } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import {
  MOST_TRIALS,
  TRIAL_FILTERS,
  TRIALS_AROUND,
  TRIALS_ON_SWEEP,
  around,
  counts,
  fix,
  opening,
  seriesOf,
  sweep,
  type TrialFilter,
} from '@/lib/trials'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Chip, ChipGroup } from '@/components/ui/chip'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { RowButton } from '@/components/ui/list-row'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { SkeletonList } from '@/components/ui/skeleton'
import { Dialog } from '@/components/AppDialog'
import { Frame, ListDetail, Pane } from '@/components/frame'
import { Loaded } from '@/components/Loaded'
import { PickWorkDialog } from '@/components/PickWorkDialog'
import { TaskPreviewDialog } from '@/features/assistant/TaskPreviewDialog'
import { AddPhraseDialog } from '@/features/styles/AddPhraseDialog'
import { ComposeDialog } from '@/features/styles/ComposeDialog'
import { TrialDetail } from '@/features/work/tabs/trials/TrialDetail'
import { TrialList } from '@/features/work/tabs/trials/TrialList'
import { useLabTask } from '@/features/work/tabs/trials/useLabTask'

/** How many a sweep of the field may ask for, as the composer offers them. */
const SWEEP_COUNTS = [3, 6, 9, MOST_TRIALS] as const

/**
 * An experiment's board of trials (v0.95, ADR 0061): its trials by series,
 * each a tree, beside the one that is open - written, heard, judged, varied,
 * and once kept, taken into a song or the dictionary.
 */
export function TrialsTab({ work }: { work: Work }) {
  const board = useQuery(queries.trialBoard(work.id))
  return (
    <Frame>
      <Loaded query={board} skeleton={<SkeletonList rows={4} />} plain fill>
        {(data) => <Board work={work} board={data} />}
      </Loaded>
    </Frame>
  )
}

type Dialogs =
  | { kind: 'compose' }
  | { kind: 'fromSong' }
  | { kind: 'version'; song: ScoredWork }
  | { kind: 'vary'; trialId: string }
  | { kind: 'intoWork'; trialId: string }
  | { kind: 'phrase'; trialId: string; phrase: string }
  | { kind: 'makeWork'; trialId: string }
  | { kind: 'preview'; request: TrialRequest }

function Board({ work, board }: { work: Work; board: TrialBoard }) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const navigate = useNavigate()
  const lab = useLabTask(work)
  const [filter, setFilter] = useState<TrialFilter>('all')
  const [chosen, setChosen] = useState<string | null>(null)
  const [count, setCount] = useState<number>(TRIALS_ON_SWEEP)
  const [dialog, setDialog] = useState<Dialogs | null>(null)
  const refresh = [keys.trialBoard(work.id)]

  const composition = useMemo(
    () => (config.compose ?? []).find((one) => one.key === board.composition) ?? null,
    [config.compose, board.composition],
  )
  const tally = counts(board.trials)
  const groups = seriesOf(board, filter)
  const open =
    board.trials.find((card) => card.trial.id === chosen) ?? groups[0]?.rows[0]?.card ?? undefined
  // Where a trial made from the composer goes: beside the one open, in a
  // board that has none yet under the first words a sweep would use.
  const seriesHere = open?.trial.series ?? board.series.at(-1) ?? t('trials.series.first')

  const create = useAppMutation({
    mutationFn: (trial: NewTrial) => createTrial(trial),
    failure: 'trials.createFailed',
    refresh,
    onSuccess: (made) => setChosen(made.id),
  })
  const vary = useAppMutation({
    mutationFn: ({ id, angle }: { id: string; angle: string }) => varyTrial(id, angle),
    failure: 'trials.varyFailed',
    refresh,
    onSuccess: (made) => setChosen(made.id),
  })
  const fromVersion = useAppMutation({
    mutationFn: ({ versionId, title }: { versionId: string; title: string }) =>
      trialFromVersion(work.id, versionId, t('trials.series.rework', { title })),
    failure: 'trials.createFailed',
    refresh,
    onSuccess: (made) => setChosen(made.id),
  })
  const remove = useAppMutation({
    mutationFn: (id: string) => deleteTrial(id),
    failure: 'trials.deleteFailed',
    refresh: [...refresh, keys.deletions],
    onSuccess: () => setChosen(null),
  })
  const intoWork = useAppMutation({
    mutationFn: ({ id, workId }: { id: string; workId: string }) => harvestTrial(id, workId),
    failure: 'trials.harvestFailed',
    refresh: [...refresh, keys.works, keys.catalogue],
    onSuccess: () => say.ok(t('trials.harvestedInto')),
  })
  const makeWork = useAppMutation({
    mutationFn: ({ id, kind, title }: { id: string; kind: string; title: string }) =>
      harvestTrialWork(id, kind, title),
    failure: 'trials.harvestFailed',
    refresh: [...refresh, keys.works, keys.catalogue],
    onSuccess: (made) => void navigate(`/works/${made.id}`),
  })

  const ask = (request: TrialRequest) => lab.ask(request)
  const sweepRequest = sweep(count, t('trials.series.sweep'))
  const writing = lab.runs.length > 0
  const coming = lab.runs
    .filter((run) => run.around === null)
    .reduce((sum, run) => sum + run.coming, 0)

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5">
      <AgentTrials workId={work.id} />
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <Menu>
          <MenuTrigger render={<Button size="sm" variant="soft" />}>
            <Plus aria-hidden />
            {t('trials.new')}
          </MenuTrigger>
          <MenuPopup align="start">
            <MenuItem onClick={() => create.mutate({ work_id: work.id, series: seriesHere })}>
              <FilePlus2 aria-hidden className="size-3.5" />
              {t('trials.newEmpty')}
            </MenuItem>
            {composition !== null && (
              <MenuItem onClick={() => setDialog({ kind: 'compose' })}>
                <BookOpen aria-hidden className="size-3.5" />
                {t('trials.newFromDictionary')}
              </MenuItem>
            )}
            {open !== undefined && (
              <MenuItem onClick={() => setDialog({ kind: 'vary', trialId: open.trial.id })}>
                <Sparkles aria-hidden className="size-3.5" />
                {t('trials.newVariation')}
              </MenuItem>
            )}
            {board.harvest_kinds.length > 0 && (
              <MenuItem onClick={() => setDialog({ kind: 'fromSong' })}>
                <Music aria-hidden className="size-3.5" />
                {t('trials.newFromSong')}
              </MenuItem>
            )}
          </MenuPopup>
        </Menu>
        {lab.action !== undefined && (
          <>
            <span className="caption ml-2">{t('trials.sweep')}</span>
            <SegmentedControl
              aria-label={t('trials.sweepCount')}
              value={String(count)}
              onValueChange={(next) => setCount(Number(next))}
            >
              {SWEEP_COUNTS.map((n) => (
                <Segment key={n} value={String(n)}>
                  {n}
                </Segment>
              ))}
            </SegmentedControl>
            <Button
              size="sm"
              variant="primary"
              disabled={lab.asking}
              title={t('trials.sweepHint')}
              onClick={() => ask(sweepRequest)}
            >
              <Sparkles aria-hidden />
              {t('trials.sweepGo')}
            </Button>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t('trials.preview')}
              title={t('trials.preview')}
              onClick={() => setDialog({ kind: 'preview', request: sweepRequest })}
            >
              <Eye aria-hidden />
            </Button>
          </>
        )}
        <ChipGroup
          className="ml-auto"
          aria-label={t('trials.filter')}
          value={[filter]}
          onValueChange={([picked]) => {
            if (picked !== undefined) setFilter(picked as TrialFilter)
          }}
        >
          {TRIAL_FILTERS.map((one) => (
            <Chip key={one} value={one}>
              {t(`trials.filters.${one}`)}
              <span className="font-mono text-2xs">{tally[one]}</span>
            </Chip>
          ))}
        </ChipGroup>
      </div>

      {board.anchors.length > 0 && (
        <p className="shrink-0 text-xs text-dim">
          {t('trials.anchors')}{' '}
          {board.anchors.map((anchor) => (
            <code key={anchor} className="mr-1.5 rounded-sm bg-soft px-1 font-mono">
              {anchor}
            </code>
          ))}
        </p>
      )}

      {writing && (
        <p className="flex shrink-0 items-center gap-2 text-xs text-dim" role="status">
          <span aria-hidden className="size-2 animate-pulse rounded-full bg-accent" />
          {t('trials.writing', { count: lab.runs.length })}
          {lab.runs
            .filter((run) => run.around === null)
            .map((run) => (
              <Button key={run.key} size="xs" variant="ghost" onClick={() => lab.stop(run.key)}>
                {t('trials.stop')}
              </Button>
            ))}
        </p>
      )}

      {board.trials.length === 0 && !writing ? (
        <EmptyBoard
          onEmpty={() => create.mutate({ work_id: work.id, series: seriesHere })}
          onSweep={lab.action === undefined ? undefined : () => ask(sweepRequest)}
        />
      ) : (
        <ListDetail
          list={
            <Pane label={t('trials.list')}>
              <TrialList
                series={groups}
                selected={open?.trial.id ?? null}
                onSelect={setChosen}
                fresh={lab.fresh}
                coming={coming}
              />
              {groups.length === 0 && (
                <p className="px-3 py-2 text-xs text-dim">{t('trials.emptyFilter')}</p>
              )}
            </Pane>
          }
          detail={
            open === undefined ? (
              <p className="p-4 text-sm text-dim">{t('trials.pick')}</p>
            ) : (
              <TrialDetail
                key={open.trial.id}
                board={board}
                card={open}
                composition={composition}
                run={lab.runs.find((run) => run.around === open.trial.id)}
                assisted={lab.action !== undefined}
                onVary={() => setDialog({ kind: 'vary', trialId: open.trial.id })}
                onAround={() =>
                  ask(
                    around(
                      open.trial.id,
                      TRIALS_AROUND,
                      t('trials.series.around', {
                        name: open.trial.angle.trim() || opening(open.trial.body, 32),
                      }),
                    ),
                  )
                }
                onFix={() => ask(fix(open.trial.id, open.trial.series))}
                onStop={lab.stop}
                onDelete={() => remove.mutate(open.trial.id)}
                onIntoWork={() => setDialog({ kind: 'intoWork', trialId: open.trial.id })}
                onIntoDictionary={(phrase) =>
                  setDialog({ kind: 'phrase', trialId: open.trial.id, phrase })
                }
                onMakeWork={() => setDialog({ kind: 'makeWork', trialId: open.trial.id })}
              />
            )
          }
        />
      )}

      {dialog?.kind === 'compose' && composition !== null && (
        <ComposeDialog
          open
          onOpenChange={(next) => {
            if (!next) setDialog(null)
          }}
          composition={composition}
          workId={work.id}
          useLabel={t('trials.putOnBoard')}
          onUse={(text, bricks) => {
            create.mutate({ work_id: work.id, series: seriesHere, body: text, bricks })
            setDialog(null)
          }}
        />
      )}
      {dialog?.kind === 'vary' && (
        <VaryDialog
          onClose={() => setDialog(null)}
          onVary={(angle) => {
            vary.mutate({ id: dialog.trialId, angle })
            setDialog(null)
          }}
        />
      )}
      <PickWorkDialog
        open={dialog?.kind === 'fromSong'}
        onOpenChange={(next) => {
          if (!next) setDialog(null)
        }}
        title={t('trials.pickSong')}
        kinds={board.harvest_kinds}
        onPick={(song) => setDialog({ kind: 'version', song })}
      />
      {dialog?.kind === 'version' && board.harvest_role !== null && (
        <VersionDialog
          song={dialog.song}
          role={board.harvest_role}
          onClose={() => setDialog(null)}
          onPick={(versionId) => {
            fromVersion.mutate({ versionId, title: dialog.song.title })
            setDialog(null)
          }}
        />
      )}
      <PickWorkDialog
        open={dialog?.kind === 'intoWork'}
        onOpenChange={(next) => {
          if (!next) setDialog(null)
        }}
        title={t('trials.intoWorkTitle')}
        kinds={board.harvest_kinds}
        onPick={(target) => {
          if (dialog?.kind === 'intoWork')
            intoWork.mutate({ id: dialog.trialId, workId: target.work_id })
        }}
      />
      {dialog?.kind === 'phrase' && composition !== null && (
        <AddPhraseDialog
          open
          onOpenChange={(next) => {
            if (!next) setDialog(null)
          }}
          composition={composition}
          phrase={dialog.phrase}
          trialId={dialog.trialId}
        />
      )}
      {dialog?.kind === 'makeWork' && (
        <MakeWorkDialog
          kinds={board.harvest_kinds}
          labelOf={(kind) =>
            sayLabel(config.work_kinds.find((one) => one.key === kind)?.label ?? kind)
          }
          onClose={() => setDialog(null)}
          onMake={(kind, title) => {
            makeWork.mutate({ id: dialog.trialId, kind, title })
            setDialog(null)
          }}
        />
      )}
      {dialog?.kind === 'preview' && lab.action !== undefined && (
        <TaskPreviewDialog
          open
          onOpenChange={(next) => {
            if (!next) setDialog(null)
          }}
          target={{ on: 'lab', workId: work.id, request: dialog.request }}
          action={lab.action}
          onStarted={() => {
            lab.started(dialog.request)
            setDialog(null)
          }}
        />
      )}
    </div>
  )
}

/** A board with nothing on it: where to start. */
function EmptyBoard({ onEmpty, onSweep }: { onEmpty: () => void; onSweep?: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col items-start gap-2 rounded-lg border border-dashed border-line-2 bg-softer px-4 py-3.5 text-sm text-dim">
      <p>{t('trials.emptyBoard')}</p>
      <div className="flex gap-1.5">
        {onSweep !== undefined && (
          <Button size="sm" variant="primary" onClick={onSweep}>
            <Sparkles aria-hidden />
            {t('trials.sweepGo')}
          </Button>
        )}
        <Button size="sm" variant="soft" onClick={onEmpty}>
          {t('trials.newEmpty')}
        </Button>
      </div>
    </div>
  )
}

/** What a variation moves, asked before it is made. */
function VaryDialog({ onClose, onVary }: { onClose: () => void; onVary: (angle: string) => void }) {
  const { t } = useTranslation()
  const [angle, setAngle] = useState('')
  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
      title={t('trials.varyTitle')}
      description={t('trials.varyBody')}
      footer={
        <Button variant="primary" onClick={() => onVary(angle.trim())}>
          {t('trials.vary')}
        </Button>
      }
    >
      <Field label={t('trials.angle')}>
        <Input
          autoFocus
          value={angle}
          placeholder={t('trials.varyPlaceholder')}
          onChange={(event) => setAngle(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') onVary(angle.trim())
          }}
        />
      </Field>
    </Dialog>
  )
}

/** Which text of a song a rework starts from: its versions in the role the
 *  lab keeps trials in, the newest first. */
function VersionDialog({
  song,
  role,
  onClose,
  onPick,
}: {
  song: ScoredWork
  role: string
  onClose: () => void
  onPick: (versionId: string) => void
}) {
  const { t } = useTranslation()
  const versions = useQuery(queries.versions(song.work_id))
  const ofRole = (versions.data ?? []).filter((version) => version.role === role)
  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
      title={t('trials.versionTitle', { title: song.title })}
      description={t('trials.versionBody')}
    >
      {versions.isPending ? (
        <SkeletonList rows={3} />
      ) : ofRole.length === 0 ? (
        <p className="text-sm text-dim">{t('trials.noVersions')}</p>
      ) : (
        <ul className="flex flex-col">
          {ofRole.map((version) => (
            <li key={version.id}>
              <RowButton
                onClick={() => onPick(version.id)}
                end={version.is_current ? t('trials.current') : undefined}
              >
                r{version.revision}
                {version.label !== null && ` · ${version.label}`}
              </RowButton>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  )
}

/** A new work of a kept trial: its title and kind. */
function MakeWorkDialog({
  kinds,
  labelOf,
  onClose,
  onMake,
}: {
  kinds: string[]
  labelOf: (kind: string) => string
  onClose: () => void
  onMake: (kind: string, title: string) => void
}) {
  const { t } = useTranslation()
  const [title, setTitle] = useState('')
  const [kind, setKind] = useState(kinds[0] ?? '')
  return (
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
      title={t('trials.makeWorkTitle')}
      description={t('trials.makeWorkBody')}
      footer={
        <Button
          variant="primary"
          disabled={title.trim() === '' || kind === ''}
          onClick={() => onMake(kind, title.trim())}
        >
          {t('trials.makeWork')}
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <Field label={t('trials.workTitle')}>
          <Input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} />
        </Field>
        {kinds.length > 1 && (
          <ChipGroup value={[kind]} onValueChange={([next]) => next !== undefined && setKind(next)}>
            {kinds.map((one) => (
              <Chip key={one} value={one}>
                {labelOf(one)}
              </Chip>
            ))}
          </ChipGroup>
        )}
      </div>
    </Dialog>
  )
}

/**
 * Trials an agent proposed for this board from outside the window: they wait
 * here until the person puts them on the board or turns them down - an agent
 * only ever proposes (ADR 0018).
 */
function AgentTrials({ workId }: { workId: string }) {
  const { t } = useTranslation()
  const pending = useQuery({
    ...queries.pendingProposals(),
    staleTime: 0,
    refetchInterval: 30_000,
  })
  const waiting = (pending.data ?? []).filter(
    (one) => one.kind === 'trials' && one.work_id === workId,
  )
  const refresh = [keys.trialBoard(workId), keys.pendingProposals, keys.transcripts]
  const put = useAppMutation({
    mutationFn: (messageId: string) => applyProposal(messageId),
    failure: 'trials.putFailed',
    refresh,
  })
  const dismiss = useAppMutation({
    mutationFn: (messageId: string) => dismissProposal(messageId),
    failure: 'assistant.dismissFailed',
    refresh,
  })
  if (waiting.length === 0) return null
  return (
    <ul className="flex shrink-0 flex-col gap-1.5">
      {waiting.map((one) => (
        <li
          key={one.message_id}
          className="flex flex-wrap items-center gap-2 rounded-lg bg-info-soft px-3 py-1.5 text-xs text-info"
        >
          <b className="font-semibold">
            {t('trials.agent', { client: one.chat_title ?? t('trials.anAgent') })}
          </b>
          <span className="ml-auto flex gap-1.5">
            <Button
              size="xs"
              variant="primary"
              disabled={put.isPending}
              onClick={() => put.mutate(one.message_id)}
            >
              {t('trials.putProposed')}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              disabled={dismiss.isPending}
              onClick={() => dismiss.mutate(one.message_id)}
            >
              {t('assistant.dismiss')}
            </Button>
          </span>
        </li>
      ))}
    </ul>
  )
}
