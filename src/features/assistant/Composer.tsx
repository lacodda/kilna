import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation } from '@tanstack/react-query'
import { CornerDownLeft, Slash, Square } from 'lucide-react'
import { actionIconOf } from '@/lib/actionIcon'
import { actionsOfScope } from '@/lib/actions'
import { renderPrompt } from '@/lib/api/assistant'
import type { PromptTemplate } from '@/lib/api/types'
import { reading } from '@/lib/palette'
import { say } from '@/lib/toast'
import { say as sayLabel, useProfile } from '@/lib/useProfile'
import { Button } from '@/components/ui/button'
import { RowButton } from '@/components/ui/list-row'
import { Menu, MenuItem, MenuPopup, MenuTrigger } from '@/components/ui/menu'
import { Textarea } from '@/components/ui/textarea'

interface Props {
  /** The work the chat is about; its actions are offered. Absent for a chat
   * about nothing, which has no work to fill a template with. */
  workId?: string
  draft: string
  onDraft: (next: string) => void
  /** Send the draft. Not called for an empty one. */
  onSend: (prompt: string) => void
  /** A question on its way: another one waits. */
  sending: boolean
  /** A run of this chat is going: the stop stands beside the send. */
  working: boolean
  onStop: () => void
  stopping: boolean
}

/**
 * The foot of a conversation: the box a question is typed into.
 *
 * Enter sends and Shift+Enter breaks the line; `/` at the start opens the
 * profile's actions, and the slash button beside the box is the same list for
 * a hand on the mouse. Either way an action fills the box with its rendered
 * template rather than firing: what is about to be sent - and paid for - is
 * read and edited first, and Enter still does the sending. The actions were
 * a row of buttons over the conversation until v0.81, a wall to read past on
 * the way to every answer.
 */
export function Composer({
  workId,
  draft,
  onDraft,
  onSend,
  sending,
  working,
  onStop,
  stopping,
}: Props) {
  const { t } = useTranslation()
  const profile = useProfile()
  const box = useRef<HTMLTextAreaElement>(null)
  // Which entry of the slash palette the arrow keys are on. Reset whenever the
  // query changes, so the highlight never points past a shortened list.
  const [highlighted, setHighlighted] = useState(0)

  const runTemplate = useMutation({
    // Not a write: it only fills the composer, so nothing is refreshed.
    mutationFn: (template: string) => renderPrompt(workId!, template),
    onSuccess: (prompt) => {
      onDraft(prompt)
      box.current?.focus()
      box.current?.setSelectionRange(0, 0)
      box.current?.scrollTo({ top: 0 })
    },
    onError: (cause) => {
      say.failed(cause)
    },
  })

  // A work chat offers the work's actions: a scene's, a style's or a
  // comment's template filled into it would ask about something it is not.
  const actions = workId === undefined ? [] : actionsOfScope(profile.config.prompts, 'work')
  // The palette is open when the draft is nothing but a slash command. Derived
  // rather than kept in state: the draft is the only truth, and a second copy
  // would be one more thing to get out of step with it.
  const palette = actions.length === 0 ? null : reading(draft, actions)
  const chosen = palette?.matches[Math.min(highlighted, palette.matches.length - 1)] ?? null
  const busy = sending || runTemplate.isPending

  // Choosing from the palette does what choosing from the menu does: render
  // the template into the composer, to be read before it is sent.
  const pick = (action: PromptTemplate) => {
    setHighlighted(0)
    runTemplate.mutate(action.template)
  }

  const send = () => {
    if (draft.trim() === '' || busy) return
    onSend(draft)
  }

  const label = t(workId === undefined ? 'assistant.placeholderAnywhere' : 'assistant.placeholder')

  return (
    <form
      className="relative flex w-full items-end gap-1.5"
      onSubmit={(event) => {
        event.preventDefault()
        if (palette !== null) {
          if (chosen !== null) pick(chosen)
          return
        }
        send()
      }}
    >
      {palette !== null && (
        <ul
          role="listbox"
          aria-label={t('assistant.paletteLabel')}
          className="absolute bottom-full z-10 mb-1 max-h-64 w-full overflow-y-auto rounded-lg border border-line bg-raise p-1 shadow-raise"
        >
          {palette.matches.length === 0 && (
            <li className="px-2.5 py-2 text-sm text-dim">{t('assistant.paletteEmpty')}</li>
          )}
          {palette.matches.map((action, index) => (
            <li key={action.key}>
              <RowButton
                role="option"
                selected={action.key === chosen?.key}
                aria-selected={action.key === chosen?.key}
                // In a listbox the highlighted entry is the selected option;
                // `aria-current` on top of it would say the same thing twice.
                aria-current={undefined}
                // Pointer down, not click: the composer keeps focus, so the
                // draft the choice replaces is still the one on screen.
                onMouseDown={(event) => {
                  event.preventDefault()
                  pick(action)
                }}
                onMouseEnter={() => {
                  setHighlighted(index)
                }}
                description={
                  action.description === undefined ? undefined : sayLabel(action.description)
                }
              >
                {sayLabel(action.label)}
              </RowButton>
            </li>
          ))}
        </ul>
      )}

      {actions.length > 0 && (
        <Menu>
          <MenuTrigger
            render={
              <Button
                variant="icon"
                size="icon-sm"
                disabled={busy}
                aria-label={t('assistant.paletteLabel')}
                title={t('assistant.paletteHint')}
              />
            }
          >
            <Slash aria-hidden />
          </MenuTrigger>
          <MenuPopup side="top" align="start">
            {actions.map((action) => {
              const Icon = actionIconOf(action)
              return (
                <MenuItem
                  key={action.key}
                  title={sayLabel(action.description)}
                  onClick={() => {
                    pick(action)
                  }}
                >
                  <Icon aria-hidden />
                  {sayLabel(action.label)}
                </MenuItem>
              )
            })}
          </MenuPopup>
        </Menu>
      )}

      <Textarea
        ref={box}
        // One line, as the mockup's box; a rendered template can be pages
        // long, and the box follows it up to a point instead of showing one
        // line of something worth reading.
        autoResize
        maxRows={10}
        rows={1}
        className="min-w-0 flex-1 bg-bg"
        value={draft}
        onChange={(event) => {
          onDraft(event.target.value)
        }}
        placeholder={label}
        aria-label={label}
        onKeyDown={(event) => {
          // While the palette is open the arrows and Enter belong to it.
          if (palette !== null) {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault()
              const count = palette.matches.length
              if (count > 0) {
                const step = event.key === 'ArrowDown' ? 1 : count - 1
                setHighlighted((current) => (Math.min(current, count - 1) + step) % count)
              }
              return
            }
            if (event.key === 'Escape') {
              event.preventDefault()
              onDraft('')
              return
            }
            if (event.key === 'Enter' && !event.shiftKey) {
              event.preventDefault()
              if (chosen !== null) pick(chosen)
              return
            }
          }

          // Enter sends; the panel is for questions, not for composing.
          if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault()
            send()
          }
        }}
      />

      {/* Beside the send rather than in its place: a second question may be
          typed while the first is still being answered. */}
      {working && (
        <Button variant="ghost" size="sm" disabled={stopping} onClick={onStop}>
          <Square aria-hidden />
          {t('assistant.cancel')}
        </Button>
      )}
      <Button
        type="submit"
        variant="primary"
        size="icon-sm"
        disabled={busy || draft.trim() === ''}
        aria-label={t('assistant.send')}
        title={t('assistant.sendHint')}
      >
        <CornerDownLeft aria-hidden />
      </Button>
    </form>
  )
}
