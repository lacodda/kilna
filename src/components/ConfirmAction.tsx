import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  ConfirmDialog,
  ConfirmDialogActions,
  ConfirmDialogClose,
  ConfirmDialogDescription,
  ConfirmDialogHeader,
  ConfirmDialogPopup,
  ConfirmDialogTitle,
} from '@/components/ui/confirm-dialog'

/*
 * kilna's own shape of a question that cannot be taken back.
 *
 * The app deletes without asking: everything lands in the trash and the toast
 * offers the way back. What is left to ask about is what has no way back -
 * deleting from the trash for good, emptying it - and those asked through the
 * ordinary Dialog, which a stray click beside it dismisses, with the accent
 * colour on the button that destroys. dowel's ConfirmDialog is the right
 * clothes (`alertdialog`, no dismissal by pointer); this file is the one
 * sentence every such question in the app is made of, so the next one does not
 * reach for the ordinary Dialog again.
 */

interface ConfirmActionProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** The question: "Delete “Winter road” for good?" */
  title: string
  /** What the answer costs, in one sentence. */
  description: string
  /** The verb on the button: "Delete for good". */
  actionLabel: string
  onConfirm: () => void
  /** Held while the action runs, so a second press cannot send it twice. */
  pending?: boolean
}

export function ConfirmAction({
  open,
  onOpenChange,
  title,
  description,
  actionLabel,
  onConfirm,
  pending = false,
}: ConfirmActionProps) {
  const { t } = useTranslation()

  return (
    <ConfirmDialog open={open} onOpenChange={onOpenChange}>
      <ConfirmDialogPopup>
        <ConfirmDialogHeader>
          <ConfirmDialogTitle>{title}</ConfirmDialogTitle>
          <ConfirmDialogDescription>{description}</ConfirmDialogDescription>
        </ConfirmDialogHeader>
        <ConfirmDialogActions>
          <ConfirmDialogClose render={<Button />}>{t('dialog.cancel')}</ConfirmDialogClose>
          <Button variant="danger" disabled={pending} onClick={onConfirm}>
            {actionLabel}
          </Button>
        </ConfirmDialogActions>
      </ConfirmDialogPopup>
    </ConfirmDialog>
  )
}
