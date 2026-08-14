import { type FormEvent, type ReactNode, useState } from 'react'
import * as Primitive from '@radix-ui/react-dialog'
import { useTranslation } from 'react-i18next'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { cn } from '@/lib/utils'

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

/**
 * The app's modal. Radix gives us the focus trap, the Esc handler and the
 * scroll lock — the things a hand-rolled overlay always forgets.
 */
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
    <Primitive.Root open={open} onOpenChange={onOpenChange}>
      <Primitive.Portal>
        <Primitive.Overlay className="fixed inset-0 z-50 bg-black/50 backdrop-blur-[2px]" />
        <Primitive.Content
          className={cn(
            'fixed left-1/2 top-1/2 z-50 w-[min(28rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2',
            'rounded-2xl border border-line bg-raise p-5 shadow-raise',
            className,
          )}
        >
          <div className="flex items-start gap-3">
            <div className="flex-1">
              <Primitive.Title className="font-semibold">{title}</Primitive.Title>
              {description !== undefined && (
                <Primitive.Description className="mt-1 text-sm text-dim">
                  {description}
                </Primitive.Description>
              )}
            </div>

            <Primitive.Close asChild>
              <Button variant="icon" size="iconSm" aria-label={t('dialog.close')}>
                <X aria-hidden />
              </Button>
            </Primitive.Close>
          </div>

          {children !== undefined && <div className="mt-4">{children}</div>}

          <div className="mt-5 flex justify-end gap-2">
            <Primitive.Close asChild>
              <Button>{t('dialog.cancel')}</Button>
            </Primitive.Close>
            {footer}
          </div>
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
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
  /** Whether an empty value may be submitted — a release URL is optional. */
  allowEmpty?: boolean
  onSubmit: (value: string) => void
}

/**
 * Asks for one line of text — the replacement for `window.prompt()`.
 *
 * `window.prompt` was blocking, unstyled, untranslatable and in a webview it is
 * not guaranteed to exist at all.
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
