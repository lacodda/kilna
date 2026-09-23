import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { listen } from '@tauri-apps/api/event'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, Eye, Sparkles } from 'lucide-react'
import { actionsOfScope } from '@/lib/actions'
import { activeTasks, startTask, type PromptTemplate, type RunEmission } from '@/lib/api'
import { keys } from '@/lib/query'
import { say } from '@/lib/toast'
import { movesTaskList, taskKey } from '@/lib/tasks'
import { say as sayLabel, useProfile, useWorkKind } from '@/lib/useProfile'
import { actionIconOf } from '@/lib/actionIcon'
import { Button } from '@/components/ui/button'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { TaskPreviewDialog } from '@/components/assistant/TaskPreviewDialog'
import { StylePickerDialog } from '@/components/styles/StylePickerDialog'

interface Props {
  workId: string
  /** The version the actions are about, when started from the versions tab:
      the template reads it and a score or a commentary binds to it. Without
      it the action is about the work as it stands. */
  versionId?: string
  /** The scene the actions are about: only scene actions are offered, and
      each starts on this row of the board. Without it, work actions. */
  sceneId?: string
  /** One prompt block of that scene: the action is aimed at this block and
      the answer is held to it. Two blocks of a scene run side by side; the
      same block twice does not. */
  block?: string
  /** A line over the buttons saying what they act on. */
  hint?: string
  /** Buttons only, no heading: for a row of the board. */
  compact?: boolean
  /** One button that opens the list, rather than a row of them. Where the
      actions sit beside other controls — the card's header, a version — a row
      of five is five buttons competing with everything around them for the
      eye, and the owner asked for one that opens. */
  menu?: boolean
}

/** Whether the action's message reads the style dictionary, and so cannot be
    started until someone has said which parts to build from. */
function readsStyles(action: PromptTemplate): boolean {
  return action.template.includes('{styles}')
}

/** The actions of the profile that belong here: for this kind, at this scope. */
export function actionsFor(
  actions: PromptTemplate[],
  kind: string | undefined,
  scope: 'work' | 'scene',
): PromptTemplate[] {
  if (kind === undefined) return []
  // By scope first: an action about a style brick or a comment belongs to
  // neither bar. Asked through `actionsOfScope`, not a `scene ? scene : work`
  // fallback here — that fallback offered the style action on every card.
  return actionsOfScope(actions, scope).filter(
    (action) =>
      action.kinds === undefined || action.kinds.length === 0 || action.kinds.includes(kind),
  )
}

/**
 * The profile's actions, started from the work rather than from the panel.
 *
 * The same actions live in the assistant tab, where clicking one fills the
 * composer so the prompt can be read before it is paid for. This bar is the
 * other way of using them: hands on the work, wanting the thing done, not
 * wanting to move. A click starts a run and says where it went — nothing to
 * watch, nothing to wait for. The eye beside a button shows exactly what the
 * click would send — the message and the method — and starts it from there,
 * with reference files if the method should look at some.
 *
 * Only the actions for the work's kind are offered: a critique of lyrics on
 * a video would send a prompt with a hole in it. Scene actions are offered
 * on a row of the board, not here, and the other way round.
 *
 * The answer lands in a chat of its own, never in whatever conversation
 * happened to be open. Two reasons: a task dropped into a live thread inherits
 * that thread's session as context, and it buries the answer in someone else's
 * subject.
 */
