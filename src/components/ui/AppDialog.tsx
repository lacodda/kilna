import { type FormEvent, type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog as Base,
  DialogActions,
  DialogClose,
  DialogDescription,
  DialogPopup,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

/*
 * Casing here is the convention, not a style: a lowercase file in
 * `components/ui/` is a copy from the registry and is never edited, so it can
 * stay byte-identical to upstream; a PascalCase one is this app's own, and
 * this is one of those.
 */

/*
 * kilna's own shape of a dialog.
 *
 * dowel exposes the parts - popup, title, description, actions - because a
 * design system cannot know what a product wants inside one. This app does
 * know: every dialog it opens is a heading, a sentence, some content, and a
 * Cancel beside one affirmative button. So the shape lives here, over dowel's
 * parts, rather than being spelled out at each of the six call sites.
 *
 * The Cancel and the close cross are the two words this file owns. They are
 * translated here rather than passed in, because they are the same words every
 * time and asking each caller for them would guarantee they eventually differ.
 */

interface DialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: string
  children?: ReactNode
  /** The action row. Cancel is provided; this is the affirmative side. */
  footer?: ReactNode
  className?: string
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  className,
}: DialogProps) {
  const { t } = useTranslation()

  return (
    <Base open={open} onOpenChange={onOpenChange}>
      <DialogPopup className={cn('relative', className)}>
        <DialogTitle>{title}</DialogTitle>
        {description !== undefined && <DialogDescription>{description}</DialogDescription>}

        <DialogClose
          render={
            <Button
              variant="icon"
              size="icon-sm"
              aria-label={t('dialog.close')}
              className="absolute right-3 top-3"
            />
          }
        >
          <X aria-hidden />
        </DialogClose>

        {children !== undefined && <div className="mt-4">{children}</div>}

        <DialogActions>
          <DialogClose render={<Button />}>{t('dialog.cancel')}</DialogClose>
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
          autoFocus
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder={placeholder}
          aria-label={label}
        />
      </form>
    </Dialog>
  )
}
