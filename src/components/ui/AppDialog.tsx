import { type FormEvent, type ReactNode, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog as Base,
  DialogActions,
  DialogBody,
  DialogClose,
  DialogDescription,
  DialogHeader,
  DialogPopup,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { firstFocus, useTypedSinceOpen } from '@/lib/dialogGuard'

/*
 * Casing here is the convention, not a style: a lowercase file in
 * `components/ui/` is a copy from the registry and is never edited, so it can
 * stay byte-identical to upstream; a PascalCase one is this app's own, and
 * this is one of those.
 */

/*
 * kilna's own shape of a dialog.
 *
 * dowel exposes the parts - header, body, actions - because a design system
 * cannot know what a product wants inside one. This app does know: every
 * dialog it opens is a heading, a sentence, some content, and a Cancel beside
 * one affirmative button. So the shape lives here, over dowel's parts, rather
 * than being spelled out at each call site.
 *
 * The Cancel and the close cross are the two words this file owns. They are
 * translated here rather than passed in, because they are the same words every
 * time and asking each caller for them would guarantee they eventually differ.
 *
 * Four behaviours are the reason this file exists rather than the parts at
 * each call site, and each was a bug before it was a rule:
 *
 * - **Only the body scrolls.** The header and the actions stay whole; the
 *   style editor at a 1280x720 window used to put Save 137px below the edge.
 * - **The width is a size, not a class.** `className="max-w-2xl"` on a popup
 *   whose width is `w-[min(28rem,…)]` changed nothing, and three dialogs that
 *   meant to be wide came out 448px with their chips in three rows.
 * - **Focus lands on the first field**, not on the close cross that happens to
 *   be the first thing in the popup. A dialog with no field focuses Cancel -
 *   the answer that costs nothing if Enter is pressed by reflex.
 * - **Typing makes the dialog stay.** A click beside a dialog that has been
 *   typed into does not close it; `Escape` and Cancel still do, because those
 *   are asked for. What is typed is noticed by the `input` event, which every
 *   text field, textarea and checkbox fires, so no caller has to remember to
 *   report it; a form whose state lives in custom controls says so through
 *   `dirty`.
 */

/** The widths a dialog comes in: `md` for a question and a field or two, `lg`
 * for a form, `xl` for an editor whose choices should fit on one line. */
export type DialogSize = 'sm' | 'md' | 'lg' | 'xl' | 'full'

interface DialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: string
  children?: ReactNode
  /** The action row. Cancel is provided; this is the affirmative side. */
  footer?: ReactNode
  /**
   * An action that is not an answer to the dialog - deleting the thing it
   * edits. Drawn at the start of the row, away from Cancel and the affirmative
   * button: between them, one slip of the pointer on the way to Save was a
   * deletion (the style dialog, 24.09).
   */
  aside?: ReactNode
  /** How wide. `md` unless the content says otherwise. */
  size?: DialogSize
  /** The form has changes that live outside a text field - a chip picked, a
   * file attached - so a stray click must not close it. Typing is noticed
   * without this. */
  dirty?: boolean
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  aside,
  size = 'md',
  dirty = false,
}: DialogProps) {
  const { t } = useTranslation()
  const body = useRef<HTMLDivElement>(null)
  const cancel = useRef<HTMLButtonElement>(null)
  const { typed, onInput } = useTypedSinceOpen(open)

  return (
    <Base open={open} onOpenChange={onOpenChange} disablePointerDismissal={dirty || typed}>
      <DialogPopup
        size={size}
        initialFocus={() => firstFocus(body.current) ?? cancel.current ?? true}
        onInput={onInput}
      >
        <DialogHeader
          action={
            <DialogClose
              render={<Button variant="icon" size="icon-sm" aria-label={t('dialog.close')} />}
            >
              <X aria-hidden />
            </DialogClose>
          }
        >
          <DialogTitle>{title}</DialogTitle>
          {description !== undefined && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>

        {/* The ref sits on a box that draws nothing: the body is dowel's part
            and takes no ref, and what is looked for is only inside it. */}
        {children !== undefined && (
          <DialogBody>
            <div ref={body} className="contents">
              {children}
            </div>
          </DialogBody>
        )}

        <DialogActions start={aside}>
          <DialogClose ref={cancel} render={<Button />}>
            {t('dialog.cancel')}
          </DialogClose>
          {footer}
        </DialogActions>
      </DialogPopup>
    </Base>
  )
}

interface PromptDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: string
  label: string
  placeholder?: string
  initialValue?: string
  confirmLabel: string
  /** Lets an empty answer through. Off by default: a prompt usually wants
   * something, and the affirmative button staying inert says so. */
  allowEmpty?: boolean
  onSubmit: (value: string) => void
}

/**
 * A dialog that asks for one line of text.
 *
 * Separate from `Dialog` because the state, the reset on reopen and the submit
 * are the same every time - and because a caller that has to remember to clear
 * the field on close will, eventually, not.
 */
export function PromptDialog({
  open,
  onOpenChange,
  title,
  description,
  label,
  placeholder,
  initialValue = '',
  confirmLabel,
  allowEmpty = false,
  onSubmit,
}: PromptDialogProps) {
  const [value, setValue] = useState(initialValue)
  const [openedWith, setOpenedWith] = useState<string | null>(null)

  // Each opening starts from the given value rather than whatever was typed and
  // abandoned last time. Tracked by a render-phase reset so the first paint of
  // the dialog already shows the right value.
  if (open && openedWith === null) {
    setOpenedWith(initialValue)
    setValue(initialValue)
  } else if (!open && openedWith !== null) {
    setOpenedWith(null)
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!allowEmpty && value.trim() === '') return
    onSubmit(value.trim())
    onOpenChange(false)
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={title}
      description={description}
      footer={
        <Button type="submit" form="prompt-dialog" variant="primary">
          {confirmLabel}
        </Button>
      }
    >
      <form id="prompt-dialog" onSubmit={submit}>
        <Input
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder={placeholder}
          aria-label={label}
        />
      </form>
    </Dialog>
  )
}
