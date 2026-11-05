import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { setParallelRuns, setRunModel, setRunSees } from '@/lib/api/assistant'
import type { AssistantSettings, RunSees } from '@/lib/api/types'
import { keys } from '@/lib/query/keys'
import { queries } from '@/lib/query/queries'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { say } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Field, FieldGroup } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Segment, SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { StatusDot } from '@/components/ui/status-dot'
import { BudgetForm } from '@/features/assistant/BudgetForm'

/** One number for each of the runs a person can ask for; the backend and the
 *  schema hold the same bound. */
const RUNS = [1, 2, 3, 4, 5, 6, 7, 8] as const

const SIGHTS: readonly RunSees[] = ['nothing', 'work']

/**
 * The assistant on this workspace (v0.108): whether Claude Code is there, which
 * model a run starts with, how many go at once, what they may read, and what a
 * week of them may cost.
 *
 * The constant that said three runs since v0.27 and the empty directory of ADR
 * 0008 both promised a place here. Each control saves as it is answered, as the
 * rest of the screen does: there is nothing here to draft.
 */
export function AssistantSection() {
  const settings = useQuery(queries.assistantSettings())

  return (
    <div className="flex max-w-105 flex-col gap-4">
      <CliStatus />
      {settings.data === undefined ? (
        <Skeleton className="h-40 w-full" />
      ) : (
        <>
          {/* Keyed by what is stored, so a box a refused name was typed into
              goes back to what stands. */}
          <ModelField key={settings.data.model ?? ''} model={settings.data.model} />
          <RunsField settings={settings.data} />
          <SightField settings={settings.data} />
        </>
      )}
      <WeekField />
    </div>
  )
}

/** Whether the CLI answers, and asking again without reopening anything: this
 *  is where someone comes straight after installing it. */
function CliStatus() {
  const { t } = useTranslation()
  const status = useQuery(queries.assistantStatus())
  const answer = status.data

  return (
    <FieldGroup label={t('settings.assistant.cli')} help={t('settings.assistant.cliHint')}>
      {answer === undefined ? (
        <Skeleton className="h-3 w-48" />
      ) : answer.available ? (
        <StatusDot
          status="good"
          label={answer.version ?? t('settings.assistant.cliThere')}
          showLabel
        />
      ) : (
        <div className="flex flex-col gap-1">
          <StatusDot status="bad" label={t('settings.assistant.cliMissing')} showLabel />
          {answer.reason !== null && (
            <p className="text-xs break-words text-dim">{answer.reason}</p>
          )}
        </div>
      )}
      <Button
        className="self-start"
        disabled={status.isFetching}
        onClick={() => {
          void status.refetch()
        }}
      >
        {t('settings.assistant.cliCheck')}
      </Button>
    </FieldGroup>
  )
}

/**
 * The model, as a name typed the way the CLI takes it.
 *
 * A box rather than a list: the CLI knows which models exist and its list moves
 * under kilna, while a name it does not know is answered in the chat in its own
 * words. Saved on leaving, or on Enter.
 */
function ModelField({ model }: { model: string | null }) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(model ?? '')

  const save = useAppMutation({
    mutationFn: (name: string | null) => setRunModel(name),
    failure: 'settings.assistant.saveFailed',
    refresh: [keys.assistantSettings],
    onSuccess: (now: AssistantSettings) => {
      say.ok(
        now.model === null
          ? t('settings.assistant.modelCleared')
          : t('settings.assistant.modelSaved', { model: now.model }),
      )
    },
    onError: (cause) => {
      setDraft(model ?? '')
      say.failedTo(t('settings.assistant.saveFailed'), cause)
    },
  })

  const settle = () => {
    const name = draft.trim()
    if (name !== (model ?? '')) save.mutate(name === '' ? null : name)
  }

  return (
    <Field label={t('settings.assistant.model')} help={t('settings.assistant.modelHint')}>
      <Input
        value={draft}
        spellCheck={false}
        placeholder={t('settings.assistant.modelPlaceholder')}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={settle}
        onKeyDown={(event) => {
          if (event.key === 'Enter') settle()
          if (event.key === 'Escape') setDraft(model ?? '')
        }}
      />
    </Field>
  )
}

/** How many runs go at once: eight worth seeing together, one press each, and a
 *  number out of bounds is not something this control can say. */
function RunsField({ settings }: { settings: AssistantSettings }) {
  const { t } = useTranslation()

  const save = useAppMutation({
    mutationFn: setParallelRuns,
    failure: 'settings.assistant.saveFailed',
    // The queue may have just started what was waiting.
    refresh: [keys.assistantSettings, keys.taskQueue, keys.activeTasks, keys.activeRuns],
    onSuccess: (now) => {
      say.ok(t('settings.assistant.runsSaved', { count: now.parallel_runs }))
    },
  })

  return (
    <FieldGroup label={t('settings.assistant.runs')} help={t('settings.assistant.runsHint')}>
      <SegmentedControl
        aria-label={t('settings.assistant.runs')}
        value={String(settings.parallel_runs)}
        onValueChange={(next) => save.mutate(Number(next))}
      >
        {RUNS.map((runs) => (
          <Segment key={runs} value={String(runs)}>
            {runs}
          </Segment>
        ))}
      </SegmentedControl>
    </FieldGroup>
  )
}

/** What a run may read: the two answers side by side, and under them what the
 *  one in force means in full. */
function SightField({ settings }: { settings: AssistantSettings }) {
  const { t } = useTranslation()

  const save = useAppMutation({
    mutationFn: setRunSees,
    failure: 'settings.assistant.saveFailed',
    refresh: [keys.assistantSettings, keys.runSightAll],
  })

  return (
    <FieldGroup
      label={t('settings.assistant.sees')}
      help={t(`settings.assistant.seesHint.${settings.run_sees}`)}
    >
      <SegmentedControl
        aria-label={t('settings.assistant.sees')}
        value={settings.run_sees}
        onValueChange={(next) => save.mutate(next as RunSees)}
      >
        {SIGHTS.map((sees) => (
          <Segment key={sees} value={sees}>
            {t(`settings.assistant.seesName.${sees}`)}
          </Segment>
        ))}
      </SegmentedControl>
    </FieldGroup>
  )
}

/** The week's budget, the same form the assistant's head opens. */
function WeekField() {
  const { t } = useTranslation()
  const spend = useQuery(queries.spend()).data

  return (
    <FieldGroup label={t('assistant.budget')}>
      {spend === undefined ? (
        <Skeleton className="h-16 w-full" />
      ) : (
        <BudgetForm key={spend.budget ?? 'none'} budget={spend.budget} />
      )}
    </FieldGroup>
  )
}
