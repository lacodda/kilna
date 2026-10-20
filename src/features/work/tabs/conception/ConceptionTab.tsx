import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { ClipboardCopy, PenLine, Wand2 } from 'lucide-react'
import { startTask } from '@/lib/api/assistant'
import { conceptionOrder } from '@/lib/api/boards'
import type { PromptTemplate, Work } from '@/lib/api/types'
import {
  boardsOf,
  conceptionOf,
  ideaFieldOf,
  refineActionOf,
  writeActionOf,
} from '@/lib/conception'
import { keys } from '@/lib/query/keys'
import { useAppMutation } from '@/lib/query/useAppMutation'
import { taskKey } from '@/lib/tasks'
import { say } from '@/lib/toast'
import { useRunningTasks } from '@/lib/useRunningTasks'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { Frame, Scroll } from '@/components/frame'
import { MetaParagraph } from '@/features/work/tabs/overview/MetaInput'
import { useWorkEdit } from '@/features/work/tabs/overview/useWorkEdit'
import { AttachedNotes, NotePanel } from '@/features/work/tabs/notes/NotePanel'
import { BoardSection } from '@/features/work/tabs/conception/BoardSection'
import { BriefPanel } from '@/features/work/tabs/conception/BriefPanel'

interface Props {
  work: Work
}

/**
 * What a work is prepared from before it is written (v0.99, ADR 0065): the
 * brief in the author's words, the idea it is refined into, the boards of
 * candidates - scenarios to write it by - and the notes and phrases attached.
 *
 * Its head holds what the preparation is for: the order a writer works from,
 * copied out as markdown or handed to the assistant to write from. A work of
 * a kind prepared on no conception has the notes attached to it alone, the
 * way its Notes tab had them.
 */
export function ConceptionTab({ work }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const conception = conceptionOf(config, work.kind)
  if (conception === undefined) return <NotePanel workId={work.id} />

  const boards = boardsOf(config, work.kind)
  // The brief takes the width the idea leaves: an experiment has no idea to
  // refine its brief into, and an empty column beside it was a hole.
  const withIdea =
    ideaFieldOf(config, work.kind) !== undefined || refineActionOf(config, work.kind) !== undefined
  return (
    <Frame head={<ConceptionHead work={work} />}>
      <Scroll label={t('conception.title')}>
        <div className="flex flex-col gap-2.5 pr-0.75 pb-2.5">
          <div
            className={
              withIdea
                ? 'grid grid-cols-[minmax(0,3fr)_minmax(0,2fr)] items-start gap-2.5 max-[1100px]:grid-cols-1'
                : 'flex flex-col'
            }
          >
            <BriefPanel work={work} />
            {withIdea && <IdeaPanel work={work} />}
          </div>
          {boards.map((board) => (
            <BoardSection key={board.key} work={work} board={board} />
          ))}
          <AttachedNotes workId={work.id} />
        </div>
      </Scroll>
    </Frame>
  )
}

/** The head of the tab: the two things the preparation is for - the order
 *  copied out, and the text written from it. The tab's name is the bar's: a
 *  caption saying it again stood beside the brief's own. */
function ConceptionHead({ work }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const write = writeActionOf(config, work.kind)

  const copy = useAppMutation({
    mutationFn: () =>
      conceptionOrder(work.id).then((order) => navigator.clipboard.writeText(order)),
    failure: 'conception.copyFailed',
    onSuccess: () => say.ok(t('conception.copied')),
  })

  return (
    <div className="flex w-full flex-wrap items-center justify-end gap-2">
      <span className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          variant="ghost"
          title={t('conception.copyHint')}
          disabled={copy.isPending}
          onClick={() => copy.mutate(undefined)}
        >
          <ClipboardCopy aria-hidden />
          {t('conception.copy')}
        </Button>
        {write !== undefined && <ActionButton work={work} action={write} icon="write" />}
      </span>
    </div>
  )
}

/** The idea the brief is refined into, written in place, and the action that
 *  refines it - which proposes the idea and the brief set straight. */
function IdeaPanel({ work }: Props) {
  const { t } = useTranslation()
  const { config } = useProfile()
  const { setField } = useWorkEdit(work)
  const field = ideaFieldOf(config, work.kind)
  const refine = refineActionOf(config, work.kind)
  if (field === undefined && refine === undefined) return null

  return (
    <Panel className="flex flex-col gap-1.5 px-3 py-2.5">
      <div className="flex items-center gap-2">
        <h3 className="caption">
          {field === undefined ? t('conception.idea') : sayLabel(field.label)}
        </h3>
        {refine !== undefined && (
          <span className="ml-auto">
            <ActionButton work={work} action={refine} icon="refine" />
          </span>
        )}
      </div>
      {field !== undefined && (
        <MetaParagraph
          label={sayLabel(field.label)}
          value={work.meta[field.key] ?? null}
          onCommit={(text) => setField(field.key, text)}
        />
      )}
      <p className="text-xs text-faint">{t('conception.refineHint')}</p>
    </Panel>
  )
}

/**
 * One of the conception's actions on the work, started where it is read: its
 * answer lands in a chat of its own, as every task's does, and the toast says
 * where. The button is held while the task runs - the backend's list of
 * running tasks says so, whoever started it.
 */
function ActionButton({
  work,
  action,
  icon,
}: {
  work: Work
  action: PromptTemplate
  icon: 'refine' | 'write'
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const running = useRunningTasks()
  const going = running.has(taskKey(action.key, work.id))
  const start = useAppMutation({
    mutationFn: () => startTask(work.id, action.key, {}),
    refresh: [keys.activeTasks, keys.allChats],
    onSuccess: (started) =>
      say.withAction(
        t('assistant.taskStarted', { title: started.title }),
        t('conception.openChat'),
        () => void navigate(`/works/${work.id}/assistant`),
      ),
  })
  const Icon = icon === 'refine' ? Wand2 : PenLine
  return (
    <Button
      size="sm"
      variant={icon === 'write' ? 'primary' : 'ghost'}
      title={sayLabel(action.description) || undefined}
      disabled={going || start.isPending}
      onClick={() => start.mutate(undefined)}
    >
      <Icon aria-hidden />
      {going ? t('conception.running') : sayLabel(action.label)}
    </Button>
  )
}