export function ActionBar({
  workId,
  versionId,
  sceneId,
  block,
  hint,
  compact = false,
  menu = false,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const kind = useWorkKind(workId)
  const client = useQueryClient()
  const [previewing, setPreviewing] = useState<PromptTemplate | null>(null)
  /** The action waiting on a pick of styles, when one reads `{styles}`. */
  const [picking, setPicking] = useState<PromptTemplate | null>(null)

  // Which actions are already going. Asked of the backend rather than kept
  // here: a run started before this card was opened still owns its button, and
  // a component's own state cannot know that.
  const running = useQuery({
    queryKey: keys.activeTasks,
    queryFn: activeTasks,
    staleTime: 0,
  })

  // A task ends without anyone watching this bar, and its button has to come
  // back by itself. Only run boundaries move the list.
  useEffect(() => {
    const subscription = listen<RunEmission>('assistant:run', ({ payload }) => {
      if (movesTaskList(payload)) {
        void client.invalidateQueries({ queryKey: keys.activeTasks })
      }
    })
    return () => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    }
  }, [client])

  const start = useMutation({
    mutationFn: ({ action, styleBrickIds }: { action: string; styleBrickIds?: string[] }) =>
      startTask(workId, action, { versionId, sceneId, block, styleBrickIds }),
    onSuccess: (started) => {
      void client.invalidateQueries({ queryKey: keys.activeTasks })
      void client.invalidateQueries({ queryKey: keys.allChats })
      say.info(t('assistant.taskStarted', { title: started.title }))
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  const actions = actionsFor(profile.config.prompts, kind, sceneId === undefined ? 'work' : 'scene')
  if (actions.length === 0) return null

  const busy = new Set(running.data ?? [])
  // The action whose start has not come back yet. Held only for that moment:
  // once the backend answers, the list of running tasks is what the buttons
  // read, and this goes back to null whether the start succeeded or failed.
  const pending = start.isPending ? start.variables.action : null

  /** The action's own words, or nothing when the profile gave it none. */
  const describe = (action: PromptTemplate) => sayLabel(action.description)

  const buttons = (
    <div className="flex flex-wrap gap-1.5">
      {actions.map((action) => {
        // The key the backend refuses duplicates by, built the same way on
        // both sides. Only this button's own task disables it: three runs
        // may go at once, and greying out the whole row because one action
        // was clicked would say otherwise.
        //
        // The list is the single source of that answer — a click that is
        // still in flight is covered by `pending` rather than by a
        // second piece of state that would have to be cleared by hand.
        const working = busy.has(taskKey(action.key, workId, sceneId, block)) || pending === action.key
        const Icon = actionIconOf(action)

        return (
          <span key={action.key} className="inline-flex items-stretch">
            {/* On a row of a table the label does not fit: the board gives
                an action about as much room as an icon, and a button drawn
                at its full width lands on top of the cell beside it — as it
                did on a fifty-scene board, measured. Compact keeps the name
                in the tooltip and for a screen reader, and shows the mark. */}
            <Button
              size="sm"
              className="rounded-r-none"
              // The long description is what the tooltip is for, at every
              // width. A row of five actions spelled out in full is five
              // sentences where the eye wants five marks - so the face of the
              // button is a glyph and a short name, and what the action
              // actually does is one hover away.
              title={
                describe(action) === ''
                  ? sayLabel(action.label)
                  : `${sayLabel(action.label)} — ${describe(action)}`
              }
              aria-label={compact ? sayLabel(action.label) : undefined}
              disabled={working}
              onClick={() => {
                // An action whose message reads the dictionary cannot start
                // without one: the parts are the question being asked, and a
                // prompt built from none would be a prompt with a hole in it.
                if (readsStyles(action)) {
                  setPicking(action)
                  return
                }
                start.mutate({ action: action.key })
              }}
            >
              <Icon aria-hidden className="size-3.5" />
              {!compact &&
                (working ? t('assistant.actionWorking', { label: sayLabel(action.label) }) : sayLabel(action.label))}
            </Button>
            <Button
              size="sm"
              className="rounded-l-none border-l-0 px-1.5"
              title={t('assistant.previewTask', { label: sayLabel(action.label) })}
              aria-label={t('assistant.previewTask', { label: sayLabel(action.label) })}
              disabled={working}
              onClick={() => {
                setPreviewing(action)
              }}
            >
              <Eye aria-hidden className="size-3.5" />
            </Button>
          </span>
        )
      })}
    </div>
  )

  // The same actions, behind one button. Each row carries its glyph and its
  // short name, and the long description is the title the way it is on a
  // button — one place decides what an action is called and what it says.
  const menuOfActions = (
    <Menu>
      <MenuTrigger
        render={
          <Button
            variant="soft"
            size="sm"
            aria-label={t('assistant.actions')}
            // What the actions would act on — "on Revision 10". The full bar
            // says this in a line under its heading; with only a button there
            // is nowhere to put a line, and losing it would leave a menu that
            // does not say which version it is about.
            title={hint}
          />
        }
      >
        <Sparkles aria-hidden className="size-3.5" />
        {t('assistant.actions')}
        <ChevronDown aria-hidden className="size-3.5 opacity-60" />
      </MenuTrigger>

      <MenuPopup align="end">
        {actions.map((action) => {
          const Icon = actionIconOf(action)
          const working = busy.has(taskKey(action.key, workId, sceneId, block)) || pending === action.key
          return (
            <MenuItem
              key={action.key}
              disabled={working}
              title={
                describe(action) === ''
                  ? sayLabel(action.label)
                  : `${sayLabel(action.label)} — ${describe(action)}`
              }
              onClick={() => {
                if (readsStyles(action)) {
                  setPicking(action)
                  return
                }
                start.mutate({ action: action.key })
              }}
            >
              <Icon aria-hidden className="size-3.5" />
              {working
                ? t('assistant.actionWorking', { label: sayLabel(action.label) })
                : sayLabel(action.label)}
            </MenuItem>
          )
        })}
      </MenuPopup>
    </Menu>
  )

  return (
    <>
      {menu ? (
        menuOfActions
      ) : compact ? (
        buttons
      ) : (
        <section className="flex flex-col gap-2">
          <h3 className="text-sm font-semibold">{t('assistant.actions')}</h3>
          {hint !== undefined && <p className="text-xs text-dim">{hint}</p>}
          {buttons}
        </section>
      )}
      {picking !== null && (
        <StylePickerDialog
          open
          onOpenChange={(open) => {
            if (!open) setPicking(null)
          }}
          actionLabel={sayLabel(picking.label)}
          onStart={(styleBrickIds) => {
            start.mutate({ action: picking.key, styleBrickIds })
            setPicking(null)
          }}
        />
      )}
      {previewing !== null && (
        <TaskPreviewDialog
          open
          onOpenChange={(open) => {
            if (!open) setPreviewing(null)
          }}
          workId={workId}
          action={previewing}
          versionId={versionId}
          sceneId={sceneId}
          block={block}
          onStarted={() => {
            setPreviewing(null)
            void client.invalidateQueries({ queryKey: keys.activeTasks })
            void client.invalidateQueries({ queryKey: keys.allChats })
          }}
        />
      )}
    </>
  )
}
